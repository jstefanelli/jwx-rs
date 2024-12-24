use std::fs::File;
use crate::ipc::{IpcMessage, IpcMessageReceiver, IpcMessageSender};

pub struct RequestPipe {
	send: File,
	recv: File
}

impl RequestPipe {
	pub fn new(send: File, recv: File) -> Self {
		Self {
			send,
			recv
		}
	}

	pub fn send_message_and_wait(&mut self, message: IpcMessage) -> Result<IpcMessage, std::io::Error> {
		if let Err(e) = self.send.send_message(message) {
			return Err(e);
		}

		self.recv.read_message()
	}
}