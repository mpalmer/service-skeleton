#![cfg_attr(doctest, doc = include_str!("../../README.md"))]
#[cfg(doctest)]
pub struct ReadmeDoctests;

#[doc(hidden)]
pub mod config;
pub use config::Service as ServiceConfig;

mod error;
pub use error::Error;

pub mod metric;

mod service;
pub use service::{service, Service};

#[doc(hidden)]
pub use heck;

pub use service_skeleton_derive::ServiceConfig;
