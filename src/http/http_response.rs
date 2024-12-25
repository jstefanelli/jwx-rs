use std::collections::HashMap;
use std::str::FromStr;
use crate::http::http_message::{HttpMessage, HttpVersion};

pub struct HttpResponse {
	code: u16,
	headers: HashMap<String, String>,
	content: Vec<u8>,
	version: HttpVersion
}

pub fn code_to_http_status(code: u16) -> Option<&'static str> {
	Some(match code {
		100 => "Continue",
		101 => "Switching Protocols",
		102 => "Processing",
		103 => "Early Hints",
		200 => "OK",
		201 => "Created",
		202 => "Accepted",
		203 => "Non-Authoritative Information",
		204 => "No Content",
		205 => "Reset Content",
		206 => "Partial Content",
		207 => "Multi-Status",
		208 => "Already Reported",
		226 => "IM Used",
		300 => "Multiple Choices",
		301 => "Moved Permanently",
		302 => "Found",
		303 => "See Other",
		304 => "Not Modified",
		305 => "Use Proxy",
		307 => "Temporary Redirect",
		308 => "Permanent Redirect",
		400 => "Bad Request",
		401 => "Unauthorized",
		402 => "Payment Required",
		403 => "Forbidden",
		404 => "Not Found",
		405 => "Method Not Allowed",
		406 => "Not Acceptable",
		407 => "Proxy Authentication Required",
		408 => "Request Timeout",
		409 => "Conflict",
		410 => "Gone",
		411 => "Length Required",
		412 => "Precondition Required",
		413 => "Payload Too Large",
		414 => "URI Too Long",
		415 => "Unsupported Media Type",
		416 => "Range Not Satisfiable",
		417 => "Expectation Failed",
		418 => "I'm a teapot",
		421 => "Misdirected Request",
		422 => "Unprocessable Entity",
		423 => "Locked",
		424 => "Failed Dependency",
		425 => "Too Many Requests",
		426 => "Upgrade Required",
		428 => "Precondition Required",
		429 => "Too Many Requests",
		431 => "Request Header Fields Too Large",
		451 => "Unavailable For Legal Reasons",
		500 => "Internal Server Error",
		501 => "Not Implemented",
		502 => "Bad Gateway",
		503 => "Service Unavailable",
		504 => "Gateway Timeout",
		505 => "HTTP Version Not Supported",
		506 => "Variant Also Negotiates",
		507 => "Insufficient Storage",
		508 => "Loop Detected",
		510 => "Not Extended",
		511 => "Network Authentication Required",
		_ => return None
	})
}

impl HttpResponse {
	pub fn parse(from: &[u8]) -> Option<HttpResponse> {
		let mut this = HttpResponse {
			code: 500,
			headers: HashMap::new(),
			content: Vec::new(),
			version: HttpVersion::Http1_1
		};

		if this.load(from) {
			return Some(this);
		}

		None
	}

	pub fn new(code: u16, headers: HashMap<String, String>, content: Vec<u8>, version: HttpVersion) -> HttpResponse {
		let mut resp = HttpResponse {
			code,
			headers,
			version,
			content
		};

		//if !resp.headers.contains_key("Content-Length") { // Maybe leave this out?
			resp.headers.insert("Content-Length".to_string(), resp.content.len().to_string());
		//}

		resp
	}
}

impl HttpMessage for HttpResponse {
	fn parse_first_line(&mut self, line: &str) -> bool {
		let next_section: &str = match line.find(" ") {
			Some(idx) => {
				if let Some(v) = HttpVersion::from_str(&line[0..idx]) {
					self.version = v;
					line[idx..].trim()
				} else {
					return false
				}
			},
			None => return false
		};

		match next_section.find(" ") {
			Some(idx) => {
				self.code = match u16::from_str(&next_section[0..idx]) {
					Ok(c) => c,
					Err(_) => return false
				};
			},
			None => return false
		};

		//Ignoring the "OK", "Not Found" parts at the end. Using only the code

		true
	}

	fn get_first_line(&self) -> String {
		let code_str = code_to_http_status(self.code).unwrap_or("Unknown");
		format!("{} {} {}\r\n", self.version.to_str(), self.code, code_str)
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
		self.content = data.to_vec();
		self.headers.insert("Content-Length".to_string(), self.content.len().to_string());
	}
}