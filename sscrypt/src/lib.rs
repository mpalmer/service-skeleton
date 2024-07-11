use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use secrecy::{ExposeSecret, Secret};
use strong_box::{SharedStrongBox, SharedStrongBoxKey, StrongBox};

mod error;
pub use error::Error;

const PUBLIC_KEY_PREFIX: &str = "sspuk:";
const PRIVATE_KEY_PREFIX: &str = "ssprk:";

pub fn make_key() -> Result<(PrvKey, PubKey), Error> {
	let key = SharedStrongBox::generate_key();

	Ok((
		PrvKey(format!(
			"{}{}",
			PRIVATE_KEY_PREFIX,
			BASE64_URL_SAFE_NO_PAD.encode(
				key.private()
					.ok_or_else(|| Error::invalid_key(
						"SHOULDN'T HAPPEN: new key does not have privates"
					))?
					.expose_secret()
			)
		)),
		PubKey(format!(
			"{}{}",
			PUBLIC_KEY_PREFIX,
			BASE64_URL_SAFE_NO_PAD.encode(key.public())
		)),
	))
}

pub fn encrypt(plaintext: &str, ctx: &str, key: &str) -> Result<String, Error> {
	let keydata = key
		.strip_prefix(PUBLIC_KEY_PREFIX)
		.ok_or_else(|| Error::invalid_key("incorrect prefix"))?;
	#[allow(clippy::shadow_unrelated)] // Au contraire, monsieur Clippy...
	let key = SharedStrongBoxKey::try_from(&BASE64_URL_SAFE_NO_PAD.decode(keydata)?)?;

	let strong_box = SharedStrongBox::new(key);

	Ok(BASE64_URL_SAFE_NO_PAD.encode(strong_box.encrypt(plaintext.as_bytes(), ctx.as_bytes())?))
}

pub fn decrypt(ciphertext: &str, ctx: &str, key: &Secret<String>) -> Result<String, Error> {
	let ciphertext = BASE64_URL_SAFE_NO_PAD.decode(ciphertext)?;
	let keydata = key
		.expose_secret()
		.strip_prefix(PRIVATE_KEY_PREFIX)
		.ok_or_else(|| Error::invalid_key("incorrect prefix"))?;
	#[allow(clippy::shadow_unrelated)] // Au contraire, monsieur Clippy...
	let key = SharedStrongBoxKey::try_from(&BASE64_URL_SAFE_NO_PAD.decode(keydata)?)?;

	let strong_box = SharedStrongBox::new(key);

	Ok(String::from_utf8(
		strong_box.decrypt(ciphertext, ctx.as_bytes())?,
	)?)
}

// These are purely "safety" types, to make sure that the tuple of key strings we pass back from
// make_key() don't get mixed up and result in Disaster
#[allow(missing_debug_implementations)]
pub struct PrvKey(String);
#[derive(Debug)]
pub struct PubKey(String);

impl std::ops::Deref for PrvKey {
	type Target = String;

	fn deref(&self) -> &String {
		&self.0
	}
}

impl std::ops::Deref for PubKey {
	type Target = String;

	fn deref(&self) -> &String {
		&self.0
	}
}
