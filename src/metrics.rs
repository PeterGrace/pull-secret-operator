use once_cell::sync::Lazy;
use rocket_prometheus::prometheus::{opts, GaugeVec};

pub(crate) static APP_INFO: Lazy<GaugeVec> = Lazy::new(|| {
    GaugeVec::new(
        opts!(
            "app_info",
            "static app labels that potentially only change at restart"
        ),
        &["app_name","crate_version", "git_hash"],
    )
        .expect("Could not create lazy GaugeVec for app_info metric")
});
