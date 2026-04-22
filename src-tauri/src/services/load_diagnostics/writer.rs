//! Atomic write of LoadReport to app data directory.

use std::path::{Path, PathBuf};
use tracing::{debug, error};

use super::LoadReport;

const REPORT_FILENAME: &str = "last_load_report.json";
const TMP_SUFFIX: &str = ".tmp";

/// Resolve the diagnostics file path using the same convention as app_config.
pub fn report_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("nwn2_save_editor").join(REPORT_FILENAME))
}

/// Serialize and atomically write the report. Returns the written path on success.
///
/// Failures are logged and swallowed — the caller must not propagate diagnostic
/// write failures into user-facing load errors.
pub fn write(report: &LoadReport) -> Option<PathBuf> {
    let target = report_path()?;
    write_to(report, &target)
}

/// Testable variant: write to an explicit target path.
pub fn write_to(report: &LoadReport, target: &Path) -> Option<PathBuf> {
    if let Some(parent) = target.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        error!("load_diagnostics: failed to create parent dir: {e}");
        return None;
    }

    let json = match serde_json::to_string_pretty(report) {
        Ok(j) => j,
        Err(e) => {
            error!("load_diagnostics: failed to serialize report: {e}");
            return None;
        }
    };

    let tmp = tmp_path(target);

    if let Err(e) = std::fs::write(&tmp, json.as_bytes()) {
        error!("load_diagnostics: failed to write tmp file: {e}");
        return None;
    }

    if let Err(e) = std::fs::rename(&tmp, target) {
        error!("load_diagnostics: failed to rename tmp into place: {e}");
        let _ = std::fs::remove_file(&tmp);
        return None;
    }

    debug!("load_diagnostics: wrote report to {}", target.display());
    Some(target.to_path_buf())
}

fn tmp_path(target: &Path) -> PathBuf {
    let ext = target
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("json");
    target.with_extension(format!("{ext}{TMP_SUFFIX}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::load_diagnostics::{LoadInput, LoadReport, LoadStage};
    use tempfile::TempDir;

    fn sample_report() -> LoadReport {
        let mut r = LoadReport::new(LoadInput {
            file_path: "test.zip".into(),
            player_index: None,
            file_size: None,
        });
        r.add_warning(LoadStage::HakLoad, "missing", serde_json::Value::Null);
        r.finalize();
        r
    }

    #[test]
    fn writes_json_to_target_path() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("report.json");
        let written = write_to(&sample_report(), &target).expect("write succeeded");
        assert_eq!(written, target);
        let contents = std::fs::read_to_string(&target).unwrap();
        assert!(contents.contains("\"status\": \"partial_ok\""));
    }

    #[test]
    fn creates_parent_directory_if_missing() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("nested/deep/report.json");
        let written = write_to(&sample_report(), &target);
        assert!(written.is_some());
        assert!(target.exists());
    }

    #[test]
    fn no_tmp_file_left_after_success() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("report.json");
        write_to(&sample_report(), &target).unwrap();
        let leftover = tmp_path(&target);
        assert!(
            !leftover.exists(),
            "tmp file should not persist after success"
        );
    }

    #[test]
    fn report_path_joins_data_dir() {
        if let Some(path) = report_path() {
            let s = path.to_string_lossy();
            assert!(
                s.ends_with("nwn2_save_editor/last_load_report.json")
                    || s.ends_with("nwn2_save_editor\\last_load_report.json"),
                "unexpected path: {s}"
            );
        }
    }
}
