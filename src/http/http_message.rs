use std::collections::HashMap;

#[derive(PartialEq, Clone)]
pub enum HttpVersion {
    Http1_0,
    Http1_1,
}

impl HttpVersion {
    pub fn from_str(string: &str) -> Option<HttpVersion> {
        match string {
            "HTTP/1.0" => Some(HttpVersion::Http1_0),
            "HTTP/1.1" => Some(HttpVersion::Http1_1),
            _ => None
        }
    }

    pub fn to_str(&self) -> &'static str {
        match *self {
            HttpVersion::Http1_0 => "HTTP/1.0",
            HttpVersion::Http1_1 => "HTTP/1.1",
        }
    }
}

#[derive(PartialEq, Debug)]
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
    pub fn from_str(string: &str) -> Option<HttpMethod> {
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

    pub fn to_str(&self) -> &'static str {
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

#[derive(PartialEq)]
enum MessageParseState {
    FirstLine,
    Headers,
    Content
}

pub trait HttpMessage {
    fn parse_first_line(&mut self, line: &str) -> bool;
    fn get_first_line(&self) -> String;
    fn get_headers(&self) -> &HashMap<String, String>;
    fn get_content(&self) -> &[u8];
    fn register_header(&mut self, name: &str, value: &str);

    fn register_content(&mut self, data: &[u8]);

    fn load(&mut self, data: &[u8]) -> bool {
        if data.len() == 0 {
            return false;
        }

        let mut current_line: &str = "";
        let mut state = MessageParseState::FirstLine;

        let mut idx = 0;
        while idx < data.len() {
            if state == MessageParseState::Content {
                self.register_content(&data[idx..]);
                return true
            }

            let mut i: usize = 0;
            while i < data.len() - idx {
                if data[idx + i] == '\n' as u8 && i > 1 && data[idx + i - 1] == '\r' as u8 {
                    current_line = match std::str::from_utf8(&data[idx..(idx + i - 1)]) {
                        Ok(string) => string,
                        Err(_) => return false
                    };

                    if current_line != "" {
                        break
                    }
                }
                i += 1;
            }

            if current_line.len() == 0 {
                current_line = match std::str::from_utf8(&data[idx..]) {
                    Ok(string) => string,
                    Err(_) => return false
                }
            }

            idx += i;

            match state {
                MessageParseState::FirstLine => {
                    if !self.parse_first_line(current_line) {
                        return false;
                    }
                    state = MessageParseState::Headers;
                },
                MessageParseState::Headers => {
                    if current_line.len() == 0 {
                        state = MessageParseState::Content;
                    }
                    self.parse_header(current_line);
                },
                _ => {}
            }

            current_line = "";
        }

        true
    }

    fn parse_header(&mut self, line: &str) {
        let idx = match line.find(": ") {
            Some(idx) => idx,
            None => return
        };

        if idx + 1 >= line.len() {
            return;
        }

        let name = line[..idx].trim();
        let val = line[idx+1..].trim();

        self.register_header(name, val);
    }

    fn serialize(&self) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();

        let first_line = self.get_first_line();
        res.extend_from_slice(first_line.as_bytes());

        for h in self.get_headers() {
            res.extend_from_slice(h.0.as_bytes());
            res.extend_from_slice(b": ");
            res.extend_from_slice(h.1.as_bytes());
            res.extend_from_slice(b"\r\n");
        }
        res.extend_from_slice(b"\r\n");

        res.extend_from_slice(self.get_content());

        res
    }
}
