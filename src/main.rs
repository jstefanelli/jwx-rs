mod url;
mod utils;
mod http_client;
mod ipc;
mod dispatcher;

mod http {
	pub mod http_message;
	pub mod http_request;
	pub mod http_response;
}

mod behaviours {
	pub mod lua_behaviour;
	pub mod behaviour_router;
	pub mod behaviour;
}

mod config {
	pub mod lua_config;
}

use std::collections::HashMap;
use std::{env, thread};
use std::fs::File;
use std::net::TcpListener;
use std::path::{Path};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle};
use http_client::HttpClient;
use crate::config::lua_config::ConfigMgr;
use crate::dispatcher::run_lua_dispatcher;
use crate::ipc::{IpcMessage};
use crate::ipc::request_pipe::RequestPipe;
use crate::utils::{new_pipe, safe_fork, ForkResult};

struct ArgDefinition {
	name: String,
	shorthand: Option<String>,
	has_value: bool,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn parse_args(args: &Vec<String>, definitions: &Vec<ArgDefinition>) -> Result<HashMap<String, String>, String> {
	let mut current_arg: Option<&ArgDefinition> = None;
	let mut values: HashMap<String, String> = HashMap::new();

	if args.len() <= 1 {
		return Ok(values);
	}

	'outer: for arg in &args[1..] {

		let short = Some(arg.to_string());
		for def in definitions {
			if arg == &def.name || short == def.shorthand {
				if let Some(curr) = current_arg {
					if curr.has_value {
						return Err(format!("No value provided for argument {}", arg));
					}
				}

				if values.contains_key(&def.name) {
					return Err(format!("Argument {} already provided", arg));
				}

				if !def.has_value {
					values.insert(def.name.clone(), "true".to_string());
				} else {
					current_arg = Some(def);
				}
				continue 'outer;
			}
		}

		if let Some(curr) = current_arg {
			if !curr.has_value {
				return Err(format!("Argument {} requires no value", arg));
			}
			values.insert(curr.name.clone(), arg.to_string());
			current_arg = None;
		} else {
			return Err(format!("Unknown argument {}", arg));
		}
	}

	if let Some(arg) = current_arg {
		if arg.has_value {
			return Err(format!("No value provided for argument {}", arg.name));
		}
	}

	Ok(values)
}

fn run_listener(target_port: u16, lua_send: File, control_recv: File, content_root: &Path) -> Result<(), std::io::Error> {

	let addr = format!("0.0.0.0:{}", target_port);

	let listener = TcpListener::bind(addr)?;
	let mut client_threads: Vec<JoinHandle<()>> = Vec::new();

	let communicator = Arc::new(Mutex::new(RequestPipe::new(lua_send, control_recv)));

	loop {
		match listener.accept() {
			Ok((stream, addr)) => {
				let clone = communicator.clone();
				let root = content_root.to_path_buf();
				let t = thread::spawn(move || {
					HttpClient::new(
						stream,
						addr,
						clone,
						HashMap::from([
							("Server".to_string(), "jwx-rs/0.1.0".to_string()),
							("Connection".to_string(), "Keep-Alive".to_string()),
						])
					).run(&root);
				});

				client_threads.push(t);
			},
			Err(e) => {
				println!("accept error = {:?}", e);
				break;
			}
		}
	}


	let mut send = match communicator.lock() {
		Ok(s) => s,
		Err(_) => {
			println!("Error while locking send mutex");
			return Ok(());
		}
	};

	println!("[Listener] Sending close message");
	_ = send.send_message_and_wait(IpcMessage::Close);

	for j in client_threads.drain(..) {
		match j.join() {
			Ok(_) => (),
			Err(e) => println!("Error while joining client thread: {:?}", e)
		}
	}

	Ok(())
}

fn main() -> Result<(), std::io::Error> {
	let mut target_content_path = "./content";
	let mut target_config_path = "./config";
	let mut target_config_file = "jwx_config.lua";
	let mut target_port: u16 = 4955;

	let args: Vec<String> = env::args().collect();

	let definitions = vec![
		ArgDefinition {
			name: "--help".to_string(),
			shorthand: Some("-h".to_string()),
			has_value: false,
		},
		ArgDefinition {
			name: "--content-path".to_string(),
			shorthand: Some("-c".to_string()),
			has_value: true,
		},
		ArgDefinition {
			name: "--port".to_string(),
			shorthand: Some("-p".to_string()),
			has_value: true,
		},
		ArgDefinition {
			name: "--config-dir".to_string(),
			shorthand: None,
			has_value: true,
		},
		ArgDefinition {
			name: "--config-file".to_string(),
			shorthand: None,
			has_value: true,
		}
	];

	let my_args = parse_args(&args, &definitions).unwrap();

	if my_args.contains_key("--help") {
		println!("jwx-rs (John's Webserver eXperiment: Rust edition) {VERSION}");
		println!("Usage: {} [options]", &args[0]);
		println!("Options:");
		println!("  -h, --help              Show this help message and exit");
		println!("  -c, --content-path      Path to content directory (default: ./content)");
		println!("      --port              Port to listen on (default: 4955)");
		println!("      --config-dir        Path to config directory (default: ./config)");
		println!("      --config-file       Name of config file (default: jwx_config.lua)");
		return Ok(());
	}

	if let Some(path) = my_args.get("--content-path") {
		target_content_path = path;
	}

	if let Some(port) = my_args.get("--port") {
		target_port = match port.parse::<u16>() {
			Ok(p) => p,
			Err(_) => {
				println!("Invalid port: {}", port);
				return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid port"));
			}
		};
	}

	if let Some(path) = my_args.get("--config-dir") {
		target_config_path = path;
	}

	if let Some(file) = my_args.get("--config-file") {
		target_config_file = file;
	}


	let mut mgr = ConfigMgr::new(target_config_path);
	mgr.run_config(&target_config_file);

	let (lua_recv, lua_send) = match new_pipe() {
		Ok(res) => res,
		Err(e) => {
			return Err(e);
		}
	};

	let (control_recv, control_send) = match new_pipe() {
		Ok(res) => res,
		Err(e) => {
			return Err(e);
		}
	};

	match safe_fork() {
		Ok(ForkResult::Child) => {
			run_lua_dispatcher(mgr, lua_recv, control_send)
		},
		Ok(ForkResult::Parent(_)) => run_listener(target_port, lua_send, control_recv, Path::new(target_content_path)),
		Err(e) => {
			return Err(e)
		}
	}

}
