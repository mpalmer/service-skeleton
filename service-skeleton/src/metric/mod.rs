use lazy_static::lazy_static;
use parking_lot::MappedRwLockReadGuard;
use prometheus_client::metrics::{
	counter::Counter, family::Family, gauge::Gauge, histogram::Histogram,
};

use std::{
	any::{type_name, Any},
	collections::HashMap,
	hash::Hash,
	sync::{Arc, Mutex},
};

mod server;
pub(crate) use server::start_metrics_server;

lazy_static! {
	static ref METRICS: Arc<Mutex<HashMap<String, Arc<dyn Any + Send + Sync + 'static>>>> =
		Arc::new(Mutex::new(HashMap::default()));
}

pub(crate) fn store_metric(name: impl AsRef<str>, metric: impl Any + Send + Sync + 'static) {
	#[allow(clippy::expect_used)] // If this explodes, we're all in a world of hurt
	let mut metrics = METRICS.lock().expect("METRICS mutex to not be poisoned");

	metrics.insert(name.as_ref().to_string(), Arc::new(metric));
}

pub fn counter<L>(name: impl AsRef<str>, labels: &L, f: impl Fn(MappedRwLockReadGuard<'_, Counter>))
where
	L: Clone + Eq + Send + Sync + Hash + 'static,
{
	metric::<L, Counter>(name, labels, f);
}

pub fn gauge<L>(name: impl AsRef<str>, labels: &L, f: impl Fn(MappedRwLockReadGuard<'_, Gauge>))
where
	L: Clone + Eq + Send + Sync + Hash + 'static,
{
	metric::<L, Gauge>(name, labels, f);
}

pub fn histogram<L>(
	name: impl AsRef<str>,
	labels: &L,
	f: impl Fn(MappedRwLockReadGuard<'_, Histogram>),
) where
	L: Clone + Eq + Send + Sync + Hash + 'static,
{
	metric::<L, Histogram>(name, labels, f);
}

fn metric<L, M>(name: impl AsRef<str>, labels: &L, f: impl Fn(MappedRwLockReadGuard<'_, M>))
where
	L: Clone + Eq + Send + Sync + Hash + 'static,
	M: Send + Sync + 'static,
{
	#[allow(clippy::expect_used)] // If this explodes, we're all in a world of hurt
	let m = METRICS.lock().expect("METRICS mutex to not be poisoned");

	if let Some(any_family) = m.get(name.as_ref()) {
		if let Some(family) = any_family.downcast_ref::<Family<L, M>>() {
			let time_series = family.get_or_create(labels);
			f(time_series);
		} else {
			log::warn!(
				"Could not find {} metric {} with labels {}",
				name.as_ref(),
				type_name::<M>(),
				type_name::<L>(),
			);
		};
	} else {
		log::warn!("No metric named {}", name.as_ref());
	}
}
