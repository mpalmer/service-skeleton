use service_skeleton::ServiceConfig;

#[derive(Debug, ServiceConfig)]
struct OptionConfig {
	value: Option<u64>,
}

#[test]
fn test_default_parse() {
	assert_eq!(
		None,
		OptionConfig::from_env_vars("FOO", vec![].into_iter())
			.unwrap()
			.value
	);
}

use {
	flexi_logger as _, heck as _, lazy_static as _, log as _, parking_lot as _,
	prometheus_client as _, service_skeleton_derive as _, thiserror as _, tiny_http as _,
};
