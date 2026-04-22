//! Load diagnostics: structured warnings and errors captured during save loading.
//!
//! Single-writer model: `load_character` owns a `LoadReport`, mutable borrows pass
//! down into resource manager and savegame handler. Written to app data on exit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod snapshot;
pub mod writer;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadStatus {
    Ok,
    PartialOk,
    Fatal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoadStage {
    SaveOpen,
    Playerlist,
    Gff,
    Bic,
    ResourceInit,
    HakLoad,
    CustomTlk,
    ModuleResolve,
    CharacterFields,
}

impl std::fmt::Display for LoadStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::SaveOpen => "save_open",
            Self::Playerlist => "playerlist",
            Self::Gff => "gff",
            Self::Bic => "bic",
            Self::ResourceInit => "resource_init",
            Self::HakLoad => "hak_load",
            Self::CustomTlk => "custom_tlk",
            Self::ModuleResolve => "module_resolve",
            Self::CharacterFields => "character_fields",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadWarning {
    pub stage: LoadStage,
    pub message: String,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadError {
    pub stage: LoadStage,
    pub message: String,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadInput {
    pub file_path: String,
    pub player_index: Option<usize>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadReport {
    pub schema_version: u32,
    pub status: LoadStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub input: LoadInput,
    pub warnings: Vec<LoadWarning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fatal: Option<LoadError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<serde_json::Value>,
}

impl LoadReport {
    pub fn new(input: LoadInput) -> Self {
        Self {
            schema_version: 1,
            status: LoadStatus::Ok,
            started_at: Utc::now(),
            finished_at: None,
            duration_ms: None,
            input,
            warnings: Vec::new(),
            fatal: None,
            snapshot: None,
        }
    }

    pub fn add_warning(
        &mut self,
        stage: LoadStage,
        message: impl Into<String>,
        context: serde_json::Value,
    ) {
        self.warnings.push(LoadWarning {
            stage,
            message: message.into(),
            context,
        });
        if self.status == LoadStatus::Ok {
            self.status = LoadStatus::PartialOk;
        }
    }

    pub fn set_fatal(
        &mut self,
        stage: LoadStage,
        message: impl Into<String>,
        context: serde_json::Value,
    ) {
        self.fatal = Some(LoadError {
            stage,
            message: message.into(),
            context,
        });
        self.status = LoadStatus::Fatal;
    }

    pub fn finalize(&mut self) {
        let finished = Utc::now();
        self.duration_ms = Some((finished - self.started_at).num_milliseconds().max(0) as u64);
        self.finished_at = Some(finished);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> LoadInput {
        LoadInput {
            file_path: "test.zip".into(),
            player_index: None,
            file_size: None,
        }
    }

    #[test]
    fn new_report_has_ok_status_and_no_warnings() {
        let report = LoadReport::new(sample_input());
        assert_eq!(report.status, LoadStatus::Ok);
        assert!(report.warnings.is_empty());
        assert!(report.fatal.is_none());
        assert_eq!(report.schema_version, 1);
    }

    #[test]
    fn add_warning_promotes_status_to_partial_ok() {
        let mut report = LoadReport::new(sample_input());
        report.add_warning(
            LoadStage::HakLoad,
            "missing hak",
            serde_json::json!({"hak": "x.hak"}),
        );
        assert_eq!(report.status, LoadStatus::PartialOk);
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn set_fatal_overrides_status() {
        let mut report = LoadReport::new(sample_input());
        report.add_warning(LoadStage::HakLoad, "missing hak", serde_json::Value::Null);
        report.set_fatal(LoadStage::SaveOpen, "cannot open", serde_json::Value::Null);
        assert_eq!(report.status, LoadStatus::Fatal);
        assert!(report.fatal.is_some());
    }

    #[test]
    fn serializes_to_expected_json_shape() {
        let input = LoadInput {
            file_path: "test.zip".into(),
            player_index: Some(0),
            file_size: Some(1024),
        };
        let mut report = LoadReport::new(input);
        report.add_warning(
            LoadStage::HakLoad,
            "hak missing",
            serde_json::json!({"hak": "foo.hak"}),
        );
        report.finalize();

        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["status"], "partial_ok");
        assert_eq!(json["warnings"][0]["stage"], "HakLoad");
        assert_eq!(json["warnings"][0]["message"], "hak missing");
        assert_eq!(json["warnings"][0]["context"]["hak"], "foo.hak");
    }

    #[test]
    fn finalize_computes_duration_ms() {
        let mut report = LoadReport::new(sample_input());
        std::thread::sleep(std::time::Duration::from_millis(5));
        report.finalize();
        assert!(report.duration_ms.unwrap() >= 5);
        assert!(report.finished_at.is_some());
    }
}
