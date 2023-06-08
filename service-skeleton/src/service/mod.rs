//! The heart of the machine.
//!

use heck::{AsShoutySnekCase, AsSnekCase};
use prometheus_client::{
	encoding::EncodeLabelSet,
	metrics::{counter::Counter, family::Family, gauge::Gauge, histogram::Histogram},
	registry::{Metric, Registry},
};

use std::{
	env::{self, vars as env_vars, VarError},
	fmt::Debug,
	hash::Hash,
	panic::{catch_unwind, UnwindSafe},
	process::exit,
};

use crate::{
	metric::{start_metrics_server, store_metric},
	ServiceConfig,
};

/// Create a new service skeleton.
///
/// Using this new skeleton, you can register metrics, and then start the service going with `run`.
///
#[must_use]
pub fn service(name: impl AsRef<str>) -> Service {
	Service {
		name: name.as_ref().to_string(),
		registry: Registry::default(),
	}
}

#[derive(Debug)]
pub struct Service {
	name: String,
	registry: Registry,
}

impl Service {
	pub fn counter<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static>(
		self,
		name: impl AsRef<str>,
		description: impl AsRef<str>,
	) -> Self {
		self.add_metric(name, description, Family::<L, Counter>::default())
	}

	pub fn gauge<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static>(
		self,
		name: impl AsRef<str>,
		description: impl AsRef<str>,
	) -> Self {
		self.add_metric(name, description, Family::<L, Gauge>::default())
	}

	pub fn histogram<
		L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static,
		C: Clone + Fn() -> Histogram + Send + Sync + 'static,
	>(
		self,
		name: impl AsRef<str>,
		description: impl AsRef<str>,
		constructor: C,
	) -> Self {
		self.add_metric(
			name,
			description,
			Family::<L, Histogram, C>::new_with_constructor(constructor),
		)
	}

	fn add_metric(
		mut self,
		name: impl AsRef<str>,
		description: impl AsRef<str>,
		family: impl Metric + Clone,
	) -> Self {
		store_metric(&name, family.clone());
		self.registry.register(
			format!("{}_{}", AsSnekCase(&self.name), name.as_ref()),
			description.as_ref(),
			family,
		);
		self
	}

	/// Run the service under suitable supervision.
	///
	/// The `name` given influences the names of metrics and the environment variables that will be
	/// examined to determine the service configuration, while the `svc` function is what you provide
	/// as the entrypoint to the service to be run.  If that function exits (which it shouldn't), or
	/// panics (which it definitely shouldn't, but might), it will be restarted.
	///
	///
	/// # Panics
	///
	/// As this function doesn't normally exit, it will panic if any fatal error occurs, such as if the
	/// logger cannot be started, or if the service configuration cannot be correctly extracted from
	/// the environment.
	///
	pub fn run<CFG>(self, svc: impl FnOnce(CFG) + Clone + UnwindSafe) -> !
	where
		CFG: ServiceConfig + Clone + Debug + Sync + Send + UnwindSafe,
	{
		#[allow(clippy::expect_used)] // If this fails to start, we're in big trouble
		flexi_logger::Logger::try_with_env_or_str("info")
			.expect("logger configuration to be valid")
			.adaptive_format_for_stderr(flexi_logger::AdaptiveFormat::WithThread)
			.start()
			.expect("logger to start");

		let env_prefix = AsShoutySnekCase(&self.name).to_string();

		let metrics_port_env_var = format!("{env_prefix}_METRICS_SERVER_PORT");

		match env::var(&metrics_port_env_var) {
			Ok(val) => match val.parse::<u16>() {
				Ok(port) => if let Err(e) = start_metrics_server(port, self.registry) {
					log::warn!("Metrics server failed to start: {e}");
				},
				Err(e) => log::warn!("Not starting metrics server: could not parse {val} (from {metrics_port_env_var}) as port number: {e}"),
			},
			Err(VarError::NotUnicode(_)) => log::warn!("Not starting metrics server: value of {metrics_port_env_var} is not valid unicode"),
			Err(VarError::NotPresent) => log::debug!("Not starting metrics server: {metrics_port_env_var} is not set"),
		};

		let cfg = CFG::from_env_vars(&env_prefix, env_vars()).unwrap_or_else(|e| {
			log::error!("Failed to configure {}: {e}", self.name);
			#[allow(clippy::exit)] // nothing else useful going to be going on after this
			exit(1);
		});

		log::debug!("Using config: {cfg:?}");

		loop {
			let svc_fn = svc.clone();
			let svc_cfg = cfg.clone();
			if let Err(e_ref) = catch_unwind(move || svc_fn(svc_cfg)) {
				if let Some(e) = e_ref.downcast_ref::<String>() {
					log::warn!("service {} panicked: {e}", self.name);
				} else if let Some(e) = e_ref.downcast_ref::<&str>() {
					log::warn!("service {} panicked: {e}", self.name);
				} else {
					log::warn!("service {} panicked with non-string payload", self.name);
				}
			}
		}
	}
}
