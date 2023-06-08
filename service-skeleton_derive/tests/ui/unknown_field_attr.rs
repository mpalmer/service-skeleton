#![allow(unused_imports, unused_crate_dependencies)]
use service_skeleton_derive::ServiceConfig;

#[derive(ServiceConfig)]
struct Config {
	#[config(something)]
	foo: bool,
}

fn main() {}
