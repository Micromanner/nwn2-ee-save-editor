#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::config::NWN2Paths;
    use crate::services::campaign::settings::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_campaign() -> (TempDir, NWN2Paths, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let campaigns_dir = temp_dir.path().join("Campaigns");
        let my_campaign_dir = campaigns_dir.join("MyCampaign");
        fs::create_dir_all(&my_campaign_dir).unwrap();

        let cam_file = my_campaign_dir.join("campaign.cam");

        use crate::parsers::gff::{GffValue, GffWriter};
        use indexmap::IndexMap;

        let mut fields = IndexMap::new();
        fields.insert("LvlCap".to_string(), GffValue::Dword(30));
        fields.insert("XPCap".to_string(), GffValue::Dword(0));
        fields.insert("CompXPWt".to_string(), GffValue::Float(1.0));
        fields.insert("HenchXPWt".to_string(), GffValue::Float(0.8));
        fields.insert("GUID".to_string(), GffValue::String("test-guid".into()));

        use crate::parsers::gff::{LocalizedString, LocalizedSubstring};
        let loc_string = LocalizedString {
            string_ref: -1,
            substrings: vec![LocalizedSubstring {
                language: 0,
                gender: 0,
                string: "My Campaign".into(),
            }],
        };
        fields.insert("DisplayName".to_string(), GffValue::LocString(loc_string));

        let mut writer = GffWriter::new("CAM ", "V3.2");
        let bytes = writer.write(fields).unwrap();
        fs::write(&cam_file, bytes).unwrap();

        let mut paths = NWN2Paths::new();
        let _ = paths.set_game_folder(temp_dir.path().to_str().unwrap());

        (temp_dir, paths, cam_file)
    }

    #[test]
    fn test_read_campaign_settings() {
        let (_tmp, paths, _cam_file) = setup_test_campaign();
        let result = read_campaign_settings("test-guid", &paths);
        assert!(
            result.is_ok(),
            "Failed to read campaign settings: {:?}",
            result.err()
        );

        let settings = result.unwrap();
        assert_eq!(settings.level_cap, 30);
        assert_eq!(settings.companion_xp_weight, 1.0);
        assert_eq!(settings.display_name, "My Campaign");
    }

    #[test]
    fn update_campaign_settings_preserves_gff_field_types() {
        use crate::parsers::gff::{GffParser, GffValue, variant_name};

        let fixture_src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/saves/STORM_Campaign/campaign.cam");
        if !fixture_src.exists() {
            eprintln!("STORM_Campaign fixture not found, skipping");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let campaign_dir = temp_dir.path().join("Campaigns/StormCampaign");
        fs::create_dir_all(&campaign_dir).unwrap();
        let cam_file = campaign_dir.join("campaign.cam");
        fs::copy(&fixture_src, &cam_file).unwrap();

        let mut paths = NWN2Paths::new();
        let _ = paths.set_game_folder(temp_dir.path().to_str().unwrap());

        let original_bytes = fs::read(&cam_file).unwrap();
        let original = GffParser::from_bytes(original_bytes)
            .unwrap()
            .read_struct_fields(0)
            .unwrap();
        let snapshot = |fields: &indexmap::IndexMap<String, GffValue<'_>>, key: &str| -> String {
            fields.get(key).map_or("missing", variant_name).to_string()
        };

        let watched_keys = [
            "LvlCap",
            "XPCap",
            "CompXPWt",
            "HenchXPWt",
            "AttackNeut",
            "AutoXPAwd",
            "JournalSynch",
            "NoCharChanging",
            "UsePersonalRep",
        ];
        let before: Vec<(String, String)> = watched_keys
            .iter()
            .map(|k| ((*k).to_string(), snapshot(&original, k)))
            .collect();

        let mut settings = read_campaign_settings_from_path(&cam_file).expect("read settings");
        settings.campaign_file_path = cam_file.to_string_lossy().to_string();
        settings.guid = campaign_guid(&original);
        update_campaign_settings(&settings, &paths).expect("write settings");

        let after_bytes = fs::read(&cam_file).unwrap();
        let after = GffParser::from_bytes(after_bytes)
            .unwrap()
            .read_struct_fields(0)
            .unwrap();

        for (key, before_variant) in &before {
            let after_variant = snapshot(&after, key);
            assert_eq!(
                before_variant, &after_variant,
                "field '{key}' GFF type mutated by writer: was {before_variant}, now {after_variant}"
            );
        }
    }

    /// Mirrors `read_campaign_settings` without the GUID-based campaign-folder
    /// lookup, so the test can point at a fixture file directly.
    fn read_campaign_settings_from_path(
        cam_file: &std::path::Path,
    ) -> Result<CampaignSettings, String> {
        use crate::parsers::gff::{GffParser, GffValue};

        let parser = GffParser::new(cam_file).map_err(|e| e.to_string())?;
        let root = parser.read_struct_fields(0).map_err(|e| e.to_string())?;

        let get_u32 = |key: &str| -> u32 {
            match root.get(key) {
                Some(GffValue::Dword(v)) => *v,
                Some(GffValue::Int(v)) => *v as u32,
                Some(GffValue::Byte(v)) => u32::from(*v),
                _ => 0,
            }
        };
        let get_bool = |key: &str| -> bool {
            match root.get(key) {
                Some(GffValue::Byte(v)) => *v != 0,
                Some(GffValue::Int(v)) => *v != 0,
                _ => false,
            }
        };
        let get_f32 = |key: &str| -> f32 {
            match root.get(key) {
                Some(GffValue::Float(v)) => *v,
                _ => 0.0,
            }
        };

        Ok(CampaignSettings {
            campaign_file_path: cam_file.to_string_lossy().to_string(),
            level_cap: get_u32("LvlCap"),
            xp_cap: get_u32("XPCap"),
            companion_xp_weight: get_f32("CompXPWt"),
            henchman_xp_weight: get_f32("HenchXPWt"),
            attack_neutrals: get_bool("AttackNeut"),
            auto_xp_award: get_bool("AutoXPAwd"),
            journal_sync: get_bool("JournalSynch"),
            no_char_changing: get_bool("NoCharChanging"),
            use_personal_reputation: get_bool("UsePersonalRep"),
            ..Default::default()
        })
    }
}
