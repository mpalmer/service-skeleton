#[allow(unused_crate_dependencies)]
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
