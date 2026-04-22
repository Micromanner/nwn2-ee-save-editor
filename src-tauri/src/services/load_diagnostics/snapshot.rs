//! Captures the debug-export payload as a JSON value for inclusion in LoadReport.

use crate::state::AppState;

/// Gather the debug snapshot as an opaque JSON value.
///
/// Failures serializing any part fall back to `serde_json::Value::Null` so the
/// report still writes.
pub async fn capture(state: &AppState) -> serde_json::Value {
    let debug_log = crate::commands::debug::gather_debug_data(state).await;
    serde_json::to_value(&debug_log).unwrap_or(serde_json::Value::Null)
}
