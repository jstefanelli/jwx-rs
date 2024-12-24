use std::collections::HashMap;
use mlua::{Function, Lua, Number, Table, Value};
use mlua::prelude::{LuaResult, LuaString};
use crate::behaviours::behaviour::Behaviour;
use crate::config::lua_config::ConfigMgr;
use crate::http::http_request::HttpRequest;
use crate::http::http_response::HttpResponse;

#[derive(Debug)]
pub enum LuaBehaviourError {
	IoError(std::io::Error),
	LuaError(mlua::Error)
}

impl From<LuaBehaviourError> for std::io::Error {
	fn from(e: LuaBehaviourError) -> Self {
		match e {
			LuaBehaviourError::IoError(e) => e,
			LuaBehaviourError::LuaError(e) => std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
		}
	}
}

const LUA_BEHAVIOUR_ENTRYPOINT_NAME: &'static str = "run_request";

pub struct LuaBehaviour {
	vm: Lua
}

impl LuaBehaviour {
	pub fn new(config_mgr: &ConfigMgr, script_path: &str) -> Result<LuaBehaviour, LuaBehaviourError> {
		let script_data = match std::fs::read(script_path) {
			Ok(data) => data,
			Err(e) => return Err(LuaBehaviourError::IoError(e))
		};

		let lua = Lua::new();
		config_mgr.append_library_folders(&lua);
		if let Err(e) = lua.load(script_data).exec() {
			return Err(LuaBehaviourError::LuaError(e));
		}

		Ok(LuaBehaviour {
			vm: lua
		})
	}

	fn table_to_hashmap(table: &Table) -> LuaResult<HashMap<String, String>> {
		let mut map: HashMap<String, String> = HashMap::new();
		for pair in table.pairs::<Value, Value>(){
			let (key, val) = pair?;

			let key_str = key.to_string()?;
			let val_str = val.to_string()?;

			map.insert(key_str, val_str);
		}

		Ok(map)
	}

	fn run_internal(&self, request: &HttpRequest, params: HashMap<String, String>) -> LuaResult<HttpResponse> {
		let request_table = self.vm.create_table()?;

		let headers_table = self.vm.create_table()?;
		for (header, val) in request.headers.iter() {
			headers_table.set(header.clone(), val.clone())?
		}
		request_table.set("headers", headers_table)?;

		let params_table = self.vm.create_table()?;
		for (key, val) in &params {
			params_table.set(key.clone(), val.clone())?
		}
		request_table.set("params", params_table)?;

		request_table.set("uri", request.url.uri.clone())?;

		let query_table = self.vm.create_table()?;
		for (k, v) in &request.url.queries {
			query_table.set(k.clone(), v.clone())?;
		}
		request_table.set("query", query_table)?;

		request_table.set("method", request.method.to_str().to_string())?;
		request_table.set("version", request.version.to_str().to_string())?;

		self.vm.globals().set("request", request_table)?;

		let func: Function = self.vm.globals().get(LUA_BEHAVIOUR_ENTRYPOINT_NAME.to_string())?;

		func.call(())?;

		let jwx: Table = self.vm.globals().get("jwx")?;
		let response: Table = jwx.get("response")?;

		let headers: Table = response.get("headers")?;
		let headers = LuaBehaviour::table_to_hashmap(&headers)?;
		let status_code: Number = response.get("statusCode")?;
		let content: LuaString = response.get("content")?;

		let response = HttpResponse::new(status_code as u16, headers,
										 content.as_bytes().to_vec(), request.version.clone());

		Ok(response)
	}
}

impl Behaviour for LuaBehaviour {
	fn run(&self, request: &HttpRequest, params: HashMap<String, String>) -> Result<HttpResponse, std::io::Error> {
		match self.run_internal(request, params) {
			Ok(req) => Ok(req),
			Err(e) => {
				Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
			}
		}
	}
}