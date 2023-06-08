The `service-skeleton` is a Rust crate that provides the "bare bones" of a program which is intended to run as a service -- a long-lived, non-(directly)-human-interactive program that typically provides some sort of functionality to a larger system.
It provides:

* Initialisation and configuration of logging (partially implemented);
* Configuration parsing and management, via environment variables;
* Supervision of subunits of functionality, automatically restarting them if they crash (partially impemented);
* A built-in Prometheus-compatible (OpenMetrics) metrics server and hooks for easily declaring and using metrics;

Features which are intended to be added in the future, but don't exist yet:

* Signal handling, including built-in support for dynamic log-level adjustment, backtrace dumping, and graceful shutdown;
* OpenTracing support;
* HTTP-based introspection and control.

The general philosophy of `service-skeleton` is to be secure-by-default, provide features that have been found near-universally useful for service programs in modern deployment scenarios, and to prefer convention over configuration.


# Installation

It's published on [`crates.io`](https://crates.io/crates/service-skeleton), so a `cargo add service-skeleton` should work.


# Usage

In its simplest form, which enables most of the available features, you can make your `main` function look pretty much exactly like this:

```rust
# use std::time::Duration;
# // Yes, this is cheating
# fn sleep(_: Duration) { std::process::exit(0) }
use service_skeleton::service;

fn main() {
    service("SayHello").run(|_cfg: ()| say_hello());
}

fn say_hello() {
    println!("Hello world!");
    sleep(Duration::from_secs(5));
}
```

This example will cause the program to print "Hello world!" every five seconds to your terminal, until you stop it with Ctrl-C (or some other way, like a `kill -9`).
Not the most exciting of services, but it does demonstrate one of `service-skeleton`'s basic functions: *service supervision*.

The closure that you provide to [`service_skeleton::Service::run`](https://docs.rs/service-skeleton/latest/service_skeleton/struct.Service.html#method.run) shouldn't ordinarily terminate -- the idea is that it'll live more-or-less forever, servicing whatever requests come its way.
However, if the closure does terminate for any reason (whether via panic or otherwise) the closure will be run again, and the fact of the restart will be logged.
Which is a nice segue into the next feature...


## Logging

One of the things that `service-skeleton` configures for you is logging, using the [`log` crate's](https://crates.io/crates/log) well-established facade.
By default, all log messages with severity `warn` or higher will be printed to `stderr` with a bunch of related useful information.
Again, you don't have to do anything special, just start logging away:

```rust
# use std::time::Duration;
# fn sleep(_: Duration) { std::process::exit(0) }
use service_skeleton::service;

fn main() {
    service("LogHello").run(|_cfg: ()| say_hello());
}

fn say_hello() {
    log::info!("Hello, logs!");
    sleep(Duration::from_secs(5));
}
```

This will print out the log message specified every five seconds.
The default logging configuration is that everything at `info` level or above is logged.
If you prefer a different default log level, to log to a file, modify the output format of the log, or set per-module levels, *at the moment* you'll have to rely on [what `RUST_LOG` can do](https://docs.rs/env_logger/latest/env_logger/#enabling-logging), but this will be expanded with various configuration "knobs" as demand requires.

Which is as good a time as any to talk about configuration.


## Configuration

Most services need some sort of configuration.
In line with the [12factor philosophy](https://12factor.net/config), `service-skeleton` encourages you to store your configuration in environment variables.

To declare your configuration, you'll need to declare your configuration struct, like this:

```rust
use service_skeleton::ServiceConfig;

#[derive(Clone, ServiceConfig, Debug)]
struct MyConfig {
    /// The name to say hello to
    #[config(default_value = "World")]
    name: String,
}
```

If you're familiar with [`clap`](https://crates.io/crates/clap), then hopefully the approach taken by `service-skeleton`'s config support will feel comfortable, as it took significant inspiration from `clap`.
See [the `ServiceConfig` docs](https://docs.rs/service-skeleton/latest/service_skeleton/derive.ServiceConfig.html) for details of the available attribute directives.

As you may have noticed from the previous examples, the configuration gets passed into the closure provided to `run`, so all you need to do is, in turn, pass that into your `say_hello` function, and you're off and running (as it were):

```rust
# use service_skeleton::ServiceConfig;
# #[derive(Clone, ServiceConfig, Debug)]
# struct MyConfig {
#     #[config(default_value = "World")]
#     name: String,
# }
# use std::time::Duration;
# fn sleep(_: Duration) { std::process::exit(0) }
use service_skeleton::service;

fn main() {
    service("Hello").run(|cfg| say_hello(cfg));
}

fn say_hello(cfg: MyConfig) {
    println!("Hello, {}!", cfg.name);
    sleep(Duration::from_secs(5));
}
```

By default, this will print "Hello, World!" every five seconds.
*However*, you can now configure who to say hello to, by using an environment variable, like this:

```sh
HELLO_NAME=Bobbie cargo run
```

This will now instead print "Hello, Bobbie!" every five seconds.

The environment variable that `service-skeleton` will use to try and read the configuration value from is determined by the name of struct member, prefixed with the name of the service (what was passed to `service`), then turned into all-uppercase.
If the environment variable is missing, the default value will be used (if specified), or the program will exit.
If the value specified cannot be [parsed](https://doc.rust-lang.org/std/primitive.str.html#method.parse) into a value of the struct member's type, the program will log an error and exit.


## Service Metrics

You can't manage what you don't measure.  That's why `service-skeleton` comes with first-class support for [Prometheus](https://prometheus.io) (aka "[OpenMetrics](https://openmetrics.io)") metrics collection and export.

Using metrics in `service-skeleton` has three separate parts, each of which we try to make as painless as possible.

* First, you need to *declare* the metrics that you use, so that everyone is on the same page about what is being measured.

* Next, you need to *populate* the metrics as your application runs, recording values of interest.

* Finally, the values need to be *exposed* to the metrics collection server, for processing, display, and alerting.

Declaring metrics is done by configuring `service-skeleton` before your service starts running.
We use the common [Builder pattern](https://en.wikipedia.org/wiki/Builder_pattern) to configure metrics (as well as everything else in `service-skeleton`).
Thus, if we wanted to have a counter that exposed how many times the service has said hello, it would look like this:

```rust
use service_skeleton::service;

fn main() {
    service("InstrumentedHello")
        .counter::<()>("count", "Number of times we've said hello")
        .run(|_cfg: ()| say_hello());
}
# fn say_hello() { std::process::exit(0) }
```

There are also `gauge` and `histogram` methods that declare a metric of those types.

To access your newly-created counter, call the [`counter`](https://docs.rs/service-skeleton/latest/...) function, passing the metric name and label set, and a closure that manipulates the counter as needed:

```rust
# use service_skeleton::service;
# fn main() {
#    service("InstrumentedHello")
#        .counter::<()>("count", "Number of times we've said hello")
#        .run(|_cfg: ()| say_hello());
# }
# use std::time::Duration;
# fn sleep(_: Duration) { std::process::exit(0) }
use service_skeleton::metric::counter;

fn say_hello() {
    println!("Hello, Metrics!");
    counter("count", &(), |m| { m.inc(); });
    sleep(Duration::from_secs(5));
}
```

> The reference to `()` in the `counter` call (and as the type in the counter declaration, `::<()>`) refers to the *label set*; it is possible to provide arbitrary types as the labels for metrics calls.
> See [the prometheus-client docs](https://docs.rs/prometheus-client/latest/prometheus_client/encoding/trait.EncodeLabelSet.html) for more information on custom label set types.

Finally, you need to be able to *scrape* your metric to get it into your monitoring system.
To do that, `service-skeleton` comes with a built-in metrics server, but for security it's turned off by default.
It's simple to enable it, though: just set the `INSTRUMENTED_HELLO_METRICS_SERVER_PORT` environment variable, then you can hit the metrics server:

```sh
INSTRUMENTED_HELLO_METRICS_SERVER_PORT=9543 cargo run
# In another terminal, run
curl http://localhost:9543
# ... and you should see your metrics appear
```

Note that, like user-defined configuration, the environment variable name for the metrics port takes its prefix from the service name passed to `start`.


# Further Reading

See [the API docs](https://docs.rs/service-skeleton) for full(ish) details on everything that's available.


# Licence

Unless otherwise stated, everything in this repo is covered by the following
copyright notice:

```text
    Copyright (C) 2023  Matt Palmer <matt@hezmatt.org>

    This program is free software: you can redistribute it and/or modify it
    under the terms of the GNU General Public License version 3, as
    published by the Free Software Foundation.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
```
