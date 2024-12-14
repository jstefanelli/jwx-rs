use std::collections::HashMap;
use std::net::TcpStream;
use crate::http::http_request::HttpRequest;

pub trait Behaviour {
    fn run(&self, request: &HttpRequest, params: HashMap<String, String>, stream: &mut TcpStream) -> bool;
}