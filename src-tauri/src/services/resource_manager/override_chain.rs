use std::cmp::Ordering;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::zip_content_reader::ZipEntryLocation;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverrideSource {
    BaseGame,
    Expansion,
    Module,
    Campaign,
    OverrideDir,
    Workshop,
    CustomOverride,
    Hak(u8),
}

impl OverrideSource {
    pub fn priority(&self) -> u8 {
        match self {
            Self::BaseGame => 0,
            Self::Expansion => 1,
            Self::Module => 2,
            Self::Campaign => 3,
            Self::OverrideDir => 4,
            Self::Workshop => 5,
            Self::CustomOverride => 6,
            Self::Hak(idx) => 7 + idx,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BaseGame => "Base Game",
            Self::Expansion => "Expansion",
            Self::Module => "Module",
            Self::Campaign => "Campaign",
            Self::OverrideDir => "Override Directory",
            Self::Workshop => "Steam Workshop",
            Self::CustomOverride => "Custom Override",
            Self::Hak(_) => "HAK Pack",
        }
    }
}

impl Ord for OverrideSource {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority().cmp(&other.priority())
    }
}

impl PartialOrd for OverrideSource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainerType {
    Zip,
    Erf,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLocation {
    pub source: OverrideSource,
    pub container_type: ContainerType,
    pub container_path: PathBuf,
    pub internal_path: Option<String>,
    pub modified_time: f64,
    /// Set for zip entries the startup scan can read directly, skipping the
    /// archive open (a one-second central-directory read on the model zips).
    #[serde(default)]
    pub zip_entry: Option<ZipEntryLocation>,
}

impl ResourceLocation {
    pub fn from_zip(
        source: OverrideSource,
        zip_path: PathBuf,
        internal: String,
        mtime: f64,
        zip_entry: Option<ZipEntryLocation>,
    ) -> Self {
        Self {
            source,
            container_type: ContainerType::Zip,
            container_path: zip_path,
            internal_path: Some(internal),
            modified_time: mtime,
            zip_entry,
        }
    }

    pub fn from_erf(
        source: OverrideSource,
        erf_path: PathBuf,
        internal: String,
        mtime: f64,
    ) -> Self {
        Self {
            source,
            container_type: ContainerType::Erf,
            container_path: erf_path,
            internal_path: Some(internal),
            modified_time: mtime,
            zip_entry: None,
        }
    }

    pub fn from_file(source: OverrideSource, file_path: PathBuf, mtime: f64) -> Self {
        Self {
            source,
            container_type: ContainerType::Directory,
            container_path: file_path,
            internal_path: None,
            modified_time: mtime,
            zip_entry: None,
        }
    }

    pub fn is_loose_file(&self) -> bool {
        matches!(self.container_type, ContainerType::Directory)
    }

    pub fn is_archive(&self) -> bool {
        matches!(self.container_type, ContainerType::Zip | ContainerType::Erf)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub mod_id: String,
    pub entry_area: String,
    pub custom_tlk: String,
    pub hak_list: Vec<String>,
    pub campaign_id: Option<String>,
    pub is_directory: bool,
    pub path: PathBuf,
}

impl Default for ModuleInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            mod_id: String::new(),
            entry_area: String::new(),
            custom_tlk: String::new(),
            hak_list: Vec::new(),
            campaign_id: None,
            is_directory: false,
            path: PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignInfo {
    pub name: String,
    pub guid: String,
    pub description: String,
    pub module_names: Vec<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub resref: String,
    pub container_type: ContainerType,
    pub container_path: PathBuf,
    pub internal_path: Option<String>,
    pub source: OverrideSource,
}

impl TemplateInfo {
    pub fn from_zip(
        resref: String,
        source: OverrideSource,
        zip_path: PathBuf,
        internal: String,
    ) -> Self {
        Self {
            resref,
            container_type: ContainerType::Zip,
            container_path: zip_path,
            internal_path: Some(internal),
            source,
        }
    }

    pub fn from_erf(
        resref: String,
        source: OverrideSource,
        erf_path: PathBuf,
        internal: String,
    ) -> Self {
        Self {
            resref,
            container_type: ContainerType::Erf,
            container_path: erf_path,
            internal_path: Some(internal),
            source,
        }
    }

    pub fn from_file(resref: String, source: OverrideSource, file_path: PathBuf) -> Self {
        Self {
            resref,
            container_type: ContainerType::Directory,
            container_path: file_path,
            internal_path: None,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_source_priority() {
        assert!(OverrideSource::BaseGame < OverrideSource::Expansion);
        assert!(OverrideSource::Expansion < OverrideSource::Module);
        assert!(OverrideSource::Module < OverrideSource::Campaign);
        assert!(OverrideSource::Campaign < OverrideSource::OverrideDir);
        assert!(OverrideSource::OverrideDir < OverrideSource::Workshop);
        assert!(OverrideSource::Workshop < OverrideSource::CustomOverride);
        assert!(OverrideSource::CustomOverride < OverrideSource::Hak(0));
        assert!(OverrideSource::Hak(0) < OverrideSource::Hak(1));
    }

    #[test]
    fn test_resource_location_types() {
        let zip_loc = ResourceLocation::from_zip(
            OverrideSource::BaseGame,
            PathBuf::from("2da.zip"),
            "classes.2da".to_string(),
            0.0,
            None,
        );
        assert!(zip_loc.is_archive());
        assert!(!zip_loc.is_loose_file());

        let file_loc = ResourceLocation::from_file(
            OverrideSource::OverrideDir,
            PathBuf::from("override/classes.2da"),
            0.0,
        );
        assert!(!file_loc.is_archive());
        assert!(file_loc.is_loose_file());
    }
}
