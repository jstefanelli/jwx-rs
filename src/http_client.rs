use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use crate::http::http_request::HttpRequest;

pub struct HttpClient {
    stream: TcpStream,
    address: SocketAddr
}

impl HttpClient {
    pub fn new(stream: TcpStream, address: SocketAddr) -> Self {
        HttpClient {
            stream,
            address
        }
    }

    pub fn run(&mut self) {
        _ = self.stream.set_nonblocking(true);
        println!("new connection");
        let mut data: Vec<u8> = Vec::new();

        let mut alive = true;

        while alive {
            let mut buffer: [u8; 1024] = [0; 1024];
            match self.stream.read(&mut buffer) {
                Ok(size) => {
                    if size > 0 {
                        println!("read {} bytes", size);
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
                                    println!("Parsed request");

                                    if req.url.uri == "/" {
                                        _ = self.stream.write("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nOk!".as_bytes());
                                    } else {
                                        _ = self.stream.write("HTTP/1.1 404 NOT FOUND\r\nContent-Length: 0\r\n\r\n".as_bytes());
                                    }
                                    _ = self.stream.flush();
                                },
                                None => {
                                    println!("Failed to parse http request");
                                }
                            };

                            println ! ("Clearing data");
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