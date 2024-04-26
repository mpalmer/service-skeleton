use std::fmt::{Debug, Display};

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
	env_value: Option<&String>,
	default: Option<&'static str>,
) -> Result<RT, Error> {
	let value_to_parse: &str = match (env_value, default) {
		(None, Some(default_value)) => Ok(default_value),
		(Some(value), _) => Ok(value.as_str()),
		(None, None) => Err(Error::no_config_value(var)),
	}?;

	parser(value_to_parse).map_err(|e| Error::config_value_parse(var, value_to_parse, e))
}

pub fn determine_optional_value<RT: Debug + Sync + Send, E: Display>(
	var: &str,
	parser: impl Fn(&str) -> Result<RT, E>,
	env_value: Option<&String>,
	default: Option<&'static str>,
) -> Result<Option<RT>, Error> {
	let value_to_parse: &str = match (env_value, default) {
		(None, Some(default_value)) => Ok(default_value),
		(Some(value), _) => Ok(value.as_str()),
		(None, None) => return Ok(None),
	}?;

	parser(value_to_parse)
		.map_err(|e| Error::config_value_parse(var, value_to_parse, e))
		.map(|v| Some(v))
}
