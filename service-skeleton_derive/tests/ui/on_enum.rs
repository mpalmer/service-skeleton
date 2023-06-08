#![allow(unused_imports, unused_crate_dependencies)]
use service_skeleton_derive::ServiceConfig;

#[derive(ServiceConfig)]
enum Config {
	Foo,
	Bar,
}

fn main() {}
