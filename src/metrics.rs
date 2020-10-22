use once_cell::sync::Lazy;
use prometheus::{opts, register_gauge_vec, GaugeVec};

pub(crate) static APP_INFO: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        opts!(
            "app_info",
            "static app labels that potentially only change at restart"
        ),
        &["app_name", "crate_version", "git_hash"]
    )
    .expect("Could not create lazy GaugeVec for app_info metric")
});
