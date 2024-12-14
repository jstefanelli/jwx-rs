use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
	let profile = env::var("PROFILE").unwrap_or("debug".to_string());
	let target_str = env::var("CARGO_TARGET_DIRE").unwrap_or_else( |_| {
		let mut tgt = "./target/".to_string();
		tgt.push_str(&profile);
		tgt.push_str("/");

		tgt
	});
	let target = Path::new(&target_str);

	println!("Target: {target_str}");

	Command::new("cp")
		.arg("-rf")
		.arg("./src/resources/lua")
		.arg(target.join("lua"))
		.spawn().unwrap().wait().expect("Failed to copy lua files");


	println!("cargo::rerun-if-changed=./src/resources/lua");
}