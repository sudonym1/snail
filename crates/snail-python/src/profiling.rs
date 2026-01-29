use std::sync::OnceLock;

pub(crate) fn profile_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("SNAIL_PROFILE_NATIVE").is_some())
}

pub(crate) fn log_profile(label: &str, elapsed: std::time::Duration) {
    if profile_enabled() {
        eprintln!(
            "[snail][native] {label}: {:.3} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }
}
