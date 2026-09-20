//! Text encoding of NWN2 game files.
//!
//! NWN2 Enhanced Edition writes every string it produces itself (typed
//! character names, `playerinfo.bin`, localized substrings it copies) as
//! UTF-8, verified 2026-09-19 against a save from a Russian client. Blueprints
//! shipped with the game and pre-EE saves still hold the legacy Windows code
//! page of their language: German module text is Windows-1252, Polish is
//! Windows-1250, Russian Windows-1251. Saves therefore mix both.
//!
//! Decoding tries strict UTF-8 first and falls back to the code page of the
//! substring's NWN language id, or of the editor language for plain strings.
//! Writers emit UTF-8 (`str::as_bytes`), like the EE engine does; the original
//! NWN2 engine reads it too (verified 2026-09-20: a UTF-8 `Jörg Müßig` written
//! into an original save rendered in the original game). That is stable across
//! save cycles because valid UTF-8 never falls back to a code page, so a legacy
//! string is transcoded once and then round-trips byte-identical.

use std::sync::atomic::{AtomicU8, Ordering};

use encoding_rs::{Encoding, WINDOWS_1250, WINDOWS_1251, WINDOWS_1252};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum GameLanguage {
    #[default]
    English = 0,
    French = 1,
    German = 2,
    Italian = 3,
    Spanish = 4,
    Polish = 5,
    Russian = 6,
}

/// Lowercase primary subtag of a BCP 47 or POSIX locale (`de-DE` -> `de`).
pub fn primary_subtag(locale: &str) -> String {
    locale
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

impl GameLanguage {
    /// Maps an editor locale (`en`, `de-DE`, ...) to the game language.
    /// Unknown locales fall back to English.
    pub fn from_locale(locale: &str) -> Self {
        match primary_subtag(locale).as_str() {
            "fr" => Self::French,
            "de" => Self::German,
            "it" => Self::Italian,
            "es" => Self::Spanish,
            "pl" => Self::Polish,
            "ru" => Self::Russian,
            _ => Self::English,
        }
    }

    /// Folder name below `<game>/localization/`.
    pub fn localization_folder(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::French => "french",
            Self::German => "german",
            Self::Italian => "italian",
            Self::Spanish => "spanish",
            Self::Polish => "polish",
            Self::Russian => "russian",
        }
    }

    /// Code page pre-EE game files use for this language.
    pub fn legacy_encoding(self) -> &'static Encoding {
        match self {
            Self::Polish => WINDOWS_1250,
            Self::Russian => WINDOWS_1251,
            _ => WINDOWS_1252,
        }
    }

    /// Language of a GFF `CExoLocString` substring or TLK header id.
    /// Russian has no id of its own: the Russian localization reuses id 0.
    fn from_nwn_language_id(id: u32) -> Option<Self> {
        match id {
            1 => Some(Self::French),
            2 => Some(Self::German),
            3 => Some(Self::Italian),
            4 => Some(Self::Spanish),
            5 => Some(Self::Polish),
            _ => None,
        }
    }

    fn from_repr(value: u8) -> Self {
        match value {
            1 => Self::French,
            2 => Self::German,
            3 => Self::Italian,
            4 => Self::Spanish,
            5 => Self::Polish,
            6 => Self::Russian,
            _ => Self::English,
        }
    }
}

static GAME_LANGUAGE: AtomicU8 = AtomicU8::new(GameLanguage::English as u8);

/// Sets the process-wide game language used for legacy text fallback and
/// localized asset selection. Called from app startup and the settings command.
pub fn set_game_language(language: GameLanguage) {
    GAME_LANGUAGE.store(language as u8, Ordering::Relaxed);
}

pub fn game_language() -> GameLanguage {
    GameLanguage::from_repr(GAME_LANGUAGE.load(Ordering::Relaxed))
}

/// Decodes a plain game string (CExoString, `playerinfo.bin`, TLK entry):
/// strict UTF-8, else the editor language's legacy code page.
pub fn decode_text(bytes: &[u8]) -> String {
    decode_localized_text(bytes, 0)
}

/// Decodes a localized substring: strict UTF-8, else the code page of the
/// substring's NWN language id. Id 0 is shared by English and Russian, so it
/// and unknown ids use the editor language.
pub fn decode_localized_text(bytes: &[u8], nwn_language_id: u32) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => GameLanguage::from_nwn_language_id(nwn_language_id)
            .unwrap_or_else(game_language)
            .legacy_encoding()
            .decode(bytes)
            .0
            .into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test because the language is process-global and tests run in parallel.
    #[test]
    fn utf8_first_then_language_code_page() {
        set_game_language(GameLanguage::Russian);
        assert_eq!(decode_text("Тест".as_bytes()), "Тест");
        assert_eq!(decode_text("Ärger".as_bytes()), "Ärger");
        assert_eq!(decode_localized_text(&[b'B', 0xE4, b'r'], 2), "Bär");
        assert_eq!(decode_localized_text(&[0xB3, 0xF3, 0xBF], 5), "łóż");
        assert_eq!(decode_localized_text(&[0xD2, 0xE5, 0xF1, 0xF2], 0), "Тест");
    }

    #[test]
    fn locale_mapping() {
        assert_eq!(GameLanguage::from_locale("de-DE"), GameLanguage::German);
        assert_eq!(GameLanguage::from_locale("ru"), GameLanguage::Russian);
        assert_eq!(GameLanguage::from_locale("xx"), GameLanguage::English);
    }
}
