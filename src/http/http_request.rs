use std::collections::HashMap;
use crate::http::http_message::{HttpMessage, HttpMethod, HttpVersion};
use crate::url::URL;

pub struct HttpRequest {
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
    pub version: HttpVersion,
    pub url: URL,
}

impl HttpRequest {
    pub fn parse(data: &[u8]) -> Option<HttpRequest> {
        let mut this = HttpRequest {
            method: HttpMethod::Get,
            headers: HashMap::new(),
            content: vec![],
            version: HttpVersion::Http1_0,
            url: URL { uri: "".to_string(), queries: HashMap::new() },
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
                match URL::parse(next_section[0..idx].trim()) {
                    Some(url) => {
                        if let Some(v) = HttpVersion::from_str(next_section[idx..].trim()) {
                            self.version = v;
                        } else {
                            return false
                        }
                        url
                    }

                    None => return false
                }
            },
            None => return false
        };

        true
    }

    fn get_first_line(&self) -> String {
        let version_str = self.version.to_str().to_string();
        let method = self.method.to_str().to_string();
        let path = self.url.to_string();
        format!("{method} {path} {version_str}\r\n")
    }

    fn get_headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    fn get_content(&self) -> &[u8] {
        &self.content
    }

    fn register_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }

    fn register_content(&mut self, data: &[u8]) {
        self.content.extend_from_slice(data);
    }
}


#[cfg(test)]
mod test {
    use crate::http::http_message::HttpMethod;
    use crate::http::http_request::HttpRequest;

    #[test]
    pub fn test_http_request() {
        let data = b"GET / HTTP/1.1\r\nHost: example.org\r\n\r\n";

        let req = HttpRequest::parse(data);
        assert!(req.is_some());

        let req = req.unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.url.uri, "/");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers["Host"], "example.org");
    }
}