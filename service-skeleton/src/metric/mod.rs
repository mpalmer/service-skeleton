use parking_lot::MappedRwLockReadGuard;
use prometheus_client::metrics::{
	counter::Counter,
	family::{Family, MetricConstructor},
	gauge::Gauge,
	histogram::Histogram,
};
use std::sync::OnceLock;

// Re-export so services don't have to depend on prometheus-client directly,
// which can get version-compatible-ugly real quick
pub use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
/// Re-exports that are needed to derive any of the `EncodeLabel*` traits
pub mod encode_labels {
	pub use prometheus_client::{self, encoding::EncodeLabelSet, encoding::EncodeLabelValue};
}

use std::{
	any::{type_name, Any},
	collections::HashMap,
	hash::Hash,
	sync::{Arc, Mutex},
};

mod server;
pub(crate) use server::start_metrics_server;

#[derive(Clone, Debug, Default)]
pub(crate) struct Histogrammer {
	buckets: Vec<f64>,
}

impl Histogrammer {
	pub(crate) fn new(buckets: &[f64]) -> Self {
		Histogrammer {
			buckets: buckets.to_vec(),
		}
	}
}

impl MetricConstructor<Histogram> for Histogrammer {
	fn new_metric(&self) -> Histogram {
		Histogram::new(self.buckets.clone().into_iter())
	}
}

fn metrics() -> &'static Mutex<HashMap<String, Arc<dyn Any + Send + Sync + 'static>>> {
	static METRICS: OnceLock<Mutex<HashMap<String, Arc<dyn Any + Send + Sync + 'static>>>> =
		OnceLock::new();
	METRICS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn store_metric(name: impl AsRef<str>, metric: impl Any + Send + Sync + 'static) {
	#[allow(clippy::expect_used)] // If this explodes, we're all in a world of hurt
	let mut metrics = metrics().lock().expect("METRICS mutex to not be poisoned");

	metrics.insert(name.as_ref().to_string(), Arc::new(metric));
}

pub fn counter<L>(name: impl AsRef<str>, labels: &L, f: impl Fn(MappedRwLockReadGuard<'_, Counter>))
where
	L: Clone + Eq + Send + Sync + Hash + 'static,
{
	metric::<L, Counter, fn() -> Counter>(name, labels, f);
}

pub fn gauge<L>(name: impl AsRef<str>, labels: &L, f: impl Fn(MappedRwLockReadGuard<'_, Gauge>))
where
	L: Clone + Eq + Send + Sync + Hash + 'static,
{
	metric::<L, Gauge, fn() -> Gauge>(name, labels, f);
}

pub fn histogram<L>(
	name: impl AsRef<str>,
	labels: &L,
	f: impl Fn(MappedRwLockReadGuard<'_, Histogram>),
) where
	L: Clone + Eq + Send + Sync + Hash + 'static,
{
	metric::<L, Histogram, Histogrammer>(name, labels, f);
}

fn metric<L, M, C>(name: impl AsRef<str>, labels: &L, f: impl Fn(MappedRwLockReadGuard<'_, M>))
where
	L: Clone + Eq + Send + Sync + Hash + 'static,
	M: Send + Sync + 'static,
	C: MetricConstructor<M> + 'static,
{
	#[allow(clippy::expect_used)] // If this explodes, we're all in a world of hurt
	let m = metrics().lock().expect("METRICS mutex to not be poisoned");

	if let Some(any_family) = m.get(name.as_ref()) {
		if let Some(family) = any_family.downcast_ref::<Family<L, M, C>>() {
			let time_series = family.get_or_create(labels);
			f(time_series);
		} else {
			tracing::warn!(
				"Could not find {} metric {} with labels {}",
				name.as_ref(),
				type_name::<M>(),
				type_name::<L>(),
			);
		};
	} else {
		tracing::warn!("No metric named {}", name.as_ref());
	}
}
