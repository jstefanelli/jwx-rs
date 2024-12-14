use std::net::TcpStream;
use crate::http::http_request::HttpRequest;

trait RequestHandler {
    fn handle(&mut self, request: &HttpRequest, stream: &mut TcpStream) -> bool;
}

struct RequestRouterNode<'a> {
    name: String,
    children: Vec<RequestRouterNode<'a>>,
    handler: Option<&'a dyn RequestHandler>,
}

pub struct RequestRouter<'a> {
    handlers: Vec<Box<dyn RequestHandler>>,
    root: RequestRouterNode<'a>
}

impl<'a> RequestRouter<'a> {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            root: RequestRouterNode {
                name: "/".to_string(),
                children: Vec::new(),
                handler: None
            }
        }
    }

    pub fn add_handler(&mut self, route: &str,  handler: Box<dyn RequestHandler>) {

    }
}