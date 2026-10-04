use std::path::PathBuf;

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use indexmap::IndexMap;

use super::apply::{self, EngineBuild};
use super::{compare, content, runner, slots};
use crate::character::{AbilityIndex, Character, ClassId, FeatId};
use crate::config::NWN2Paths;
use crate::parsers::gff::{GffValue, merge_fields_into_gff};
use crate::services::load_diagnostics::{LoadInput, LoadReport};
use crate::state::AppState;
use crate::utils::parsing::row_str;

pub const NAMES: &[&str] = &[
    "engine_check",
    "engine_level_up",
    "make_test_save",
    "list_test_saves",
    "delete_test_saves",
];

pub fn schemas() -> Vec<Value> {
    vec![
        json!({
            "name": "engine_check",
            "description": "Dev only. Compare the loaded character (unsaved edits included) with the game's own rules library \
                 (NWN2RulesLib64 via toolset-oracle, with the save's module, campaign, override, Workshop and HAK 2DAs): \
                 abilities, BAB, save parts, max HP, AC ability/dodge parts, skill max ranks and class skills for the \
                 levelling class, caster levels and spell slots. Items are stripped on both sides. Not modelled by the \
                 engine side: item effects, mod scripts, story feats a campaign grants on load. Set all=true for every row.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "all": { "type": "boolean", "description": "Return matching rows too" }
                }
            }
        }),
        json!({
            "name": "engine_level_up",
            "description": "Dev only. Level a copy of the loaded character up with both the game's rules library \
                 (LevelUpAutomatic) and the editor's level_up, then compare per level: skill points gained, max HP, \
                 BAB and class saves after the level, and whether every feat the editor grants automatically is among \
                 the feats the engine added. The engine's package picks the level's free choices; its ability \
                 increases and feat picks are copied to the editor side so later levels stay comparable. The session \
                 is not changed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "class_id": { "type": "integer" },
                    "levels": { "type": "integer", "description": "Default 1" },
                    "package": { "type": "integer", "description": "packages.2da row; default: the class's own package, else the character's starting package" }
                },
                "required": ["class_id"]
            }
        }),
        json!({
            "name": "make_test_save",
            "description": format!(
                "Dev only. Build a character with the game's own rules code (class packages, like the \
                 game's Recommended choices) and write it as the player into a copy of a template save \
                 (the save loaded in the editor, or any save slot) in slots {}, then load that copy \
                 into the editor session. Refused while the session has unsaved changes. The UI does \
                 not refresh by itself: reopen the save there to see it.",
                slots::test_range()
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "race": { "type": "integer", "description": "racialtypes.2da row" },
                    "subrace": { "type": "integer", "description": "racialsubtypes.2da row; default: racialtypes.2da DefaultSubRace" },
                    "classes": {
                        "type": "array",
                        "description": "Levels in order, e.g. [{\"class\":4,\"levels\":2},{\"class\":10,\"levels\":3}]",
                        "items": {
                            "type": "object",
                            "properties": {
                                "class": { "type": "integer" },
                                "levels": { "type": "integer" },
                                "package": { "type": "integer", "description": "packages.2da row; default: classes.2da Package" }
                            },
                            "required": ["class", "levels"]
                        }
                    },
                    "abilities": { "type": "array", "items": { "type": "integer" }, "description": "Base Str Dex Con Int Wis Cha before racial adjustments; default 14 14 14 14 12 10" },
                    "good_evil": { "type": "integer", "description": "0-100, default 50" },
                    "lawful_chaotic": { "type": "integer", "description": "0-100, default 50" },
                    "name": { "type": "string", "description": "First name and save name; default derived from race and classes" },
                    "template": { "type": "integer", "description": "Template save slot number; default: the save loaded in the editor" }
                },
                "required": ["race", "classes"]
            }
        }),
        json!({
            "name": "list_test_saves",
            "description": format!("Dev only. List generated test saves (slots {}).", slots::test_range()),
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "delete_test_saves",
            "description": format!(
                "Dev only. Delete generated test saves. Only slots {}, never the save loaded in the \
                 editor. Without slots: all test saves except the loaded one.",
                slots::test_range()
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slots": { "type": "array", "items": { "type": "integer" } }
                }
            }
        }),
    ]
}

pub async fn call(app: &AppHandle, name: &str, args: Value) -> Result<Value, String> {
    let state = app.state::<AppState>();
    match name {
        "engine_check" => engine_check(&state, &args).await,
        "engine_level_up" => engine_level_up(&state, &args).await,
        "make_test_save" => make_test_save(&state, args).await,
        "list_test_saves" | "delete_test_saves" => {
            let saves = saves_dir(&state)?;
            let loaded = state.session.read().save_dir.clone();
            let list = name == "list_test_saves";
            let wanted: Option<Vec<u32>> = args.get("slots").and_then(Value::as_array).map(|a| {
                a.iter()
                    .filter_map(Value::as_u64)
                    .map(|n| n as u32)
                    .collect()
            });
            tokio::task::spawn_blocking(move || {
                if list {
                    Ok(json!(slots::list(&saves, loaded.as_deref())?))
                } else {
                    let deleted = slots::delete(&saves, wanted.as_deref(), loaded.as_deref())?;
                    Ok(json!({ "deleted": deleted }))
                }
            })
            .await
            .map_err(|e| e.to_string())?
        }
        _ => Err(format!("unknown dev tool: {name}")),
    }
}

async fn make_test_save(state: &AppState, args: Value) -> Result<Value, String> {
    ensure_game_data_ready(state)?;
    if state.session.read().has_unsaved_changes() {
        return Err("the editor session has unsaved changes; save or discard them first".into());
    }
    let (paths, install) = oracle_paths(state)?;
    let saves = saves_dir(state)?;
    let template = match args.get("template").and_then(Value::as_u64) {
        Some(slot) => slots::find_template(&saves, slot as u32)?,
        None => state
            .session
            .read()
            .save_dir
            .clone()
            .ok_or("no save loaded in the editor; pass template (a save slot number)")?,
    };

    let mut request = copy_args(
        &args,
        &[
            "race",
            "subrace",
            "classes",
            "abilities",
            "good_evil",
            "lawful_chaotic",
        ],
    );
    if request.get("subrace").is_none() {
        let race = args
            .get("race")
            .and_then(Value::as_i64)
            .ok_or("race is required")? as i32;
        let subrace = Character::default_subrace_for_race(race, &state.game_data.read())
            .ok_or_else(|| format!("race {race} has no subrace; pass subrace"))?;
        request["subrace"] = json!(subrace);
    }
    let raw = run_oracle(
        paths,
        install,
        Some(template.clone()),
        None,
        "build",
        request,
    )
    .await?;
    let build: EngineBuild = serde_json::from_value(raw.clone())
        .map_err(|e| format!("unexpected oracle build result: {e}"))?;

    let name = match args.get("name").and_then(Value::as_str) {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => default_name(state, &build),
    };
    let folder = {
        let (saves, template, name) = (saves.clone(), template.clone(), name.clone());
        tokio::task::spawn_blocking(move || slots::create(&saves, &template, &name))
            .await
            .map_err(|e| e.to_string())??
    };

    let folder_str = folder.to_string_lossy().into_owned();
    let mut report = LoadReport::new(LoadInput {
        file_path: folder_str.clone(),
        player_index: None,
        file_size: None,
    });
    {
        let mut session = state.session.write();
        if session.has_unsaved_changes() {
            return Err(format!(
                "copied to {folder_str}, but the editor session got unsaved changes meanwhile, so it was not loaded"
            ));
        }
        session
            .load_character(&folder_str, None, false, &mut report)
            .map_err(|e| {
                format!(
                    "copied to {folder_str} but loading it failed: {}",
                    e.message
                )
            })?;
    }
    crate::commands::session::sync_module_content(state).await;

    {
        let game_data = state.game_data.read();
        let mut session = state.session.write();
        let character = session
            .character_mut()
            .ok_or("the copied save has no player character")?;
        apply::apply(character, &build, &game_data, &name);
        session.stage_save_name(&name)?;
        session
            .save_character(&game_data, 0)
            .map_err(|e| format!("writing {folder_str} failed: {e}"))?;
    }

    Ok(json!({
        "save": folder_str,
        "name": name,
        "template": template.to_string_lossy(),
        "not_modelled": "Campaigns can re-grant story feats on load (the OC: Blessed of Waukeen +1 all saves, epithets worth +10 max HP), so in-game saves and max HP can read higher than the engine build below. Item effects and mod scripts are not modelled either.",
        "engine": raw,
    }))
}

type Fields = IndexMap<String, GffValue<'static>>;

/// The loaded character without items (the engine does not model them), the save it came
/// from and the install, for the oracle ops that read the loaded character.
struct Loaded {
    fields: Fields,
    save_dir: Option<PathBuf>,
    paths: NWN2Paths,
    install: PathBuf,
}

impl Loaded {
    async fn run(self, op: &'static str, request: Value) -> Result<Value, String> {
        run_oracle(
            self.paths,
            self.install,
            self.save_dir,
            Some(self.fields),
            op,
            request,
        )
        .await
    }
}

fn loaded_without_items(state: &AppState) -> Result<Loaded, String> {
    ensure_game_data_ready(state)?;
    let (paths, install) = oracle_paths(state)?;
    let session = state.session.read();
    let character = session.character().ok_or("no character loaded")?;
    let fields: Fields = character
        .gff()
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "Equip_ItemList" | "ItemList"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Ok(Loaded {
        fields,
        save_dir: session.save_dir.clone(),
        paths,
        install,
    })
}

/// Adds the save's content layers (and `bic`, written to a temp `.bic`) to `request` and runs
/// `op` in the oracle.
async fn run_oracle(
    paths: NWN2Paths,
    install: PathBuf,
    save_dir: Option<PathBuf>,
    bic: Option<Fields>,
    op: &'static str,
    mut request: Value,
) -> Result<Value, String> {
    tokio::task::spawn_blocking(move || {
        let scratch = tempfile::tempdir().map_err(|e| e.to_string())?;
        if let Some(fields) = bic {
            let path = scratch.path().join("oracle.bic");
            let bytes = merge_fields_into_gff(None, &fields, "BIC ", false)?;
            std::fs::write(&path, bytes).map_err(|e| format!("failed to write temp .bic: {e}"))?;
            request["bic"] = json!(path);
        }
        request["content"] = Value::Array(content::layers(
            &paths,
            save_dir.as_deref(),
            scratch.path(),
        )?);
        runner::run(&install, op, &request)
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn engine_level_up(state: &AppState, args: &Value) -> Result<Value, String> {
    let class_id = args
        .get("class_id")
        .and_then(Value::as_i64)
        .ok_or("class_id is required")? as i32;
    let levels = args
        .get("levels")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .clamp(1, 40) as i32;
    let loaded = loaded_without_items(state)?;
    let mut character = Character::from_gff(loaded.fields.clone()).map_err(|e| e.to_string())?;

    let mut request = copy_args(args, &["package"]);
    request["class"] = json!(class_id);
    request["levels"] = json!(levels);
    let raw = loaded.run("level_up", request).await?;
    let engine: compare::EngineLevelUp = serde_json::from_value(raw)
        .map_err(|e| format!("unexpected oracle level_up result: {e}"))?;

    let mut steps = Vec::new();
    {
        let game_data = state.game_data.read();
        let session = state.session.read();
        let decoder = &session.item_property_decoder;
        for step in &engine.steps {
            let result = character
                .level_up(ClassId(class_id), &game_data)
                .map_err(|e| format!("editor level_up failed at level {}: {e}", step.level))?;
            if let Ok(index) = u8::try_from(step.ability_increase) {
                let ability = AbilityIndex(index);
                character
                    .set_ability(ability, character.base_ability(ability) + 1)
                    .map_err(|e| e.to_string())?;
            }
            for feat in &step.feats_added {
                if !character.has_feat(FeatId(*feat)) {
                    character
                        .add_feat(FeatId(*feat))
                        .map_err(|e| format!("copying engine feat {feat} failed: {e}"))?;
                }
            }
            let saves = character.get_save_summary(&game_data, decoder).saves;
            steps.push(compare::level_step(
                step,
                &result,
                character.max_hp(),
                character.calculate_bab(&game_data),
                [saves.fortitude.base, saves.reflex.base, saves.will.base],
            ));
        }
    }

    let mismatches: usize = steps.iter().map(|s| s.mismatches.len()).sum();
    Ok(json!({
        "package": engine.package,
        "mismatches": mismatches,
        "levels": steps,
        "not_modelled": "Items are stripped. The engine's package makes the free choices (copied to the editor side before comparing). Mod level-up scripts do not run.",
    }))
}

async fn engine_check(state: &AppState, args: &Value) -> Result<Value, String> {
    let loaded = loaded_without_items(state)?;
    let stripped = Character::from_gff(loaded.fields.clone()).map_err(|e| e.to_string())?;
    let levelling_class = stripped
        .levelling_class()
        .ok_or("the character has no class")?;
    let editor = {
        let game_data = state.game_data.read();
        let session = state.session.read();
        let decoder = &session.item_property_decoder;
        let spells = stripped.get_spells_state(&game_data, decoder);
        let casters = spells
            .spellcasting_classes
            .iter()
            .map(|c| compare::EditorCaster {
                class: c.class_id.0,
                caster_level: c.caster_level,
                slots: spells
                    .spell_summary
                    .caster_classes
                    .iter()
                    .find(|s| s.id == c.class_id)
                    .map(|s| {
                        (0..10)
                            .map(|l| s.slots_by_level.get(&l).copied().unwrap_or(0))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect();
        compare::EditorSide {
            abilities: stripped.get_abilities_state(&game_data, decoder),
            saves: stripped.get_save_summary(&game_data, decoder),
            combat: stripped.get_combat_summary(&game_data, decoder),
            skills: stripped.get_skill_summary(&game_data, Some(decoder)),
            casters,
            max_hp: stripped.max_hp(),
            base_hp: stripped.base_hp(),
        }
    };

    let raw = loaded
        .run("check", json!({ "levelling_class": levelling_class.0 }))
        .await?;
    let engine: compare::EngineCheck =
        serde_json::from_value(raw).map_err(|e| format!("unexpected oracle check result: {e}"))?;

    let rows = compare::rows(&editor, &engine);
    let checked = rows.len();
    let (mismatches, matching): (Vec<_>, Vec<_>) =
        rows.into_iter().partition(|r| r.editor != r.engine);
    let mut out = json!({
        "levelling_class": levelling_class.0,
        "checked": checked,
        "mismatches": mismatches,
        "not_modelled": "Items are stripped on both sides. The engine side does not know mod scripts or story feats a campaign grants on load; hp.max compares the stored MaxHitPoints, which the game rewrites on load.",
    });
    if args.get("all").and_then(Value::as_bool).unwrap_or(false) {
        out["matching"] = json!(matching);
    }
    Ok(out)
}

/// The non-null `keys` of `args`, as an oracle request.
fn copy_args(args: &Value, keys: &[&str]) -> Value {
    let mut request = serde_json::Map::new();
    for &key in keys {
        if let Some(v) = args.get(key).filter(|v| !v.is_null()) {
            request.insert(key.to_string(), v.clone());
        }
    }
    Value::Object(request)
}

fn default_name(state: &AppState, build: &EngineBuild) -> String {
    let game_data = state.game_data.read();
    let label = |table: &str, id: i32| {
        game_data
            .get_table(table)
            .and_then(|t| t.get_by_id(id))
            .and_then(|row| row_str(&row, "label"))
            .unwrap_or_else(|| id.to_string())
    };
    let classes = build
        .classes
        .iter()
        .map(|c| format!("{} {}", label("classes", c.class), c.level))
        .collect::<Vec<_>>()
        .join(" ");
    format!("TEST {} {classes}", label("racialsubtypes", build.subrace))
}

/// Editor-side values and `calculate_xp_for_level` read game data, which is empty until
/// startup initialization (or a module content rebuild) finishes.
fn ensure_game_data_ready(state: &AppState) -> Result<(), String> {
    if state.init_status.read().step == "ready" {
        Ok(())
    } else {
        Err("game data is still loading; try again in a moment".into())
    }
}

fn oracle_paths(state: &AppState) -> Result<(NWN2Paths, PathBuf), String> {
    let paths = state.paths.read().clone();
    let install = paths
        .game_folder()
        .cloned()
        .filter(|_| paths.is_enhanced_edition())
        .ok_or("the engine oracle needs the Enhanced Edition install")?;
    Ok((paths, install))
}

fn saves_dir(state: &AppState) -> Result<PathBuf, String> {
    state
        .paths
        .read()
        .saves()
        .ok_or_else(|| "Documents folder not set".to_string())
}
