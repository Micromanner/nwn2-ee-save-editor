use crate::config::NWN2Paths;
use crate::parsers::gff::{
    GffParser, GffValue, GffWriter, insert_bool_preserving_type, insert_u32_preserving_type,
};
use chrono::Local;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use super::backup::{collect_backups, ensure_backup_in_dir, replace_file_atomically};
use super::content::find_campaign_path;
use crate::services::savegame_handler::backup::BACKUP_TIMESTAMP_FORMAT;
use crate::services::savegame_handler::commit_outputs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignSettings {
    pub campaign_file_path: String,
    pub guid: String,
    pub level_cap: u32,
    pub xp_cap: u32,
    pub companion_xp_weight: f32,
    pub henchman_xp_weight: f32,
    pub attack_neutrals: bool,
    pub auto_xp_award: bool,
    pub journal_sync: bool,
    pub no_char_changing: bool,
    pub use_personal_reputation: bool,
    pub start_module: String,
    pub module_names: Vec<String>,
    pub display_name: String,
    pub description: String,
}

fn campaign_guid(root: &IndexMap<String, GffValue<'_>>) -> String {
    match root.get("GUID") {
        Some(GffValue::Void(bytes)) => hex::encode(bytes),
        Some(GffValue::String(s)) => s.to_string(),
        _ => String::new(),
    }
}

pub fn read_campaign_settings(
    campaign_id: &str,
    paths: &NWN2Paths,
) -> Result<CampaignSettings, String> {
    let campaign_file = find_campaign_path(campaign_id, paths)
        .ok_or_else(|| format!("Campaign file for GUID {campaign_id} not found"))?;

    let parser = GffParser::new(&campaign_file)
        .map_err(|e| format!("Failed to parse campaign file: {e}"))?;

    let root = parser
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read root struct: {e}"))?;

    let mut settings = CampaignSettings {
        campaign_file_path: campaign_file.to_string_lossy().to_string(),
        ..Default::default()
    };

    // Helper closure to getting values
    let get_bool = |key: &str| -> bool {
        match root.get(key) {
            Some(GffValue::Byte(v)) => *v != 0,
            Some(GffValue::Int(v)) => *v != 0,
            _ => false,
        }
    };

    let get_u32 = |key: &str| -> u32 {
        match root.get(key) {
            Some(GffValue::Dword(v)) => *v,
            Some(GffValue::Int(v)) => *v as u32,
            Some(GffValue::Byte(v)) => u32::from(*v),
            _ => 0,
        }
    };

    let get_f32 = |key: &str| -> f32 {
        match root.get(key) {
            Some(GffValue::Float(v)) => *v,
            _ => 0.0,
        }
    };

    let get_string = |key: &str| -> String {
        match root.get(key) {
            Some(GffValue::String(s) | GffValue::ResRef(s)) => s.to_string(),
            _ => String::new(),
        }
    };

    settings.guid = campaign_guid(&root);

    settings.level_cap = get_u32("LvlCap");
    settings.xp_cap = get_u32("XPCap");
    settings.companion_xp_weight = get_f32("CompXPWt");
    settings.henchman_xp_weight = get_f32("HenchXPWt");
    settings.attack_neutrals = get_bool("AttackNeut");
    settings.auto_xp_award = get_bool("AutoXPAwd");
    settings.journal_sync = get_bool("JournalSynch");
    settings.no_char_changing = get_bool("NoCharChanging");
    settings.use_personal_reputation = get_bool("UsePersonalRep");
    settings.start_module = get_string("StartModule");

    // Module Names
    if let Some(GffValue::List(mod_names)) = root.get("ModNames") {
        for item in mod_names {
            let fields = item.force_load();
            if let Some(GffValue::String(s) | GffValue::ResRef(s)) = fields.get("ModuleName") {
                settings.module_names.push(s.to_string());
            }
        }
    }

    // Display Name
    if let Some(GffValue::LocString(ls)) = root.get("DisplayName") {
        settings.display_name = ls
            .substrings
            .first()
            .map(|s| s.string.clone().into_owned())
            .unwrap_or_default();
    }

    // Description
    if let Some(GffValue::LocString(ls)) = root.get("Description") {
        settings.description = ls
            .substrings
            .first()
            .map(|s| s.string.clone().into_owned())
            .unwrap_or_default();
    }

    Ok(settings)
}

pub fn update_campaign_settings(
    settings: &CampaignSettings,
    paths: &NWN2Paths,
) -> Result<(), String> {
    let campaign_file = find_campaign_path(&settings.guid, paths)
        .ok_or_else(|| format!("Campaign file for GUID {} not found", settings.guid))?;

    backup_campaign_file(&campaign_file, settings, paths)?;
    // Read existing GFF into memory to preserve fields not in CampaignSettings
    let file_bytes =
        fs::read(&campaign_file).map_err(|e| format!("Failed to read campaign file: {e}"))?;
    let parser = GffParser::from_bytes(file_bytes)
        .map_err(|e| format!("Failed to parse campaign file for update: {e}"))?;
    let root_fields = parser
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read root struct: {e}"))?;
    let mut owned_fields: IndexMap<String, GffValue<'static>> = root_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    insert_u32_preserving_type(&mut owned_fields, "LvlCap", settings.level_cap);
    insert_u32_preserving_type(&mut owned_fields, "XPCap", settings.xp_cap);
    owned_fields.insert(
        "CompXPWt".to_string(),
        GffValue::Float(settings.companion_xp_weight),
    );
    owned_fields.insert(
        "HenchXPWt".to_string(),
        GffValue::Float(settings.henchman_xp_weight),
    );
    insert_bool_preserving_type(&mut owned_fields, "AttackNeut", settings.attack_neutrals);
    insert_bool_preserving_type(&mut owned_fields, "AutoXPAwd", settings.auto_xp_award);
    insert_bool_preserving_type(&mut owned_fields, "JournalSynch", settings.journal_sync);
    insert_bool_preserving_type(
        &mut owned_fields,
        "NoCharChanging",
        settings.no_char_changing,
    );
    insert_bool_preserving_type(
        &mut owned_fields,
        "UsePersonalRep",
        settings.use_personal_reputation,
    );

    // Write back
    let mut writer = GffWriter::new("CAM ", "V3.2");

    let bytes = writer
        .write(owned_fields)
        .map_err(|e| format!("Failed to serialize campaign settings: {e:?}"))?;

    commit_outputs(vec![(campaign_file.clone(), bytes)])
        .map_err(|e| format!("Failed to write campaign file: {e}"))?;

    info!("Updated campaign settings in {:?}", campaign_file);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignBackupInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

fn get_campaign_backups_dir(paths: &NWN2Paths) -> Result<PathBuf, String> {
    Ok(paths
        .saves()
        .ok_or("Saves path not configured")?
        .join("backups")
        .join("campaign_backups"))
}

fn backup_campaign_file(
    path: &Path,
    settings: &CampaignSettings,
    paths: &NWN2Paths,
) -> Result<PathBuf, String> {
    let backup_dir = get_campaign_backups_dir(paths)?;
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = Local::now().format(BACKUP_TIMESTAMP_FORMAT);
    let safe_name: String = settings
        .display_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect();
    let safe_name = safe_name.replace(' ', "_");
    let safe_name: String = safe_name.chars().take(30).collect();

    let guid_prefix: String = settings.guid.chars().take(8).collect();
    let filename = format!("{safe_name}_{guid_prefix}_{timestamp}.cam");
    let backup_path = backup_dir.join(filename);

    fs::copy(path, &backup_path).map_err(|e| e.to_string())?;

    info!("Created campaign backup at {:?}", backup_path);
    Ok(backup_path)
}

pub fn list_campaign_backups(
    campaign_id: &str,
    paths: &NWN2Paths,
) -> Result<Vec<CampaignBackupInfo>, String> {
    let guid_prefix: String = campaign_id.chars().take(8).collect();
    let mut backups = collect_backups(&get_campaign_backups_dir(paths)?, "cam")?;
    backups.retain(|b| guid_prefix.is_empty() || b.filename.contains(&guid_prefix));
    Ok(backups)
}

pub fn restore_campaign_from_backup(
    backup_path: &str,
    campaign_id: &str,
    paths: &NWN2Paths,
) -> Result<(), String> {
    let backup = PathBuf::from(backup_path);
    ensure_backup_in_dir(&backup, &get_campaign_backups_dir(paths)?)?;

    let campaign_file = find_campaign_path(campaign_id, paths)
        .ok_or_else(|| format!("Campaign file for GUID {campaign_id} not found"))?;

    let backup_root = GffParser::new(&backup)
        .and_then(|p| p.read_struct_fields(0))
        .map_err(|e| format!("Failed to read campaign backup: {e}"))?;
    if !campaign_guid(&backup_root).eq_ignore_ascii_case(campaign_id) {
        return Err(format!(
            "Backup {} belongs to a different campaign",
            backup.display()
        ));
    }

    replace_file_atomically(&backup, &campaign_file)?;

    info!(
        "Restored campaign from backup: {} -> {}",
        backup.display(),
        campaign_file.display()
    );
    Ok(())
}

#[cfg(test)]
mod settings_tests;
