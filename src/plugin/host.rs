/// Host functions exposed to WASM plugins via extism.
/// Each function is registered via extism's UserData / host function mechanism.

use tracing::{debug, info, warn};

pub fn horae_log(level: &str, message: &str) {
    match level {
        "error" => tracing::error!(plugin_msg = %message),
        "warn" => warn!(plugin_msg = %message),
        "info" => info!(plugin_msg = %message),
        _ => debug!(plugin_msg = %message),
    }
}
