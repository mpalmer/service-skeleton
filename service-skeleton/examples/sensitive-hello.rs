#![allow(unused_crate_dependencies)]

use service_skeleton::{metric::counter, service, ServiceConfig};

use std::{env, thread::sleep, time::Duration};

#[derive(Clone, Debug, ServiceConfig)]
struct Config {
	#[config(default_value = "World")]
	name: String,

	#[config(sensitive)]
	password: String,
}

fn main() {
	service("Hello")
		.counter::<Vec<(&str, String)>>("count", "Number of times we've said hello")
		.run(|cfg| say_hello(cfg));
}

fn say_hello(cfg: Config) {
	println!("Hello, {}!", cfg.name);
	println!("(The secret password is {})", cfg.password);
	counter("count", &vec![("name", cfg.name)], |c| {
		c.inc();
	});
	println!(
		"But you won't find it in the environment: HELLO_PASSWORD={:?}",
		env::var("HELLO_PASSWORD")
	);
	sleep(Duration::from_secs(5));
}