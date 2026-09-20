use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::text_encoding::primary_subtag;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: String,
    pub language: String,
    pub font_size: String,
    pub auto_backup: bool,
    pub backup_count: u32,
    pub auto_close_on_launch: bool,
    pub show_launch_dialog: bool,
    pub last_save_path: Option<PathBuf>,
    pub recent_saves: Vec<PathBuf>,
    pub max_recent_saves: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "en".to_string(),
            font_size: "medium".to_string(),
            auto_backup: true,
            backup_count: 3,
            auto_close_on_launch: false,
            show_launch_dialog: true,
            last_save_path: None,
            recent_saves: Vec::new(),
            max_recent_saves: 10,
        }
    }
}

/// UI locales that have an `i18n/<locale>.json` file; mirrors that directory
/// and the language menu in `SettingsPanel.tsx`.
const UI_LANGUAGES: [&str; 6] = ["en", "de", "es", "fr", "pl", "ru"];

/// BCP 47 tag of the OS regional format. `NWN2EE_LOCALE` overrides it for
/// testing other regions without changing system settings.
pub fn system_locale() -> String {
    std::env::var("NWN2EE_LOCALE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(sys_locale::get_locale)
        .unwrap_or_else(|| "en-US".to_string())
}

/// The UI language matching the OS locale, English when there is no
/// translation for it.
fn detect_ui_language(locale: &str) -> String {
    let primary = primary_subtag(locale);
    if UI_LANGUAGES.contains(&primary.as_str()) {
        primary
    } else {
        "en".to_string()
    }
}

impl AppConfig {
    /// Reads the saved config. On the first start (no file yet) the UI
    /// language follows the OS locale; once saved, the user's choice wins.
    pub fn load() -> Self {
        if let Some(config_path) = Self::get_config_path()
            && config_path.exists()
            && let Ok(content) = std::fs::read_to_string(&config_path)
            && let Ok(config) = serde_json::from_str(&content)
        {
            return config;
        }
        Self {
            language: detect_ui_language(&system_locale()),
            ..Self::default()
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(config_path) = Self::get_config_path() {
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            std::fs::write(config_path, json)?;
        }
        Ok(())
    }

    fn get_config_path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("nwn2_save_editor").join("app_config.json"))
    }

    pub fn add_recent_save(&mut self, path: PathBuf) {
        self.recent_saves.retain(|p| p != &path);
        self.recent_saves.insert(0, path.clone());
        if self.recent_saves.len() > self.max_recent_saves {
            self.recent_saves.truncate(self.max_recent_saves);
        }
        self.last_save_path = Some(path);
        let _ = self.save();
    }

    pub fn clear_recent_saves(&mut self) {
        self.recent_saves.clear();
        let _ = self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::detect_ui_language;

    #[test]
    fn ui_language_follows_os_locale_when_translated() {
        assert_eq!(detect_ui_language("de-DE"), "de");
        assert_eq!(detect_ui_language("ru_RU"), "ru");
        assert_eq!(detect_ui_language("pt-BR"), "en");
        assert_eq!(detect_ui_language(""), "en");
    }
}
