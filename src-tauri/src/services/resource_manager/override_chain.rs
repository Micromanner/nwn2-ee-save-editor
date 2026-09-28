use std::cmp::Ordering;
use std::path::PathBuf;

use crate::utils::zip_content_reader::ZipEntryLocation;

/// Where a resource comes from. `priority` is the engine's lookup order:
/// module HAKs (first listed wins) > override folders > campaign folder >
/// module > base game.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OverrideSource {
    BaseGame,
    Module,
    Campaign,
    OverrideDir,
    Workshop,
    CustomOverride,
    /// Position in the module's `Mod_HakList`, 0 = top.
    Hak(u8),
}

impl OverrideSource {
    pub fn priority(&self) -> u16 {
        match self {
            Self::BaseGame => 0,
            Self::Module => 1,
            Self::Campaign => 2,
            Self::OverrideDir => 3,
            Self::Workshop => 4,
            Self::CustomOverride => 5,
            Self::Hak(idx) => 6 + u16::from(u8::MAX - idx),
        }
    }

    /// Sources that belong to the loaded save's module and are swapped with it.
    pub fn is_module_content(&self) -> bool {
        matches!(self, Self::Module | Self::Campaign | Self::Hak(_))
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::BaseGame => "base_game",
            Self::Module => "module",
            Self::Campaign => "campaign",
            Self::OverrideDir => "override_dir",
            Self::Workshop => "workshop",
            Self::CustomOverride => "custom_override",
            Self::Hak(_) => "hak",
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContainerType {
    Zip,
    Erf,
    Directory,
}

#[derive(Debug, Clone)]
pub struct ResourceLocation {
    pub source: OverrideSource,
    pub container_type: ContainerType,
    pub container_path: PathBuf,
    pub internal_path: Option<String>,
    /// Set for zip entries the startup scan can read directly, skipping the
    /// archive open (a one-second central-directory read on the model zips).
    pub zip_entry: Option<ZipEntryLocation>,
}

impl ResourceLocation {
    pub fn from_zip(
        source: OverrideSource,
        zip_path: PathBuf,
        internal: String,
        zip_entry: Option<ZipEntryLocation>,
    ) -> Self {
        Self {
            source,
            container_type: ContainerType::Zip,
            container_path: zip_path,
            internal_path: Some(internal),
            zip_entry,
        }
    }

    pub fn from_erf(source: OverrideSource, erf_path: PathBuf, internal: String) -> Self {
        Self {
            source,
            container_type: ContainerType::Erf,
            container_path: erf_path,
            internal_path: Some(internal),
            zip_entry: None,
        }
    }

    pub fn from_file(source: OverrideSource, file_path: PathBuf) -> Self {
        Self {
            source,
            container_type: ContainerType::Directory,
            container_path: file_path,
            internal_path: None,
            zip_entry: None,
        }
    }
}

/// The module a save plays in, read from the `module.ifo` inside its `.z`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleContent {
    /// Module resref from `currentmodule.txt`.
    pub name: String,
    pub custom_tlk: String,
    pub hak_list: Vec<String>,
    pub campaign_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_source_priority() {
        assert!(OverrideSource::BaseGame < OverrideSource::Module);
        assert!(OverrideSource::Module < OverrideSource::Campaign);
        assert!(OverrideSource::Campaign < OverrideSource::OverrideDir);
        assert!(OverrideSource::OverrideDir < OverrideSource::Workshop);
        assert!(OverrideSource::Workshop < OverrideSource::CustomOverride);
        assert!(OverrideSource::CustomOverride < OverrideSource::Hak(u8::MAX));
        assert!(OverrideSource::Hak(1) < OverrideSource::Hak(0));
    }
}
