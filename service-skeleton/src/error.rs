use thiserror::Error;

use std::{
	error::Error as StdError,
	fmt::{Debug, Display},
};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
	#[error("no value found for required configuration item {name}")]
	ConfigValueRequired { name: String },

	#[error("failed to parse value {value} for {name}: {cause}")]
	ConfigValueParse {
		value: String,
		name: String,
		cause: String,
	},

	#[error("no metric named {name}")]
	NoSuchMetric { name: String },

	#[error("the metric named {name} is not a {metric_type}, or does not take labels of type {labels_type}")]
	InvalidMetric {
		name: String,
		metric_type: String,
		labels_type: String,
	},

	#[error("could not start metrics server on [::]:{port}: {cause}")]
	MetricsServerStart {
		port: u16,
		cause: Box<dyn StdError + Send + Sync + 'static>,
	},
}

impl Error {
	#[must_use]
	pub fn no_config_value(name: &str) -> Error {
		Error::ConfigValueRequired {
			name: name.to_string(),
		}
	}

	#[must_use]
	pub fn config_value_parse(name: &str, value: impl Debug, cause: impl Display) -> Error {
		Error::ConfigValueParse {
			name: name.to_string(),
			value: format!("{value:?}"),
			cause: cause.to_string(),
		}
	}

	#[must_use]
	pub fn no_metric(name: &str) -> Error {
		Error::NoSuchMetric {
			name: name.to_string(),
		}
	}

	#[must_use]
	pub fn invalid_metric(name: &str, metric_type: &str, labels_type: &str) -> Error {
		Error::InvalidMetric {
			name: name.to_string(),
			metric_type: metric_type.to_string(),
			labels_type: labels_type.to_string(),
		}
	}

	#[must_use]
	pub fn metrics_server_start(
		port: u16,
		cause: Box<dyn StdError + Send + Sync + 'static>,
	) -> Error {
		Error::MetricsServerStart { port, cause }
	}
}
