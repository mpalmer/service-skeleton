#![allow(unused_crate_dependencies)]

use service_skeleton::{metric::counter, service, ServiceConfig};

use std::{thread::sleep, time::Duration};

#[derive(Clone, Debug, ServiceConfig)]
struct Config {
	#[config(default_value = "World")]
	name: String,
}

fn main() {
	service("Hello")
		.counter::<Vec<(&str, String)>>("count", "Number of times we've said hello")
		.run(|cfg| say_hello(cfg));
}

fn say_hello(cfg: Config) {
	println!("Hello, {}!", cfg.name);
	counter("count", &vec![("name", cfg.name)], |c| {
		c.inc();
	});
	sleep(Duration::from_secs(5));
}
