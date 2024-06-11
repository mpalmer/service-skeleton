#![allow(clippy::expect_used, clippy::print_stdout, clippy::panic)] // It's a CLI, we'll live
use clap::Parser;

use sscrypt::{make_key, PrvKey, PubKey};

fn main() {
	let cli = Cli::parse();

	cli.command.execute();
}

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
	Init { name: String },
	Encrypt { field: String, name: String },
}

impl Command {
	fn execute(self) {
		match self {
			Self::Init { name } => Self::do_init(&name),
			Self::Encrypt { field, name } => Self::do_encrypt(&field, &name),
		}
	}

	fn do_init(name: &str) {
		let (prvkey, pubkey): (PrvKey, PubKey) = make_key().expect("failed to create new key");

		println!("Private key: {}", &*prvkey);
		std::fs::write(format!("{name}.key"), &*pubkey)
			.unwrap_or_else(|e| panic!("failed to write public key to {name}.key: {e}"));
		println!("Public key written to {name}.key");
	}

	fn do_encrypt(field: &str, name: &str) {
		use std::io::Write as _;

		print!("Enter secret to be encrypted: ");
		std::io::stdout().flush().expect("flush failed");
		let mut line = String::new();
		std::io::stdin().read_line(&mut line).expect("read failed");
		let key = std::fs::read_to_string(format!("{name}.key"))
			.unwrap_or_else(|e| panic!("failed to read public key from {name}.key: {e}"));
		println!(
			"Encrypted secret: {}",
			sscrypt::encrypt(line.trim_end(), field, &key).expect("encryption failed")
		);
	}
}
