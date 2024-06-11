#[derive(Debug, thiserror::Error, thiserror_ext::Construct)]
#[non_exhaustive]
pub enum Error {
	#[error("failed to decode base64 data: {0}")]
	Base64DecodingFailed(#[from] base64::DecodeError),

	#[error("cryptographic failure: {0}")]
	Cryptgraphy(#[from] strong_box::Error),

	#[error("invalid key: {0}")]
	InvalidKey(String),

	#[error("invalid string: {0}")]
	InvalidString(#[from] std::string::FromUtf8Error),
}
