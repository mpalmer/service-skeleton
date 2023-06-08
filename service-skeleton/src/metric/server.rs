use prometheus_client::{
	encoding::{text, EncodeLabelSet},
	metrics::{counter::Counter, family::Family},
	registry::Registry,
};
use tiny_http::{Method, Request, Response, Server};

use std::{fmt::Debug, thread};

use crate::Error;

pub(crate) fn start_metrics_server(port: u16, mut registry: Registry) -> Result<(), Error> {
	let server =
		Server::http(format!("[::]:{port}")).map_err(|e| Error::metrics_server_start(port, e))?;
	let req_count = Family::<ReqLabels, Counter>::default();
	registry.register(
		"http_requests",
		"Number of requests to the metrics server",
		req_count.clone(),
	);

	thread::Builder::new()
		.name("MetricsServer".to_string())
		.spawn(move || {
			log::info!("Metrics server listening on [::]:{port}");

			loop {
				let request = match server.recv() {
					Ok(req) => req,
					Err(e) => {
						log::error!("Error while receiving metrics server request: {e}");
						break;
					}
				};

				#[allow(clippy::wildcard_enum_match_arm)] // Yes, that's the kinda the point
				match request.method() {
					Method::Get => {
						if request.url() == "/metrics" {
							let mut buf = String::new();
							if let Err(e) = text::encode(&mut buf, &registry) {
								log::warn!("Failed to encode metrics: {e}");
								send_response(request, Response::empty(500u16), &req_count);
							} else {
								send_response(request, Response::from_string(buf), &req_count);
							}
						} else {
							send_response(request, Response::empty(404u16), &req_count);
						}
					}
					_ => {
						send_response(request, Response::empty(405u16), &req_count);
					}
				}
			}
		})
		.map_err(|e| Error::metrics_server_start(port, Box::new(e)))?;

	Ok(())
}

fn send_response<R: std::io::Read>(
	request: Request,
	response: Response<R>,
	counter: &Family<ReqLabels, Counter>,
) {
	let mut req_labels = ReqLabels {
		method: request.method().as_str().to_string(),
		path: request.url().to_string(),
		status: *response.status_code().as_ref(),
	};

	if let Err(e) = request.respond(response) {
		log::warn!("Failed to send metrics response: {e}");
		req_labels.status = 666;
	}

	counter.get_or_create(&req_labels).inc();
}

#[derive(Clone, Debug, EncodeLabelSet, Eq, Hash, PartialEq)]
struct ReqLabels {
	method: String,
	path: String,
	status: u16,
}
