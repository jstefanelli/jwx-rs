use std::collections::HashMap;
use std::fs;
use std::path::Path;
use mlua::prelude::*;
use mlua::{Table, Error};

pub struct ConfigMgr {
	config_directory: String,
	endpoints: HashMap<String, String>,
	library_folders: Vec<String>
}

const CONFIG_ENV_CFG_PATH_NAME: &'static str = "internal_config_path";

impl ConfigMgr {
	pub fn new(config_dr: &str) -> Self {
		ConfigMgr {
			config_directory: config_dr.to_string(),
			endpoints: HashMap::new(),
			library_folders: Vec::new()
		}
	}

	pub fn add_library_folder(&mut self, folder: &str) {
		self.library_folders.push(folder.to_string());
	}

	pub fn set_endpoint(&mut self, endpoint: &str, script: &str) {
		self.endpoints.insert(endpoint.to_string(), script.to_string());
	}

	pub fn remove_endpoint(&mut self, endpoint: &str) {
		self.endpoints.remove(endpoint);
	}

	pub fn get_library_folders(&self) -> &Vec<String> {
		&self.library_folders
	}

	pub fn get_endpoints(&self) -> &HashMap<String, String> {
		&self.endpoints
	}

	pub fn append_library_folders(&self, lua: &Lua) {
		let package_table: Table = match lua.globals().get("package") {
			Ok(v) => v,
			Err(_) => return //TODO: Log error
		};

		let package_path: String = match package_table.get("path") {
			Ok(v) => v,
			Err(_) => return //TODO: Log error
		};

		let mut full_path = String::new();
		for folder in self.library_folders.iter() {
			full_path.push_str(folder);
			full_path.push_str(";");
		}

		full_path.push_str(&package_path);

		match package_table.set("path", full_path) {
			Ok(_) => {},
			Err(_) => return //TODO: Log error
		}
	}

	pub fn run_config(&mut self, config_path: &str) {
		let lua = Lua::new();
		self.append_library_folders(&lua);

		let endpoints = self.endpoints.clone();
		let library_folders = self.library_folders.clone();

		lua.globals().set("config_endpoints", endpoints).unwrap();
		lua.globals().set("config_library_folders", library_folders).unwrap();

		if let Err(e) = lua.globals().set(CONFIG_ENV_CFG_PATH_NAME, self.config_directory.to_string()) {
			println!("[ConfigMgr] Error setting config_directory: {}", e);
			return
		}

		let set_config_endpoint = match lua.create_function(|lua: &Lua, args: (String, String)| -> Result<i32, Error> {
			let mut endpoints: HashMap<String, String> = lua.globals().get("config_endpoints").unwrap();
			endpoints.insert(args.0, args.1);
			lua.globals().set("config_endpoints", endpoints).unwrap();

			Ok(0)
		}) {
			Ok(f) => f,
			Err(e) => {
				println!("[ConfigMgr] Error creating config_set_endpoint: {}", e);
				return
			}
		};

		if let Err(e) = lua.globals().set("config_set_endpoint", set_config_endpoint) {
			println!("[ConfigMgr] Error setting config_set_endpoint: {}", e);
			return
		}

		let remove_config_endpoint = match lua.create_function(|lua: &Lua, endpoint: String| -> Result<i32, Error> {
			let mut endpoints: HashMap<String, String> = lua.globals().get("config_endpoints").unwrap();
			endpoints.remove(&endpoint);
			lua.globals().set("config_endpoints", endpoints).unwrap();
			Ok(0)
		}) {
			Ok(f) => f,
			Err(e) => {
				println!("[ConfigMgr] Error creating config_remove_endpoint: {}", e);
				return
			}
		};

		if let Err(e) = lua.globals().set("config_remove_endpoint", remove_config_endpoint) {
			println!("[ConfigMgr] Error setting config_remove_endpoint: {}", e);
			return
		}

		let add_config_library_folder = match lua.create_function(|lua: &Lua, folder: String| -> Result<i32, Error> {
			let mut library_folders: Vec<String> = lua.globals().get("config_library_folders").unwrap();
			library_folders.push(folder);
			lua.globals().set("config_library_folders", library_folders).unwrap();
			Ok(0)
		}) {
			Ok(f) => f,
			Err(e) => {
				println!("[ConfigMgr] Error creating config_add_library_folder: {}", e);
				return
			}
		};

		if let Err(e) = lua.globals().set("config_add_library_folder", add_config_library_folder) {
			println!("[ConfigMgr] Error setting config_add_library_folder: {}", e);
			return
		}

		let p = Path::new(config_path);
		let mut target = p.to_str().unwrap().to_string();
		if p.is_relative() && !p.exists() {
			target = Path::new(&self.config_directory).join(p).to_str().unwrap().to_string();
		}

		println!("[ConfigMgr] Running config: {}", target);

		let stream = match fs::read_to_string(target) {
			Ok(s) => s,
			Err(e) => {
				println!("[ConfigMgr] Error reading config file: {}", e);
				return
			}
		};

		if let Err(e) = lua.load(stream).exec() {
			println!("[ConfigMgr] Error executing config file: {}", e);
			return
		}

		self.library_folders = lua.globals().get("config_library_folders").unwrap();
		self.endpoints = lua.globals().get("config_endpoints").unwrap();
	}
}
