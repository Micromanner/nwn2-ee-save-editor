use crate::character::types::{ClassId, SpellId};
use crate::character::{
    MemorizedSpellRaw, SpellDetails, SpellSummary, is_displayable_spell, is_mod_prefixed_name,
};
use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellAction {
    Learned,
    Forgotten,
    Prepared,
    Unprepared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellChangeResult {
    pub spell_id: SpellId,
    pub action: SpellAction,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellSlots {
    pub class_id: ClassId,
    pub slots_by_level: HashMap<i32, i32>,
}

#[tauri::command]
pub async fn is_spellcaster(state: State<'_, AppState>, class_id: i32) -> CommandResult<bool> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.is_spellcaster(ClassId(class_id), &game_data))
}

#[tauri::command]
pub async fn get_spell_summary(state: State<'_, AppState>) -> CommandResult<SpellSummary> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_spells_state(&game_data).spell_summary)
}

#[tauri::command]
pub async fn get_known_spells(
    state: State<'_, AppState>,
    class_id: i32,
    spell_level: i32,
) -> CommandResult<Vec<SpellId>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_all_known_spells(ClassId(class_id), spell_level, &game_data))
}

#[tauri::command]
pub fn get_class_available_spells(
    state: State<'_, AppState>,
    class_id: i32,
    spell_level: i32,
) -> CommandResult<Vec<SpellId>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_spells_available_to_class(ClassId(class_id), spell_level, &game_data))
}

#[tauri::command]
pub fn calculate_metamagic_cost(state: State<'_, AppState>, metamagic: u8) -> CommandResult<i32> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.calculate_metamagic_cost(metamagic, &game_data))
}

#[tauri::command]
pub async fn get_memorized_spells(
    state: State<'_, AppState>,
    class_id: i32,
    spell_level: i32,
) -> CommandResult<Vec<MemorizedSpellRaw>> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.memorized_spells(ClassId(class_id), spell_level))
}

#[tauri::command]
pub async fn get_domain_spells(
    state: State<'_, AppState>,
    class_id: i32,
) -> CommandResult<HashMap<i32, Vec<SpellId>>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_domain_spells(ClassId(class_id), &game_data))
}

#[tauri::command]
pub async fn get_spell_details(
    state: State<'_, AppState>,
    spell_id: i32,
) -> CommandResult<SpellDetails> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    character
        .get_spell_details(SpellId(spell_id), &game_data)
        .ok_or_else(|| CommandError::NotFound {
            item: format!("Spell {spell_id}"),
        })
}

#[tauri::command]
pub async fn get_max_castable_spell_level(
    state: State<'_, AppState>,
    class_id: i32,
) -> CommandResult<i32> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let slots = character.calculate_spell_slots(ClassId(class_id), &game_data);

    for level in (0..=9).rev() {
        if slots.get(level as usize).copied().unwrap_or(0) > 0 {
            return Ok(level);
        }
    }
    Ok(0)
}

#[tauri::command]
pub async fn calculate_spell_slots(
    state: State<'_, AppState>,
) -> CommandResult<HashMap<ClassId, SpellSlots>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    let mut result = HashMap::new();
    for class_entry in character.class_entries() {
        if !character.is_spellcaster(class_entry.class_id, &game_data) {
            continue;
        }

        let slots_vec = character.calculate_spell_slots(class_entry.class_id, &game_data);
        let mut slots_by_level = HashMap::new();
        for (level, count) in slots_vec.iter().enumerate() {
            if *count > 0 {
                slots_by_level.insert(level as i32, *count);
            }
        }

        result.insert(
            class_entry.class_id,
            SpellSlots {
                class_id: class_entry.class_id,
                slots_by_level,
            },
        );
    }

    Ok(result)
}

#[tauri::command]
pub async fn add_known_spell(
    state: State<'_, AppState>,
    class_index: usize,
    spell_level: i32,
    spell_id: i32,
) -> CommandResult<SpellChangeResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let (spell_label, class_label) = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        let spell_label = character.get_spell_name(SpellId(spell_id), &game_data);
        let class_label = character
            .class_entries()
            .get(class_index)
            .map(|e| character.get_class_name(e.class_id, &game_data))
            .unwrap_or_else(|| format!("#{class_index}"));
        (spell_label, class_label)
    };
    session.record_history(format!("Learn {spell_label} ({class_label})"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    match character.add_known_spell(class_index, spell_level, SpellId(spell_id)) {
        Ok(()) => Ok(SpellChangeResult {
            spell_id: SpellId(spell_id),
            action: SpellAction::Learned,
            success: true,
            message: "Spell added successfully".to_string(),
        }),
        Err(e) => Ok(SpellChangeResult {
            spell_id: SpellId(spell_id),
            action: SpellAction::Learned,
            success: false,
            message: e.to_string(),
        }),
    }
}

#[tauri::command]
pub async fn remove_known_spell(
    state: State<'_, AppState>,
    class_index: usize,
    spell_level: i32,
    spell_id: i32,
) -> CommandResult<SpellChangeResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let (spell_label, class_label) = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        let spell_label = character.get_spell_name(SpellId(spell_id), &game_data);
        let class_label = character
            .class_entries()
            .get(class_index)
            .map(|e| character.get_class_name(e.class_id, &game_data))
            .unwrap_or_else(|| format!("#{class_index}"));
        (spell_label, class_label)
    };
    session.record_history(format!("Forget {spell_label} ({class_label})"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    match character.remove_known_spell(class_index, spell_level, SpellId(spell_id)) {
        Ok(()) => Ok(SpellChangeResult {
            spell_id: SpellId(spell_id),
            action: SpellAction::Forgotten,
            success: true,
            message: "Spell removed successfully".to_string(),
        }),
        Err(e) => Ok(SpellChangeResult {
            spell_id: SpellId(spell_id),
            action: SpellAction::Forgotten,
            success: false,
            message: e.to_string(),
        }),
    }
}

#[tauri::command]
pub async fn prepare_spell(
    state: State<'_, AppState>,
    class_index: usize,
    spell_level: i32,
    spell_id: i32,
    metamagic: u8,
    ready: bool,
) -> CommandResult<SpellChangeResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let spell_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_spell_name(SpellId(spell_id), &game_data)
    };
    session.record_history(format!("Prepare {spell_label}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    let spell = MemorizedSpellRaw {
        spell_id: SpellId(spell_id),
        meta_magic: metamagic,
        ready,
        is_domain: false,
    };

    match character.add_memorized_spell(class_index, spell_level, spell) {
        Ok(()) => Ok(SpellChangeResult {
            spell_id: SpellId(spell_id),
            action: SpellAction::Prepared,
            success: true,
            message: "Spell prepared successfully".to_string(),
        }),
        Err(e) => Ok(SpellChangeResult {
            spell_id: SpellId(spell_id),
            action: SpellAction::Prepared,
            success: false,
            message: e.to_string(),
        }),
    }
}

#[tauri::command]
pub async fn clear_memorized_spells(
    state: State<'_, AppState>,
    class_index: usize,
    spell_level: Option<i32>,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let class_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character
            .class_entries()
            .get(class_index)
            .map(|e| character.get_class_name(e.class_id, &game_data))
            .unwrap_or_else(|| format!("#{class_index}"))
    };
    session.record_history(format!("Clear prepared spells: {class_label}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    match spell_level {
        Some(level) => character
            .clear_memorized_spells(class_index, level)
            .map_err(CommandError::from),
        None => character
            .clear_all_memorized_spells(class_index)
            .map_err(CommandError::from),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableSpellInfo {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub school_id: Option<i32>,
    pub school_name: Option<String>,
    pub level: i32,
    pub available_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableSpellsResponse {
    pub spells: Vec<AvailableSpellInfo>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: i32,
    pub limit: i32,
    pub total: i32,
    pub pages: i32,
    pub has_next: bool,
    pub has_previous: bool,
}

const LEARNABLE_CLASS_COLUMNS: &[(&str, &str)] = &[
    ("bard", "Bard"),
    ("cleric", "Cleric"),
    ("druid", "Druid"),
    ("paladin", "Paladin"),
    ("ranger", "Ranger"),
    ("wiz_sorc", "Wizard/Sorcerer"),
    ("favsoul", "Favored Soul"),
    ("spiritshm", "Spirit Shaman"),
];

#[tauri::command]
pub async fn get_character_available_spells(
    state: State<'_, AppState>,
    page: Option<i32>,
    limit: Option<i32>,
    class_id: Option<i32>,
    spell_level: Option<i32>,
    school_ids: Option<Vec<i32>>,
    search: Option<String>,
    show_all: Option<bool>,
) -> CommandResult<AvailableSpellsResponse> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    let page = page.unwrap_or(1).max(1);
    let limit = limit.unwrap_or(50).clamp(10, 10000);
    let show_all = show_all.unwrap_or(false);

    let spells_table = game_data
        .get_table("spells")
        .ok_or_else(|| CommandError::NotFound {
            item: "spells table".to_string(),
        })?;

    // Get all spellcasting classes for this character
    let class_columns: Vec<(ClassId, String, String)> = character
        .class_entries()
        .iter()
        .filter(|ce| character.is_spellcaster(ce.class_id, &game_data))
        .filter_map(|ce| {
            let col = character.get_spell_table_column_for_class(ce.class_id, &game_data)?;
            let name = character.get_class_name(ce.class_id, &game_data);
            Some((ce.class_id, col.to_lowercase(), name))
        })
        .collect();

    if !show_all && class_columns.is_empty() {
        return Ok(AvailableSpellsResponse {
            spells: vec![],
            pagination: PaginationInfo {
                page,
                limit,
                total: 0,
                pages: 0,
                has_next: false,
                has_previous: false,
            },
        });
    }

    // Collect all available spells
    let mut all_spells: Vec<AvailableSpellInfo> = Vec::new();
    let search_lower = search.as_ref().map(|s| s.to_lowercase());

    for row_id in 0..spells_table.row_count() {
        let Ok(spell_row) = spells_table.get_row(row_id) else {
            continue;
        };

        if !is_displayable_spell(&spell_row) {
            continue;
        }
        // Only include spells on a traditional caster class list - excludes Warlock
        // invocations, Stormlord abilities, and other class features that only appear
        // in Warlock/Innate columns
        let on_caster_list = LEARNABLE_CLASS_COLUMNS.iter().any(|&(col, _)| {
            spell_row
                .get(col)
                .and_then(|v| v.as_ref())
                .is_some_and(|v| {
                    !v.is_empty() && v != "****" && v.parse::<i32>().is_ok_and(|n| n >= 0)
                })
        });
        if !on_caster_list {
            continue;
        }

        // Check which classes can cast this spell and at what level
        let mut available_classes: Vec<String> = Vec::new();
        let mut found_level: Option<i32> = None;

        if show_all {
            for &(col, class_name) in LEARNABLE_CLASS_COLUMNS {
                if let Some(level_str) = spell_row.get(col).and_then(|v| v.as_ref())
                    && let Ok(lvl) = level_str.parse::<i32>()
                    && lvl >= 0
                {
                    if let Some(filter_level) = spell_level
                        && lvl != filter_level
                    {
                        continue;
                    }
                    available_classes.push(class_name.to_string());
                    if found_level.is_none() {
                        found_level = Some(lvl);
                    }
                }
            }
        } else {
            for (cid, col, class_name) in &class_columns {
                if let Some(filter_class) = class_id
                    && cid.0 != filter_class
                {
                    continue;
                }

                if let Some(level_str) = spell_row.get(col).and_then(|v| v.as_ref())
                    && let Ok(lvl) = level_str.parse::<i32>()
                    && lvl >= 0
                {
                    if let Some(filter_level) = spell_level
                        && lvl != filter_level
                    {
                        continue;
                    }
                    available_classes.push(class_name.clone());
                    if found_level.is_none() {
                        found_level = Some(lvl);
                    }
                }
            }
        }

        if available_classes.is_empty() {
            continue;
        }

        // Get spell name
        let name = spell_row
            .get("name")
            .and_then(|v| v.as_ref())
            .and_then(|name_raw| {
                if let Ok(strref) = name_raw.parse::<i32>() {
                    game_data.get_string(strref)
                } else if !name_raw.is_empty() {
                    Some(name_raw.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| format!("Spell {row_id}"));

        if is_mod_prefixed_name(&name) {
            continue;
        }

        // Apply search filter
        if let Some(ref search_lower) = search_lower
            && !name.to_lowercase().contains(search_lower)
        {
            continue;
        }

        let icon = spell_row
            .get("iconresref")
            .and_then(|v| v.as_ref())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| "io_unknown".to_string());

        let school_raw = spell_row.get("school").and_then(|v| v.as_ref());
        let school_id = school_raw.and_then(|s| {
            if let Some(c) = s.chars().next() {
                match c.to_ascii_uppercase() {
                    'G' => Some(0),
                    'A' => Some(1),
                    'C' => Some(2),
                    'D' => Some(3),
                    'E' => Some(4),
                    'V' => Some(5),
                    'I' => Some(6),
                    'N' => Some(7),
                    'T' => Some(8),
                    _ => s.parse().ok(),
                }
            } else {
                None
            }
        });

        if let Some(ref filter_schools) = school_ids
            && !filter_schools.is_empty()
        {
            if let Some(sid) = school_id {
                if !filter_schools.contains(&sid) {
                    continue;
                }
            } else {
                continue;
            }
        }

        let school_name = school_id.and_then(|id| {
            game_data
                .get_table("spellschools")
                .and_then(|t| t.get_by_id(id))
                .and_then(|row| {
                    row.get("label")
                        .and_then(|v| v.as_ref())
                        .map(std::string::ToString::to_string)
                })
        });

        let description = spell_row
            .get("spelldesc")
            .and_then(|v| v.as_ref())
            .and_then(|desc_raw| {
                if let Ok(strref) = desc_raw.parse::<i32>() {
                    game_data.get_string(strref).filter(|s| !s.is_empty())
                } else if !desc_raw.is_empty() {
                    Some(desc_raw.clone())
                } else {
                    None
                }
            });

        all_spells.push(AvailableSpellInfo {
            id: row_id as i32,
            name,
            description,
            icon,
            school_id,
            school_name,
            level: found_level.unwrap_or(0),
            available_classes,
        });
    }

    // Sort by name
    all_spells.sort_by(|a, b| a.name.cmp(&b.name));

    // Paginate
    let total = all_spells.len() as i32;
    let pages = (total + limit - 1) / limit;
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(all_spells.len());

    let spells = if start < all_spells.len() {
        all_spells[start..end].to_vec()
    } else {
        vec![]
    };

    Ok(AvailableSpellsResponse {
        spells,
        pagination: PaginationInfo {
            page,
            limit,
            total,
            pages,
            has_next: page < pages,
            has_previous: page > 1,
        },
    })
}

#[tauri::command]
pub async fn get_character_ability_spells(
    state: State<'_, AppState>,
) -> CommandResult<Vec<crate::character::AbilitySpellEntry>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_ability_spells(&game_data))
}
