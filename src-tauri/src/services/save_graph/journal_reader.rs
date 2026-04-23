//! Reads the player's live journal state from the save's `playerlist.ifo`.
//!
//! NWN/NWN2 scripts (via `SetJournalEntry`) persist quest progress into the player's
//! per-character `VarTable` as `NW_JOURNAL_ENTRY<tag>` entries — not into a dedicated
//! `JournalList` struct. Each entry's value is the current state integer; sibling
//! entries `NW_JOURNAL_DATE<tag>` / `NW_JOURNAL_TIME<tag>` track when the transition
//! occurred. We only expose the state int here; the journal editor doesn't need
//! timestamps.

use crate::parsers::gff::{GffParser, GffValue};
use crate::services::savegame_handler::SaveGameHandler;

/// Prefix the engine uses when recording journal progress on the PC's VarTable.
/// Everything after the prefix is the quest tag (tags may start with digits, so no
/// separator — strip the prefix verbatim).
const JOURNAL_ENTRY_PREFIX: &str = "NW_JOURNAL_ENTRY";

#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveJournalEntry {
    pub tag: String,
    pub state: u32,
}

/// Read the VarTable-backed live journal from the save's primary player slot.
/// `player_index` should match `SessionState::selected_player_index` / `primary_player_index`.
pub fn read_live_journal(
    handler: &SaveGameHandler,
    player_index: usize,
) -> Result<Vec<LiveJournalEntry>, String> {
    let playerlist = handler
        .extract_player_data()
        .map_err(|e| format!("Failed to extract playerlist.ifo: {e}"))?;

    let gff = GffParser::from_bytes(playerlist)
        .map_err(|e| format!("Failed to parse playerlist.ifo: {e}"))?;
    let root = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read playerlist.ifo root: {e}"))?;

    let players = match root.get("Mod_PlayerList") {
        Some(GffValue::List(lazy_structs)) => lazy_structs,
        _ => return Err("Mod_PlayerList missing or not a list".to_string()),
    };

    let entry = players
        .get(player_index)
        .ok_or_else(|| {
            format!(
                "player_index {player_index} out of range ({})",
                players.len()
            )
        })?
        .force_load();

    let Some(GffValue::List(var_table)) = entry.get("VarTable") else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for var_lazy in var_table {
        let fields = var_lazy.force_load();
        let Some(GffValue::String(name)) = fields.get("Name") else {
            continue;
        };
        let Some(tag) = name.strip_prefix(JOURNAL_ENTRY_PREFIX) else {
            continue;
        };
        if tag.is_empty() {
            continue;
        }
        let state = match fields.get("Value") {
            Some(GffValue::Int(v)) if *v >= 0 => *v as u32,
            Some(GffValue::Dword(v)) => *v,
            _ => continue,
        };
        out.push(LiveJournalEntry {
            tag: tag.to_string(),
            state,
        });
    }

    Ok(out)
}
