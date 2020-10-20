#![feature(decl_macro)]

#[cfg(test)]
mod tests;
mod metrics;

extern crate serde;
#[macro_use]
extern crate rocket;
extern crate env_logger;
#[macro_use]
extern crate log;

use serde_json::{json};
use rocket::Rocket;
use rocket_prometheus::{prometheus, PrometheusMetrics};
use crate::metrics::APP_INFO;

static COMPRESSED_DEPENDENCY_LIST: &[u8] = auditable::inject_dependency_list!();

#[get("/health")]
fn get_health() -> String {
    json!({"status":"ok"}).to_string()
}

fn prepare_rocket() -> Rocket {
    let prometheus = PrometheusMetrics::new();
    prometheus
        .registry()
        .register(Box::new(APP_INFO.clone()))
        .unwrap();
    let appdata_gauge =
        APP_INFO.with_label_values(&[env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), env!("GIT_HASH")]);
    appdata_gauge.set(1.0);
    prometheus::gather();

    rocket::ignite()
        .attach(prometheus.clone())
        .mount("/metrics", prometheus)
        .mount("/", routes![get_health])
}
fn main() {
    match std::env::var("RUST_LOG") {
        Err(_) => std::env::set_var("RUST_LOG", "debug"),
        Ok(_) => (),
    }
    env_logger::init();
    info!(
        "cargover:{}, githash:{}, auditable_dependencies:{}",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        COMPRESSED_DEPENDENCY_LIST[0]);
    prepare_rocket().launch();
}