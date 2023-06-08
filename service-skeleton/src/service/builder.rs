use heck::{AsShoutySnekCase, AsSnekCase};
use prometheus_client::{encoding::EncodeLabelSet, metrics::histogram::Histogram};

use std::{
    env::{self, VarError, vars as env_vars},
    fmt::{Debug, Display},
	hash::Hash,
    panic::{catch_unwind, UnwindSafe},
    process::exit,
};

use crate::{ServiceConfig, metric::{Collection as MetricCollection, set_metrics_collection, start_metrics_server}};


#[must_use]
pub fn build() -> Builder {
	Builder { metrics: MetricCollection::default() }
}
#[derive(Debug)]
pub struct Builder {
	metrics: MetricCollection,
}

impl Builder {
	pub fn counter<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static>(mut self, name: impl AsRef<str>, description: impl AsRef<str>) -> Self {
		self.metrics.register_counter::<L>(name, description, None);
		self
	}

	pub fn gauge<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static>(mut self, name: impl AsRef<str>, description: impl AsRef<str>) -> Self {
		self.metrics.register_gauge::<L>(name, description, None);
		self
	}

	pub fn histogram<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static, C: Clone + Fn() -> Histogram + Send + Sync + 'static>(mut self, name: impl AsRef<str>, description: impl AsRef<str>, constructor: C) -> Self {
		self.metrics.register_histogram::<L, C>(name, description, None, constructor);
		self
	}

	pub fn run<CFG>(self, name: impl Display + AsRef<str>, svc: impl FnOnce(CFG) + Clone + UnwindSafe) -> !
	where
		CFG: ServiceConfig + Clone + Debug + Sync + Send + UnwindSafe,
	{
		#[allow(clippy::expect_used)] // If this fails to start, we're in big trouble
		flexi_logger::Logger::try_with_env_or_str("info")
			.expect("logger configuration to be valid")
			.adaptive_format_for_stderr(flexi_logger::AdaptiveFormat::WithThread)
			.start()
			.expect("logger to start");

		let env_prefix = AsShoutySnekCase(&name).to_string();
		let metrics_prefix = AsSnekCase(&name).to_string();

		let metrics_port_env_var = format!("{env_prefix}_METRICS_SERVER_PORT");

		set_metrics_collection(&self.metrics);

		match env::var(&metrics_port_env_var) {
			Ok(val) => match val.parse::<u16>() {
				Ok(port) => if let Err(e) = start_metrics_server(&metrics_prefix, port, self.metrics) {
					log::warn!("Metrics server failed to start: {e}");
				},
				Err(e) => log::warn!("Not starting metrics server: could not parse {val} (from {metrics_port_env_var}) as port number: {e}"),
			},
			Err(VarError::NotUnicode(_)) => log::warn!("Not starting metrics server: value of {metrics_port_env_var} is not valid unicode"),
			Err(VarError::NotPresent) => log::debug!("Not starting metrics server: {metrics_port_env_var} is not set"),
		};

		let cfg =
			CFG::from_env_vars(&env_prefix, env_vars()).unwrap_or_else(|e| {
				log::error!("Failed to configure {name}: {e}");
				#[allow(clippy::exit)] // nothing else useful going to be going on after this
				exit(1);
			});

		log::debug!("Using config: {cfg:?}");

		loop {
			let svc_fn = svc.clone();
			let svc_cfg = cfg.clone();
			if let Err(e_ref) = catch_unwind(move || svc_fn(svc_cfg)) {
				if let Some(e) = e_ref.downcast_ref::<String>() {
					log::warn!("service {name} panicked: {e}");
				} else if let Some(e) = e_ref.downcast_ref::<&str>() {
					log::warn!("service {name} panicked: {e}");
				} else {
					log::warn!("service {name} panicked with non-string payload");
				}
			}
		}
	}
}
