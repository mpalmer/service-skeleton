use secrecy::Secret;
use std::{
	collections::HashMap,
	fmt::{Debug, Display},
};

use crate::Error;

pub trait Service {
	fn from_env_vars(
		prefix: &str,
		vars: impl Iterator<Item = (String, String)>,
	) -> Result<Self, Error>
	where
		Self: Sized;
}

impl Service for () {
	fn from_env_vars(
		_prefix: &str,
		_vars: impl Iterator<Item = (String, String)>,
	) -> Result<Self, Error> {
		Ok(())
	}
}

pub fn determine_value<RT: Debug + Sync + Send, E: Display>(
	var: &str,
	parser: impl Fn(&str) -> Result<RT, E>,
	env_value: Option<&str>,
	default: Option<&'static str>,
) -> Result<RT, Error> {
	let value_to_parse: &str = match (env_value, default) {
		(None, Some(default_value)) => Ok(default_value),
		(Some(value), _) => Ok(value),
		(None, None) => Err(Error::no_config_value(var)),
	}?;

	parser(value_to_parse).map_err(|e| Error::config_value_parse(var, e))
}

pub fn determine_optional_value<RT: Debug + Sync + Send, E: Display>(
	var: &str,
	parser: impl Fn(&str) -> Result<RT, E>,
	env_value: Option<&str>,
	default: Option<&'static str>,
) -> Result<Option<RT>, Error> {
	let value_to_parse: &str = match (env_value, default) {
		(None, Some(default_value)) => Ok::<&str, Error>(default_value),
		(Some(value), _) => Ok::<&str, Error>(value),
		(None, None) => return Ok(None),
	}?;

	parser(value_to_parse)
		.map_err(|e| Error::config_value_parse(var, e))
		.map(|v| Some(v))
}

pub fn fetch_encrypted_field<H1: ::std::hash::BuildHasher, H2: ::std::hash::BuildHasher>(
	var_map: &HashMap<String, String, H1>,
	key_map: &mut HashMap<Key, Secret<String>, H2>,
	value_field_var: &str,
	key_spec: &Key,
) -> Result<Option<String>, Error> {
	let Some(value) = var_map.get(value_field_var) else {
		return Ok(None);
	};

	let key = if let Some(k) = key_map.get(key_spec) {
		k
	} else {
		let k = match key_spec {
			Key::File(ref file_env) => {
				let Some(key_file) = var_map.get(file_env) else {
					return Err(Error::no_config_value(file_env));
				};
				std::fs::read_to_string(key_file)
					.map_err(|e| Error::key_read(key_file, e))?
					.trim_end()
					.to_string()
			}
		};
		key_map.insert(key_spec.clone(), Secret::new(k));
		// How much I wish there was an Entry method that allowed fallible closures...
		#[allow(clippy::unwrap_used)]
		key_map.get(key_spec).unwrap()
	};

	Ok(Some(sscrypt::decrypt(value, value_field_var, key)?))
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
#[non_exhaustive]
// This enum is not meant to be used directly; it is an implementation detail that must be made
// public because it is used in derived code
#[doc(hidden)]
pub enum Key {
	File(String),
}
