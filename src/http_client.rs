use crate::http::http_message::{HttpMessage, HttpVersion};
use crate::http::http_request::HttpRequest;
use crate::http::http_response::HttpResponse;
use crate::ipc::request_pipe::RequestPipe;
use crate::ipc::IpcMessage;
use crate::utils::new_named_pipe;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{atomic, Arc, Mutex};

pub struct HttpClient {
    stream: TcpStream,
    address: SocketAddr,
    sender: Arc<Mutex<RequestPipe>>,
	default_headers: HashMap<String, String>
}

static CLIENT_COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

impl HttpClient {
    pub fn new(stream: TcpStream, address: SocketAddr, lua_send: Arc<Mutex<RequestPipe>>, default_headers: HashMap<String, String>) -> Self {
        Self {
            stream,
            address,
            sender: lua_send,
			default_headers
        }
    }

	fn mk_response(&self, code: u16, mut headers: HashMap<String, String>, content: Vec<u8>, version: HttpVersion) -> HttpResponse {
		for (k, v) in &self.default_headers {
			headers.insert(k.to_string(), v.to_string());
		}

		HttpResponse::new(code, headers, content, version)
	}

	fn serve_static_file(&mut self, req: &HttpRequest, path: &Path) -> bool {
		let data = match fs::read(path) {
			Ok(data) => data,
			Err(_) => return false
		};

		let content_type = match path.extension() {
			Some(ext) => {
				match ext.to_string_lossy().to_lowercase().as_str() {
					"html" => "text/html",
					"js" => "text/javascript",
					"css" => "text/css",
					"png" => "image/png",
					"jpg" => "image/jpeg",
					"jpeg" => "image/jpeg",
					"gif" => "image/gif",
					"ico" => "image/x-icon",
					_ => "application/octet-stream"
				}
			},
			None => "application/octet-stream",
		};

		let request = self.mk_response(
			200,
			HashMap::from([("Content-Type".to_string(), content_type.to_string())]),
			data,
			req.version.clone()
		);

		_ = self.stream.write(request.serialize().as_ref());
		true
	}

    fn handle_static_file_request(&mut self, req: &HttpRequest, content_root: &Path) -> bool {
		let uri_no_root: &str = if req.url.uri.starts_with("/") {
			&req.url.uri[1..]
		} else {
			&req.url.uri
		};

		let path = content_root.join(uri_no_root);

		if !path.starts_with(content_root) {
			return false
		}

		if !path.exists() {
			return false
		}

		if path.is_dir() {
			if !path.to_string_lossy().ends_with("/") {
				let mut redirect_location = req.url.uri.to_string();
				redirect_location.push('/');
				let content = "301: Moved Permanently".as_bytes().to_vec();
				let resp = self.mk_response(
					301,
					HashMap::from([
						("Location".to_string(), redirect_location),
						("Content-Type".to_string(), "text/plain".to_string())
					]),
					content,
					req.version.clone()
				);

				let serialized = resp.serialize();
				_ = self.stream.write_all(serialized.as_slice());
				return true;
			}

			let idx_path = path.join("index.html");
			if idx_path.is_file() {
				return self.serve_static_file(req, &idx_path);
			}
		} else {
			return self.serve_static_file(req, &path);
		}

		false
	}

	fn handle_dynamic_request(&mut self, req: &HttpRequest) -> bool {
		let id =
			CLIENT_COUNTER.fetch_add(1, atomic::Ordering::AcqRel);

		let name = format!("{}_{:?}", std::process::id(), id);
		let out_name = format!("{name}.out");
		let in_name = format!("{name}.in");

		let out_fifo_path = match new_named_pipe(&out_name) {
			Ok(p) => p,
			Err(e) => {
				println!("Failed to create named pipe: {:?}", e);
				return false;
			}
		};

		let in_fifo_path = match new_named_pipe(&in_name) {
			Ok(p) => p,
			Err(e) => {
				println!("Failed to create named pipe: {:?}", e);
				return false;
			}
		};

		let mut l = match self.sender.lock() {
			Ok(s) => s,
			Err(_) => {
				println!("Error while locking send mutex");
				return false;
			}
		};

		let msg =
			match l.send_message_and_wait(IpcMessage::Request {
				request_path: name.clone(),
			}) {
				Ok(msg) => msg,
				Err(e) => {
					println!("Failed to send IPC request: {:?}", e);
					return false;
				}
			};

		drop(l);

		match msg {
			IpcMessage::Ok => {
				let mut fifo = match File::options()
					.read(false)
					.write(true)
					.open(&out_fifo_path)
				{
					Ok(f) => f,
					Err(e) => {
						println!("Failed to open fifo: {:?}", e);
						return false;
					}
				};

				let data = req.serialize();

				let size: u64 = data.len() as u64;
				let size_buff = size.to_ne_bytes();
				if let Err(e) = fifo.write_all(&size_buff) {
					println!(
						"Failed to write request size to fifo: {:?}",
						e
					);
					return false;
				}

				if let Err(e) = fifo.write_all(&data) {
					println!("Failed to write to fifo: {:?}", e);
					return false;
				}

				let mut fifo = match File::options()
					.read(true)
					.write(false)
					.open(&in_fifo_path)
				{
					Ok(f) => f,
					Err(e) => {
						println!("Failed to open fifo: {:?}", e);
						return false;
					}
				};

				let mut data: Vec<u8> = Vec::new();
				let mut buff = [0u8; 1024];
				loop {
					match fifo.read(&mut buff) {
						Ok(a) => {
							if a == 0 {
								break;
							}
							data.extend_from_slice(&buff[0..a]);
						}
						Err(_) => {
							break;
						}
					};
				}

				drop(fifo);

				if let Err(e) = self.stream.write_all(&data) {
					println!("Failed to write to stream: {:?}", e);
					return false;
				}

				_ = fs::remove_file(&out_fifo_path);
				_ = fs::remove_file(&in_fifo_path);

				true
			}
			IpcMessage::Close => {
				println!("Request denied");
				false
			}
			any => {
				println!("Invalid IPC message: {:?}", any);
				false
			}
		}
	}


    pub fn run(&mut self, content_root: &Path) {
        _ = self.stream.set_nonblocking(true);
        let mut data: Vec<u8> = Vec::new();

        let mut alive = true;

        while alive {
            let mut buffer: [u8; 1024] = [0; 1024];
            match self.stream.read(&mut buffer) {
                Ok(size) => {
                    if size > 0 {
                        data.extend_from_slice(&buffer[0..size]);
                    } else {
                        alive = false;
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        if data.len() > 0 {
                            match HttpRequest::parse(&data) {
                                Some(req) => {
                                    println!(
                                        "[HttpClient] {} {}",
                                        req.method.to_str(),
                                        req.url.to_string()
                                    );

                                    if !self.handle_static_file_request(&req, content_root) && !self.handle_dynamic_request(&req) {
										let content = "500: Internal server error".as_bytes();
										let resp = self.mk_response(
											500,
											HashMap::from([
												("Content-Type".to_string(), "text/plain".to_string())
											]),
											content.to_vec(),
											req.version.clone()
										);

										if let Err(e) = self.stream.write_all(resp.serialize().as_ref()) {
											println!("Failed to write response to stream: {:?}", e);
											break;
										}
                                    }
                                }
                                None => {
                                    println!("Failed to parse http request");
                                }
                            };
                            data.clear();
                        }
                    } else {
                        println!("{}", e);
                        alive = false;
                        continue;
                    }
                }
            };
        }
    }
}
