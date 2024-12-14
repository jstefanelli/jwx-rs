mod url;
mod utils;
mod request_router;
mod http_client;

mod http {
    pub mod http_message;
    pub mod http_request;
}

mod behaviours {
    pub mod behaviour_router;
    pub mod behaviour;
}

mod config {
    pub mod lua_config;
}

use std::collections::HashMap;
use std::env;
use std::net::TcpListener;
use mlua::prelude::*;
use http_client::HttpClient;
use crate::config::lua_config::ConfigMgr;
use crate::utils::{safe_fork, ForkResult};

struct ArgDefinition {
    name: String,
    shorthand: Option<String>,
    has_value: bool,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn parse_args(args: &Vec<String>, definitions: &Vec<ArgDefinition>) -> Result<HashMap<String, String>, String> {
    let mut current_arg: Option<&ArgDefinition> = None;
    let mut values: HashMap<String, String> = HashMap::new();
    let mut first = true;

    'outer: for arg in args {
        if first {
            first = false;
            continue;
        }

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

    Ok(values)
}

fn main() -> LuaResult<()> {
    let mut target_content_path = "./content";
    let mut target_config_path = "./config";
    let mut target_config_file = "jwx_config.lua";
    let mut target_port = "4955";

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
            name: "--config_dir_path".to_string(),
            shorthand: None,
            has_value: true,
        },
        ArgDefinition {
            name: "--config_file".to_string(),
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
        println!("      --config_dir_path   Path to config directory (default: ./config)");
        println!("      --config_file       Name of config file (default: jwx_config.lua)");;
        return Ok(());
    }

    if let Some(path) = my_args.get("--content-path") {
        target_content_path = path;
    }

    if let Some(port) = my_args.get("--port") {
        target_port = port;
    }

    if let Some(path) = my_args.get("--config_dir_path") {
        target_config_path = path;
    }

    if let Some(file) = my_args.get("--config_file") {
        target_config_file = file;
    }

    let addr = format!("0.0.0.0:{}", target_port);

    let listener = TcpListener::bind(addr)?;

    let mut mgr = ConfigMgr::new(target_config_path);
    mgr.run_config(&target_config_file);

    loop {
        match listener.accept() {
            Ok((stream, addr)) => {
                match safe_fork() {
                    Ok(ForkResult::Child) => {
                        HttpClient::new(stream, addr).run();
                    },
                    Ok(_) => {},
                    Err(e) => {
                        println!("Fork error: {}", e);
                        break;
                    }
                }
            },
            Err(e) => {
                println!("accept error = {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
