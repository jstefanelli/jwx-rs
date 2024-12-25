use std::collections::HashMap;
use std::fmt::Display;

pub struct URL {
	pub uri: String,
	pub queries: HashMap<String, String>
}

impl URL {
	pub fn parse(string: &str) -> Option<URL> {
		let string = string.trim();
		let question_idx = match string.find('?') {
			Some(idx) => idx,
			None => {
				return Some(URL {
					uri: string.to_string(),
					queries: HashMap::new()
				})
			}
		};

		let uri = string[0..question_idx].to_string();
		if question_idx == string.len() {
			return Some(URL {
				uri: uri,
				queries: HashMap::new()
			})
		}

		let string = string[question_idx + 1..].to_string();

		let mut queries = HashMap::new();
		let mut work_idx: Option<usize> = Some(0);
		'a: loop {
			let start = match work_idx {
				Some(idx) => idx,
				None => break 'a
			};
			let string = &string[start..];

			let eq_idx = match string.find('=') {
				Some(idx) => idx,
				None => break 'a
			};


			let name = string[0..eq_idx].trim();
			let val = match string.find('&') {
				Some(and_idx) => {
					work_idx = if and_idx < string.len() - 1 {
						Some(start + and_idx + 1)
					} else {
						None
					};

					if eq_idx == and_idx - 1 {
						""
					} else {
						string[(eq_idx + 1)..and_idx].trim()
					}
				}
				None => {
					work_idx = None;
					string[(eq_idx + 1)..].trim()
				}
			};

			queries.insert(String::from(name), String::from(val));
		};

		return Some(URL {
			uri: uri,
			queries: queries
		});
	}
}

impl Display for URL {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut res = self.uri.clone();
		if !self.queries.is_empty() {
			res = res + "?";
			let mut first = true;
			for (k, v) in &self.queries {
				if !first {
					res = res + "&";
				} else {
					first = false;
				}

				res = res + k + "=" + v;
			}
		}

		write!(f, "{}", res)
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn test_url_0() {
		let data = "/some/url?query0=0&query1=1";
		let url = URL::parse(data);
		assert!(url.is_some());

		let url = url.unwrap();
		assert_eq!(url.uri, "/some/url");
		assert_eq!(url.queries.len(), 2);

		assert!(url.queries.get("query0").is_some());
		assert_eq!(url.queries.get("query0").unwrap(), "0");

		assert!(url.queries.get("query1").is_some());
		assert_eq!(url.queries.get("query1").unwrap(), "1");
	}

	#[test]
	pub fn test_url_1() {
		let data = "/lmao";

		let url = URL::parse(data);
		assert!(url.is_some());

		let url = url.unwrap();
		assert_eq!(url.uri, "/lmao");
		assert_eq!(url.queries.len(), 0);
	}

	#[test]
	pub fn test_url_2() {
		let data = "/test?a=b&c=&d=e";
		let url = URL::parse(data);
		assert!(url.is_some());

		let url = url.unwrap();
		assert_eq!(url.uri, "/test");
		assert_eq!(url.queries.len(), 3);

		assert!(url.queries.get("a").is_some());
		assert_eq!(url.queries.get("a").unwrap(), "b");

		assert!(url.queries.get("c").is_some());
		assert_eq!(url.queries.get("c").unwrap(), "");

		assert!(url.queries.get("d").is_some());
		assert_eq!(url.queries.get("d").unwrap(), "e");
	}
}