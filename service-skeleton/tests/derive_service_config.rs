use service_skeleton::ServiceConfig;

#[test]
fn test_default_parse() {
	#[derive(Debug, ServiceConfig)]
	struct OptionConfig {
		value: Option<u64>,
	}

	assert_eq!(
		None,
		OptionConfig::from_env_vars("FOO", vec![].into_iter())
			.unwrap()
			.value
	);
}

#[test]
fn test_parse_secrets() {
	use secrecy::{ExposeSecret, Secret};

	#[derive(Debug, ServiceConfig)]
	struct SecretConfig {
		value: Secret<String>,
	}

	// this is the sort of thing that gets you kicked out of the nicer establishments
	std::env::set_var("FOO_VALUE", "s3kr1t");

	assert_eq!(
		"s3kr1t",
		SecretConfig::from_env_vars(
			"FOO",
			vec![("FOO_VALUE".to_string(), "s3kr1t".to_string())].into_iter()
		)
		.unwrap()
		.value
		.expose_secret()
	);

	assert_eq!(
		Err(std::env::VarError::NotPresent),
		std::env::var("FOO_VALUE")
	);
}

#[test]
fn test_encrypted_config() {
	use std::net::IpAddr;

	#[derive(Debug, ServiceConfig)]
	struct SecretConfig {
		name: String,

		#[config(encrypted, key_file_field = "da_key")]
		secret_string: String,

		#[config(encrypted, key_file_field = "da_key")]
		maybe_secret: Option<String>,

		#[config(encrypted, key_file_field = "da_key")]
		secret_address: IpAddr, // It was the first thing I could think of that impl'd FromStr
	}

	let cfg = SecretConfig::from_env_vars(
		"FOO",
		vec![
			("FOO_NAME", "Jaime"),
			("FOO_SECRET_STRING", "ssb1glggNkkrqpr3IZF-5bpSkhD0TvhEKmuHS0R2a-COwlRF8zxYObG49YNQecjHPEHbwxHPhzkiuZ0-KEzH8yqr-tFEmHCuouxW7x0INpNCeI91FE6AeNUyoPIuRpk8Iw"),
			("FOO_SECRET_ADDRESS", "ssb1glggLFZmujR858TBxh3y_3o_uOo4v4q3nEdKzJ4h0Kgma1RYPrG49YNQ5Qd1hgtrbVLeUUFfZ4B9IkzXEnXMbeMFZTIwmVRYGgVF81Ur1rYMcBBx58DH6snP-Cpk25EGsLHT"),
			// Tests run in the crate's root, not the workspace root
			("FOO_DA_KEY", "./tests/test_encrypted_config.key"),
		].into_iter().map(|(k, v)| (k.to_string(), v.to_string()))
	).unwrap();

	assert_eq!("Jaime", cfg.name);
	assert_eq!("s3kr1t", cfg.secret_string);
	assert_eq!(None, cfg.maybe_secret);
	assert_eq!("192.0.2.42".parse::<IpAddr>().unwrap(), cfg.secret_address);
}
