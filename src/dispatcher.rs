use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use crate::behaviours::behaviour::Behaviour;
use crate::behaviours::behaviour_router::BehaviourRouter;
use crate::behaviours::lua_behaviour::LuaBehaviour;
use crate::config::lua_config::ConfigMgr;
use crate::http::http_message::HttpMessage;
use crate::http::http_request::HttpRequest;
use crate::ipc::{IpcMessage, IpcMessageReceiver, IpcMessageSender};
use crate::utils::{safe_fork, ForkResult};

pub fn run_lua_dispatcher(config_mgr: ConfigMgr, mut lua_recv: File, mut control_send: File) -> std::io::Result<()> {
	let mut behaviours: HashMap<String, Box<dyn Behaviour>> = HashMap::new();
	for e in config_mgr.get_endpoints() {
		if e.1.to_lowercase().ends_with(".lua") {
			let b = LuaBehaviour::new(&config_mgr, &e.1)?;

			behaviours.insert(e.0.clone(), Box::new(b));
		} else {
			println!("Unknown behaviour type for endpoint {}. Suppoerted types are: .lua", e.0);
		}

	}

	let router = BehaviourRouter::new(behaviours);

	loop {
		let msg = lua_recv.read_message()?;
		match msg {
			IpcMessage::Poll => {
				control_send.send_message(IpcMessage::Ok)?;
			}
			IpcMessage::Ok => {
				control_send.send_message(IpcMessage::Ok)?;
			}
			IpcMessage::Request {request_path } => {
				match safe_fork() {
					Ok(ForkResult::Parent(_)) => {
						control_send.send_message(IpcMessage::Ok)?;
					}
					Ok(ForkResult::Child) => {
						drop(control_send);
						drop(lua_recv);

						let out_name = format!("/tmp/jwx_client_{request_path}.out");
						let in_name = format!("/tmp/jwx_client_{request_path}.in");

						if Path::new(&out_name).exists() && Path::new(&in_name).exists() {

							let mut stream = match File::options().read(true).write(false).open(&out_name) {
								Ok(stream) => stream,
								Err(e) => {
									println!("Failed to open FIFO: {:?}", e);
									return Err(e)
								}
							};

							let mut len_buff = [0u8; 8];
							stream.read_exact(&mut len_buff)?;
							let len = u64::from_ne_bytes(len_buff);

							let mut data = vec![0u8; len as usize];
							stream.read_exact(&mut data)?;

							let request = match HttpRequest::parse(&data) {
								Some(r) => r,
								None => {
									println!("Error while parsing request.");
									return Ok(());
								}
							};

							drop(stream);

							let mut stream = match File::options().read(false).write(true).open(&in_name) {
								Ok(stream) => stream,
								Err(e) => {
									println!("Failed to open FIFO: {:?}", e);
									return Err(e)
								}
							};

							let response = router.run(&request).serialize();
							stream.write_all(&response)?;

							drop(stream);
						}

						return Ok(());
					},
					Err(e) => {
						println!("Error while forking: {:?}", e);
						control_send.send_message(IpcMessage::Close)?;
						break
					}
				}
			},
			IpcMessage::Close => {
				print!("Dispatcher: CLOSE");
				break
			}
		}
	}

	println!("[Dispatcher] Exiting");

	Ok(())
}