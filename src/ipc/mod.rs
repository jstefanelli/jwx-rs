pub mod request_pipe;

use std::io::{Read, Write};

#[derive(Debug)]
pub enum IpcMessage {
	Poll,
	Request{ request_path: String },
	Ok,
	Close
}

pub trait IpcMessageSender {
	fn send_message(&mut self, msg: IpcMessage) -> Result<(), std::io::Error>;
}

pub trait IpcMessageReceiver {
	fn read_message(&mut self) -> Result<IpcMessage, std::io::Error>;
}

impl <T: Write> IpcMessageSender for T {
	fn send_message(&mut self, msg: IpcMessage) -> Result<(), std::io::Error> {
		match msg {
			IpcMessage::Poll => {
				let preamble = ['p' as u8];
				self.write_all(&preamble)
			},
			IpcMessage::Ok => {
				let preamble = ['o' as u8];
				self.write_all(&preamble)
			}
			IpcMessage::Request { request_path } => {
				let data = request_path.as_bytes();
				let len = (data.len() as u64).to_ne_bytes();

				let preamble = ['r' as u8];
				match self.write_all(&preamble) {
					Ok(_) => {},
					Err(e) => {
						return Err(e);
					}
				}

				match self.write(&len) {
					Ok(_) => {},
					Err(e) => {
						return Err(e);
					}
				}

				self.write_all(data)
			},
			IpcMessage::Close => {
				let preamble = ['c' as u8];
				self.write_all(&preamble)
			}
		}
	}
}

impl <T: Read> IpcMessageReceiver for T {
	fn read_message(&mut self) -> Result<IpcMessage, std::io::Error> {
		let mut preamble: [u8; 1] = [0];

		if let Err(e) = self.read_exact(&mut preamble) {
			return Err(e);
		}

		match preamble[0] as char {
			'p' => Ok(IpcMessage::Poll),
			'c' => Ok(IpcMessage::Close),
			'o' => Ok(IpcMessage::Ok),
			'r' => {
				let mut len: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
				if let Err(e) = self.read_exact(&mut len) {
					return Err(e);
				}

				let len = u64::from_ne_bytes(len);
				let mut data = vec![0; len as usize];
				if let Err(e) = self.read(&mut data) {
					return Err(e);
				}

				Ok(IpcMessage::Request{ request_path: String::from_utf8(data).unwrap() })
			}
			_ => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid IPC message"))
		}
	}
}