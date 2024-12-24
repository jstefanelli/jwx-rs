use std::collections::HashMap;
use std::io::{Error};
use crate::http::http_request::HttpRequest;
use crate::http::http_response::HttpResponse;

pub trait Behaviour {
    fn run(&self,request: &HttpRequest, params: HashMap<String, String>) -> Result<HttpResponse, Error>;
}