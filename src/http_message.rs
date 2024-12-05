use std::collections::HashMap;
use crate::url::URL;

pub enum HttpVersion {
    Http1_0,
    Http1_1,
}

impl HttpVersion {
    fn from_str(string: &str) -> Option<HttpVersion> {
        match string {
            "HTTP/1.0" => Some(HttpVersion::Http1_0),
            "HTTP/1.1" => Some(HttpVersion::Http1_1),
            _ => None
        }
    }

    fn to_str(&self) -> &'static str {
        match *self {
            HttpVersion::Http1_0 => "HTTP/1.0",
            HttpVersion::Http1_1 => "HTTP/1.1",
        }
    }
}

pub enum HttpMethod {
    Get,
    Post,
    Head,
    Options,
    Put,
    Patch,
    Delete
}

impl HttpMethod {
    fn from_str(string: &str) -> Option<HttpMethod> {
        let s = string.to_lowercase();

        match s.as_str() {
            "get" => Some(HttpMethod::Get),
            "post" => Some(HttpMethod::Post),
            "head" => Some(HttpMethod::Head),
            "options" => Some(HttpMethod::Options),
            "put" => Some(HttpMethod::Put),
            "patch" => Some(HttpMethod::Patch),
            "delete" => Some(HttpMethod::Delete),
            _ => None
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}

pub struct HttpRequest {
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
    pub version: HttpVersion,
    pub url: URL,
}

pub trait HttpMessage {
    fn parse_first_line(&mut self, line: &str) -> bool;

    fn load(&mut self, data: &[u8]) -> bool {
        if data.len() == 0 {
            return false;
        }

        let mut current_line: &str = "";

        for i in 0..data.len() {
            if data[i] == '\n' as u8 && i > 1 && data[i - 1] == '\r' as u8 {
                current_line = match std::str::from_utf8(&data[0..(i-2)]) {
                    Ok(string) => string,
                    Err(_) => return false
                }
            }
        }

        if current_line.len() == 0 {
            current_line = match std::str::from_utf8(data) {
                Ok(string) => string,
                Err(_) => return false
            }
        }

        if !self.parse_first_line(current_line) {
            return false;
        }

        return true;


    }
}

impl HttpRequest {
    pub fn parse(data: &[u8]) -> Option<HttpRequest> {
        let mut this = HttpRequest {
            method: HttpMethod::Get,
            headers: Default::default(),
            content: vec![],
            version: HttpVersion::Http1_0,
            url: URL { uri: "".to_string(), queries: Default::default() },
        };

        if this.load(data) {
            return Some(this);
        }
        None
    }
}

impl HttpMessage for HttpRequest {
    fn parse_first_line(&mut self, line: &str) -> bool {

        let next_section: &str = match line.find(' ') {
            Some(idx) => {
                if let Some(m) = HttpMethod::from_str(line[0..idx].trim()) {
                    self.method = m;
                    line[idx..].trim()
                } else {
                    return false
                }
            },
            None => return false
        };

        self.url = match next_section.find(' ') {
            Some(idx) => {
                match URL::parse(line[0..idx].trim()) {
                    Some(url) => url,
                    None => return false
                }
            },
            None => return false
        };

        return true;
    }
}
