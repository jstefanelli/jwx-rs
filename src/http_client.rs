use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{atomic, Arc, Mutex};
use crate::http::http_message::HttpMessage;
use crate::http::http_request::HttpRequest;
use crate::ipc::{IpcMessage};
use crate::ipc::request_pipe::RequestPipe;
use crate::utils::new_named_pipe;

pub struct HttpClient {
    stream: TcpStream,
    address: SocketAddr,
    sender: Arc<Mutex<RequestPipe>>
}

static CLIENT_COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

impl HttpClient {
    pub fn new(stream: TcpStream, address: SocketAddr, lua_send: Arc<Mutex<RequestPipe>>) -> Self {
        Self {
            stream,
            address,
            sender: lua_send
        }
    }

    pub fn run(&mut self) {
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
                },
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        if data.len() > 0 {
                            match HttpRequest::parse(&data) {
                                Some(req) => {
                                    println!("[HttpClient] {} {}", req.method.to_str(), req.url.to_string());
                                    let id = CLIENT_COUNTER.fetch_add(1, atomic::Ordering::AcqRel);

                                    let name = format!("{}_{:?}", std::process::id(), id);
                                    let out_name = format!("{name}.out");
                                    let in_name = format!("{name}.in");

                                    let out_fifo_path = match new_named_pipe(&out_name) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            println!("Failed to create named pipe: {:?}", e);
                                            return;
                                        }
                                    };

                                    let in_fifo_path = match new_named_pipe(&in_name) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            println!("Failed to create named pipe: {:?}", e);
                                            return;
                                        }
                                    };

                                    let mut l = match self.sender.lock() {
                                        Ok(s) => s,
                                        Err(_) => {
                                            println!("Error while locking send mutex");
                                            return;
                                        }
                                    };

                                    let msg = match l.send_message_and_wait(IpcMessage::Request { request_path: name.clone() }) {
                                        Ok(msg) => msg,
                                        Err(e) => {
                                            println!("Failed to send IPC request: {:?}", e);
                                            return;
                                        }
                                    };

                                    drop(l);

                                    match msg {
                                        IpcMessage::Ok => {
                                            let mut fifo = match File::options().read(false).write(true).open(&out_fifo_path) {
                                                Ok(f) => f,
                                                Err(e) => {
                                                    println!("Failed to open fifo: {:?}", e);
                                                    return;
                                                }
                                            };

                                            let data = req.serialize();

                                            let size: u64 = data.len() as u64;
                                            let size_buff = size.to_ne_bytes();
                                            if let Err(e) = fifo.write_all(&size_buff) {
                                                println!("Failed to write request size to fifo: {:?}", e);
                                                return;
                                            }

                                            if let Err(e) = fifo.write_all(&data) {
                                                println!("Failed to write to fifo: {:?}", e);
                                                return;
                                            }

                                            let mut fifo = match File::options().read(true).write(false).open(&in_fifo_path) {
                                                Ok(f) => f,
                                                Err(e) => {
                                                    println!("Failed to open fifo: {:?}", e);
                                                    return;
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
                                                    },
                                                    Err(_) => {
                                                        break;
                                                    }
                                                };
                                            }

                                            drop(fifo);

                                            if let Err(e) = self.stream.write_all(&data) {
                                                println!("Failed to write to stream: {:?}", e);
                                                return;
                                            }

                                            _ = fs::remove_file(&out_fifo_path);
                                            _ = fs::remove_file(&in_fifo_path);

                                        },
                                        IpcMessage::Close => {
                                            println!("Request denied");
                                            break;
                                        },
                                        any => {
                                            println!("Invalid IPC message: {:?}", any);
                                        }
                                    }
                                },
                                None => {
                                    println!("Failed to parse http request");
                                }
                            };
                            data.clear();
                        }
                    } else {
                        println ! ("{}", e);
                        alive = false;
                        continue;
                    }
                }
            };
        }
        println!("Client shutting down");
    }
}