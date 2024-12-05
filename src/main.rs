mod http_message;
mod url;

use std::io::{Read, Write};
use std::net::TcpListener;
use mlua::prelude::*;
use crate::http_message::HttpRequest;

fn main() -> LuaResult<()> {
    let listener = TcpListener::bind("127.0.0.1:4955")?;

    loop {
        match listener.accept() {
            Ok((mut stream, addr)) => {
                let mut vec: Vec<u8> = Vec::new();
                match stream.read_to_end(&mut vec) {
                    Ok(res) => {
                        if match HttpRequest::parse(&vec[0..res]) {
                            Some(_) => true,
                            None => false,
                        } {
                            let response = "HTTP/1.1 200 OK\r\n\r\n";
                            stream.write(response.as_bytes())?;
                        }
                    },
                    Err(err) => {
                        println!("{}", err);
                    }
                }
            },
            Err(e) => {
                println!("accept error = {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
