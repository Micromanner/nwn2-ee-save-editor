use crate::character::types::{ClassId, SpellId};
use crate::character::{Character, is_displayable_spell, is_mod_prefixed_name};
use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

fn spell_edit_label(
    character: &crate::character::Character,
    game_data: &crate::loaders::GameData,
    verb: &str,
    class_index: usize,
    spell_id: i32,
) -> String {
    let spell_label = character.get_spell_name(SpellId(spell_id), game_data);
    let class_label = character
        .class_entries()
        .get(class_index)
        .map(|e| character.get_class_name(e.class_id, game_data))
        .unwrap_or_else(|| format!("#{class_index}"));
    format!("{verb} {spell_label} ({class_label})")
}

#[tauri::command]
pub async fn add_known_spell(
    state: State<'_, AppState>,
    class_index: usize,
    spell_id: i32,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let label = spell_edit_label(
        session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?,
        &game_data,
        "Learn",
        class_index,
        spell_id,
    );
    session.record_edit(label, |character| {
        let spell_level = character
            .spell_level_for_class_index(class_index, SpellId(spell_id), &game_data)
            .ok_or_else(|| CommandError::InvalidValue {
                field: "spell_id".to_string(),
                expected: "a spell on the class's spell list".to_string(),
                actual: spell_id.to_string(),
            })?;
        Ok(character.add_known_spell(class_index, spell_level, SpellId(spell_id), &game_data)?)
    })
}

#[tauri::command]
pub async fn remove_known_spell(
    state: State<'_, AppState>,
    class_index: usize,
    spell_level: i32,
    spell_id: i32,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let label = spell_edit_label(
        session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?,
        &game_data,
        "Forget",
        class_index,
        spell_id,
    );
    session.record_edit(label, |character| {
        Ok(
            character.remove_known_spell(
                class_index,
                spell_level,
                SpellId(spell_id),
                &game_data,
            )?,
        )
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellClassLevel {
    pub class_id: i32,
    pub class_name: String,
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableSpellInfo {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub school_id: Option<i32>,
    pub school_name: Option<String>,
    pub class_levels: Vec<SpellClassLevel>,
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

#[tauri::command]
pub async fn get_character_available_spells(
    state: State<'_, AppState>,
    page: Option<i32>,
    limit: Option<i32>,
    class_id: Option<i32>,
    spell_level: Option<i32>,
    school_ids: Option<Vec<i32>>,
    search: Option<String>,
) -> CommandResult<AvailableSpellsResponse> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    let page = page.unwrap_or(1).max(1);
    let limit = limit.unwrap_or(50).clamp(10, 10000);

    let spells_table = game_data
        .get_table("spells")
        .ok_or_else(|| CommandError::NotFound {
            item: "spells table".to_string(),
        })?;

    let learnable_class_labels = character.learnable_spell_class_labels(&game_data);

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

    if class_columns.is_empty() {
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
        let on_caster_list = learnable_class_labels
            .iter()
            .any(|(col, _)| Character::spell_level_in_row(&spell_row, col).is_some());
        if !on_caster_list {
            continue;
        }

        let mut class_levels: Vec<SpellClassLevel> = Vec::new();
        for (cid, col, class_name) in &class_columns {
            if let Some(filter_class) = class_id
                && cid.0 != filter_class
            {
                continue;
            }

            if let Some(lvl) = Character::spell_level_in_row(&spell_row, col) {
                if let Some(filter_level) = spell_level
                    && lvl != filter_level
                {
                    continue;
                }
                class_levels.push(SpellClassLevel {
                    class_id: cid.0,
                    class_name: class_name.clone(),
                    level: lvl,
                });
            }
        }

        if class_levels.is_empty() {
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
            class_levels,
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
