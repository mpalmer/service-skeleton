//! The heart of the machine.
//!

use heck::{AsShoutySnekCase, AsSnekCase};
use prometheus_client::{
	encoding::EncodeLabelSet,
	metrics::{counter::Counter, family::Family, gauge::Gauge, histogram::Histogram},
	registry::{Metric, Registry},
};
use tracing_subscriber::layer::SubscriberExt as _;

use std::{
	env::{self, vars as env_vars, VarError},
	fmt::Debug,
	hash::Hash,
	panic::{catch_unwind, UnwindSafe},
	process::exit,
};

use crate::{
	metric::{start_metrics_server, store_metric, Histogrammer},
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
	#[must_use]
	pub fn counter<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static>(
		self,
		name: impl AsRef<str>,
		description: impl AsRef<str>,
	) -> Self {
		self.add_metric(name, description, Family::<L, Counter>::default())
	}

	#[must_use]
	pub fn gauge<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static>(
		self,
		name: impl AsRef<str>,
		description: impl AsRef<str>,
	) -> Self {
		self.add_metric(name, description, Family::<L, Gauge>::default())
	}

	#[must_use]
	pub fn histogram<L: Clone + Debug + EncodeLabelSet + Eq + Hash + Send + Sync + 'static>(
		self,
		name: impl AsRef<str>,
		description: impl AsRef<str>,
		buckets: &[f64],
	) -> Self {
		self.add_metric(
			name,
			description,
			Family::<L, Histogram, Histogrammer>::new_with_constructor(Histogrammer::new(buckets)),
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
		let layer = tracing_tree::HierarchicalLayer::default()
			.with_writer(tracing_subscriber::fmt::TestWriter::new())
			.with_indent_lines(true)
			.with_indent_amount(2)
			.with_targets(true);

		let sub = tracing_subscriber::registry::Registry::default()
			.with(layer)
			.with(tracing_subscriber::EnvFilter::from_default_env());
		#[allow(clippy::expect_used)] // If this fails to start, we're in big trouble
		tracing::subscriber::set_global_default(sub).expect("tracing subscriber failed to start");
		if let Err(e) = tracing_log::LogTracer::init() {
			tracing::warn!("Failed to initialize LogTracer: {e}");
		}

		let env_prefix = AsShoutySnekCase(&self.name).to_string();

		let metrics_port_env_var = format!("{env_prefix}_METRICS_SERVER_PORT");

		match env::var(&metrics_port_env_var) {
			Ok(val) => match val.parse::<u16>() {
				Ok(port) => if let Err(e) = start_metrics_server(port, self.registry) {
					tracing::warn!("Metrics server failed to start: {e}");
				},
				Err(e) => tracing::warn!("Not starting metrics server: could not parse {val} (from {metrics_port_env_var}) as port number: {e}"),
			},
			Err(VarError::NotUnicode(_)) => tracing::warn!("Not starting metrics server: value of {metrics_port_env_var} is not valid unicode"),
			Err(VarError::NotPresent) => tracing::info!("Not starting metrics server: {metrics_port_env_var} is not set"),
		}

		let cfg = CFG::from_env_vars(&env_prefix, env_vars()).unwrap_or_else(|e| {
			tracing::error!("Failed to configure {}: {e}", self.name);
			#[allow(clippy::exit)] // nothing else useful going to be going on after this
			exit(1);
		});

		tracing::debug!("Using config: {cfg:?}");

		loop {
			let svc_fn = svc.clone();
			let svc_cfg = cfg.clone();
			if let Err(e_ref) = catch_unwind(move || svc_fn(svc_cfg)) {
				if let Some(e) = e_ref.downcast_ref::<String>() {
					tracing::warn!("service {} panicked: {e}", self.name);
				} else if let Some(e) = e_ref.downcast_ref::<&str>() {
					tracing::warn!("service {} panicked: {e}", self.name);
				} else {
					tracing::warn!("service {} panicked with non-string payload", self.name);
				}
			}
		}
	}
}
