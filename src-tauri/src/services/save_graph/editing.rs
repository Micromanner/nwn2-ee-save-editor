use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{SaveGraph, TransitionNode, read_live_journal};
use crate::config::NWN2Paths;
use crate::parsers::gff::{GffParser, GffValue, GffWriter};
use crate::services::campaign::{CampaignManager, content};
use crate::services::savegame_handler::SaveGameHandler;
use crate::services::toolset_bridge::ConvoFunctor;

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VariableKey {
    pub module: Option<String>,
    pub name: String,
    pub value_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableWrite {
    pub key: VariableKey,
    pub saved: Option<Value>,
    pub before: Option<Value>,
    pub after: Value,
    pub action_index: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionEffect {
    pub action: ConvoFunctor,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestPreview {
    pub revision: String,
    pub tag: String,
    pub target: u32,
    pub saved_state: Option<u32>,
    pub before_state: Option<u32>,
    pub route: Option<usize>,
    pub writes: Vec<VariableWrite>,
    pub effects: Vec<ActionEffect>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingQuestEdit {
    pub tag: String,
    pub target: u32,
    pub saved_state: Option<u32>,
    pub source: Option<TransitionNode>,
    pub writes: Vec<VariableWrite>,
}

pub struct QuestEdits {
    generation: u64,
    pub pending: Vec<PendingQuestEdit>,
}

impl Default for QuestEdits {
    fn default() -> Self {
        Self {
            generation: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
            pending: Vec::new(),
        }
    }
}

impl QuestEdits {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn discard(&mut self, tag: Option<&str>) {
        self.pending.retain(|p| tag.is_some_and(|tag| p.tag != tag));
        self.generation = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    }

    pub fn preview(
        &self,
        graph: &SaveGraph,
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        player: usize,
        tag: &str,
        target: u32,
        route: Option<usize>,
    ) -> Result<QuestPreview, String> {
        let quest = graph
            .quests
            .iter()
            .find(|q| q.tag == tag)
            .ok_or("Quest is no longer available")?;
        let bytes = handler.extract_player_data().map_err(|e| e.to_string())?;
        // Validate storage and range before offering Apply, including absent entries.
        write_journal(&bytes, player, &[(tag.to_string(), target)])?;
        let saved_state = read_live_journal(handler, player)?
            .iter()
            .find(|q| q.tag == tag)
            .map(|q| q.state);
        let before_state = self
            .pending
            .iter()
            .find(|p| p.tag == tag)
            .map(|p| p.target)
            .or(saved_state);
        let mut result = QuestPreview {
            revision: String::new(),
            tag: tag.into(),
            target,
            saved_state,
            before_state,
            route,
            writes: Vec::new(),
            effects: Vec::new(),
        };
        let mut reader = VariableReader::new(handler, paths);
        if let Some(index) = route {
            let tr = quest
                .transitions
                .get(index)
                .filter(|tr| tr.new_state == target)
                .ok_or("The selected route does not reach this state")?;
            let mut candidates = Vec::new();
            for (action_index, action) in tr.actions.iter().enumerate() {
                match decode_assignment(action, &tr.module) {
                    Ok((key, after)) => match reader.read(&key) {
                        Ok(saved) => {
                            let before = self
                                .pending
                                .iter()
                                .flat_map(|p| &p.writes)
                                .rfind(|w| w.key == key)
                                .map(|w| w.after.clone())
                                .or(saved.clone());
                            candidates.push(VariableWrite {
                                key,
                                saved,
                                before,
                                after,
                                action_index,
                            });
                        }
                        Err(_) => result.effects.push(ActionEffect {
                            action: action.clone(),
                            reason: "unavailable_target".into(),
                        }),
                    },
                    Err(reason) => result.effects.push(ActionEffect {
                        action: action.clone(),
                        reason: reason.into(),
                    }),
                }
            }
            // Multiple writes to one variable have ordering semantics; don't offer a subset.
            for candidate in &candidates {
                if candidates.iter().filter(|w| w.key == candidate.key).count() == 1 {
                    result.writes.push(candidate.clone());
                } else {
                    result.effects.push(ActionEffect {
                        action: tr.actions[candidate.action_index].clone(),
                        reason: "multiple_assignments".into(),
                    });
                }
            }
        }
        let mut hash = Sha256::new();
        hash.update(self.generation.to_le_bytes());
        hash.update(&bytes);
        hash.update(serde_json::to_vec(&result).map_err(|e| e.to_string())?);
        result.revision = format!("{:x}", hash.finalize());
        Ok(result)
    }

    pub fn stage(
        &mut self,
        preview: QuestPreview,
        graph: &SaveGraph,
        selected: &[usize],
        replace: bool,
    ) -> Result<(), String> {
        let indices: BTreeSet<_> = selected.iter().copied().collect();
        if indices.iter().any(|i| *i >= preview.writes.len()) {
            return Err("Invalid selected assignment".into());
        }
        let writes: Vec<_> = indices
            .into_iter()
            .map(|i| preview.writes[i].clone())
            .collect();
        let conflicts: BTreeSet<String> = self
            .pending
            .iter()
            .filter(|p| {
                p.tag != preview.tag
                    && p.writes.iter().any(|old| {
                        writes
                            .iter()
                            .any(|new| old.key == new.key && old.after != new.after)
                    })
            })
            .map(|p| p.tag.clone())
            .collect();
        if !replace && !conflicts.is_empty() {
            return Err(format!(
                "Conflicting pending quests: {}. Review and choose replacement.",
                conflicts.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        self.pending
            .retain(|p| p.tag != preview.tag && !conflicts.contains(&p.tag));
        let source = preview.route.and_then(|i| {
            graph
                .quests
                .iter()
                .find(|q| q.tag == preview.tag)?
                .transitions
                .get(i)
                .cloned()
        });
        self.pending.push(PendingQuestEdit {
            tag: preview.tag,
            target: preview.target,
            saved_state: preview.saved_state,
            source,
            writes,
        });
        self.generation = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn validate_saved(
        &self,
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        player: usize,
    ) -> Result<(), String> {
        let journal = read_live_journal(handler, player)?;
        let mut reader = VariableReader::new(handler, paths);
        for edit in &self.pending {
            if journal.iter().find(|j| j.tag == edit.tag).map(|j| j.state) != edit.saved_state {
                return Err(format!(
                    "Saved quest {} changed. Discard and review its pending edit again.",
                    edit.tag
                ));
            }
            for write in &edit.writes {
                if reader.read(&write.key)? != write.saved {
                    return Err(format!(
                        "Saved variable {} changed. Discard and review quest {} again.",
                        write.key.name, edit.tag
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn flush(&self, handler: &mut SaveGameHandler, player: usize) -> Result<(), String> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let journal: Vec<_> = self
            .pending
            .iter()
            .map(|p| (p.tag.clone(), p.target))
            .collect();
        let bytes = handler.extract_player_data().map_err(|e| e.to_string())?;
        let bytes = write_journal(&bytes, player, &journal)?;
        handler
            .update_file("playerlist.ifo", &bytes)
            .map_err(|e| e.to_string())?;
        let mut globals = Vec::new();
        let mut modules: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
        for write in self.pending.iter().flat_map(|p| &p.writes) {
            let value = write
                .after
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| write.after.to_string());
            let update = (write.key.name.clone(), value, write.key.value_type.clone());
            if let Some(module) = &write.key.module {
                modules.entry(module.clone()).or_default().push(update);
            } else {
                globals.push(update);
            }
        }
        if !globals.is_empty() {
            let xml = handler.extract_globals_xml().map_err(|e| e.to_string())?;
            let mut parser = crate::parsers::xml::RustXmlParser::from_string(&xml)?;
            for (name, value, kind) in globals {
                match kind.as_str() {
                    "int" => {
                        parser
                            .data
                            .integers
                            .insert(name, value.parse::<i32>().map_err(|e| e.to_string())?);
                    }
                    "float" => {
                        parser
                            .data
                            .floats
                            .insert(name, value.parse::<f32>().map_err(|e| e.to_string())?);
                    }
                    "string" => {
                        parser.data.strings.insert(name, value);
                    }
                    _ => return Err("Unsupported assignment type".into()),
                }
            }
            handler
                .update_file("globals.xml", parser.to_xml_string()?.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        for (module, updates) in modules {
            let (path, bytes) =
                content::prepare_module_variable_updates(handler, &updates, Some(&module))?;
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid module filename")?;
            handler
                .update_file(name, &bytes)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub struct VariableReader<'a> {
    handler: &'a SaveGameHandler,
    paths: &'a NWN2Paths,
    globals: Option<Result<crate::parsers::xml::XmlData, String>>,
    modules: BTreeMap<String, Result<content::ModuleVariables, String>>,
}

impl<'a> VariableReader<'a> {
    pub fn new(handler: &'a SaveGameHandler, paths: &'a NWN2Paths) -> Self {
        Self {
            handler,
            paths,
            globals: None,
            modules: BTreeMap::new(),
        }
    }

    pub fn read(&mut self, key: &VariableKey) -> Result<Option<Value>, String> {
        if let Some(module) = &key.module {
            let vars = self
                .modules
                .entry(module.clone())
                .or_insert_with(|| {
                    CampaignManager::get_module_info_by_id(self.handler, self.paths, module)
                        .map(|(_, vars)| vars)
                })
                .as_ref()
                .map_err(Clone::clone)?;
            Ok(match key.value_type.as_str() {
                "int" => vars.integers.get(&key.name).map(|v| Value::from(*v)),
                "float" => vars.floats.get(&key.name).map(|v| Value::from(*v)),
                "string" => vars.strings.get(&key.name).map(|v| Value::from(v.clone())),
                _ => return Err("Unsupported module variable type".into()),
            })
        } else {
            let vars = self
                .globals
                .get_or_insert_with(|| CampaignManager::get_campaign_variables(self.handler))
                .as_ref()
                .map_err(Clone::clone)?;
            Ok(match key.value_type.as_str() {
                "int" => vars.integers.get(&key.name).map(|v| Value::from(*v)),
                "float" => vars.floats.get(&key.name).map(|v| Value::from(*v)),
                "string" => vars.strings.get(&key.name).map(|v| Value::from(v.clone())),
                "bool" => vars.booleans.get(&key.name).map(|v| Value::from(*v)),
                _ => return Err("Unsupported global variable type".into()),
            })
        }
    }
}

pub fn decode_assignment(
    action: &ConvoFunctor,
    module: &str,
) -> Result<(VariableKey, Value), &'static str> {
    let script = action.script.to_ascii_lowercase();
    let kind = match script.as_str() {
        "ga_global_int" | "ga_local_int" => "int",
        "ga_global_float" => "float",
        "ga_global_string" => "string",
        "ga_journal" | "__connector_quest_tuple__" => return Err("journal_action"),
        _ => return Err("unsupported_action"),
    };
    let local = script == "ga_local_int";
    if local && action.params.get(2).and_then(Value::as_str) != Some("$MODULE") {
        return Err("object_target");
    }
    let name = action
        .params
        .first()
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("invalid_assignment")?;
    let raw = action
        .params
        .get(1)
        .and_then(Value::as_str)
        .ok_or("invalid_assignment")?;
    let after = if kind == "string" {
        Value::from(raw)
    } else {
        if raw.is_empty() || raw.starts_with(['+', '-']) {
            return Err("arithmetic_action");
        }
        let number = raw.strip_prefix('=').unwrap_or(raw);
        // ginc_var_ops.GetChangeComponents treats a signed operand after '=' as 1.
        // Leave those surprising expressions for the engine instead of guessing intent.
        if kind == "int" {
            if number.starts_with(['+', '-']) || number.is_empty() {
                return Err("unsupported_expression");
            }
            Value::from(number.parse::<i32>().map_err(|_| "invalid_assignment")?)
        } else {
            let value = number.parse::<f32>().map_err(|_| "invalid_assignment")?;
            if !value.is_finite() {
                return Err("invalid_assignment");
            }
            Value::from(value)
        }
    };
    Ok((
        VariableKey {
            module: local.then(|| module.to_string()),
            name: name.into(),
            value_type: kind.into(),
        },
        after,
    ))
}

pub fn write_journal(
    bytes: &[u8],
    player: usize,
    edits: &[(String, u32)],
) -> Result<Vec<u8>, String> {
    let parser = GffParser::from_bytes(bytes.to_vec()).map_err(|e| e.to_string())?;
    let mut root: indexmap::IndexMap<_, _> = parser
        .read_struct_fields(0)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    let Some(GffValue::ListOwned(players)) = root.get_mut("Mod_PlayerList") else {
        return Err("Mod_PlayerList missing".into());
    };
    let fields = players.get_mut(player).ok_or("Primary player missing")?;
    let vars = fields
        .entry("VarTable".into())
        .or_insert_with(|| GffValue::ListOwned(Vec::new()));
    let GffValue::ListOwned(vars) = vars else {
        return Err("Invalid player VarTable".into());
    };
    for (tag, state) in edits {
        if tag.is_empty() {
            return Err("Empty quest tag".into());
        }
        let name = format!("NW_JOURNAL_ENTRY{tag}");
        let indices: Vec<_> = vars
            .iter()
            .enumerate()
            .filter(
                |(_, v)| matches!(v.get("Name"), Some(GffValue::String(n)) if n.as_ref() == name),
            )
            .map(|(i, _)| i)
            .collect();
        if indices.len() > 1 {
            return Err(format!("Duplicate journal variable: {tag}"));
        }
        if let Some(&index) = indices.first() {
            let var = &mut vars[index];
            if !matches!(var.get("Type"), Some(GffValue::Dword(1) | GffValue::Int(1))) {
                return Err("Journal variable is not an integer local".into());
            }
            let value = match var.get("Value") {
                Some(GffValue::Int(_)) => {
                    GffValue::Int(i32::try_from(*state).map_err(|_| "State exceeds INT range")?)
                }
                Some(GffValue::Dword(_)) => GffValue::Dword(*state),
                _ => return Err("Unsupported journal state storage type".into()),
            };
            var.insert("Value".into(), value);
        } else {
            let value = i32::try_from(*state).map_err(|_| "New state exceeds INT range")?;
            vars.push(indexmap::indexmap! {
                "Name".into() => GffValue::String(name.into()),
                "Type".into() => GffValue::Dword(1),
                "Value".into() => GffValue::Int(value),
            });
        }
    }
    GffWriter::new("IFO ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::toolset_bridge::FunctorKind;
    use indexmap::indexmap;
    use serde_json::json;

    fn action(script: &str, params: Value) -> ConvoFunctor {
        ConvoFunctor {
            kind: FunctorKind::Custom,
            script: script.into(),
            params: params.as_array().unwrap().clone(),
        }
    }

    #[test]
    fn assignments_follow_installed_wrapper_semantics() {
        for (script, params, expected) in [
            ("ga_global_int", json!(["x", "=5"]), json!(5)),
            ("ga_global_int", json!(["x", "0"]), json!(0)),
            ("ga_global_float", json!(["x", "=-2.5"]), json!(-2.5)),
            (
                "ga_global_string",
                json!(["x", "+anything"]),
                json!("+anything"),
            ),
            ("ga_local_int", json!(["x", "5", "$MODULE"]), json!(5)),
        ] {
            let (key, value) = decode_assignment(&action(script, params), "module_a").unwrap();
            assert_eq!(value, expected);
            assert_eq!(key.module.is_some(), script == "ga_local_int");
        }
        for expression in ["+4", "-4", "", "++", "--", "=-4", "=", "2147483648"] {
            assert!(
                decode_assignment(&action("ga_global_int", json!(["x", expression])), "m").is_err(),
                "{expression}"
            );
        }
        for script in [
            "gc_global_int",
            "custom_reward",
            "ga_module_int",
            "ga_global_bool",
        ] {
            assert!(decode_assignment(&action(script, json!(["x", "5"])), "m").is_err());
        }
        assert!(decode_assignment(&action("ga_local_int", json!(["x", "5", ""])), "m").is_err());
        assert!(decode_assignment(&action("ga_local_int", json!(["x", "5", "npc"])), "m").is_err());
    }

    fn playerlist() -> Vec<u8> {
        let vars = vec![
            indexmap! { "Name".into() => GffValue::String("NW_JOURNAL_ENTRYtest".into()), "Type".into() => GffValue::Dword(1), "Value".into() => GffValue::Int(10) },
            indexmap! { "Name".into() => GffValue::String("NW_JOURNAL_TIMEtest".into()), "Type".into() => GffValue::Dword(1), "Value".into() => GffValue::Int(123) },
            indexmap! { "Name".into() => GffValue::String("unrelated".into()), "Type".into() => GffValue::Dword(3), "Value".into() => GffValue::String("keep".into()) },
        ];
        let player = indexmap! { "VarTable".into() => GffValue::ListOwned(vars) };
        GffWriter::new("IFO ", "V3.2").write(indexmap! { "Mod_PlayerList".into() => GffValue::ListOwned(vec![player.clone(), player]) }).unwrap()
    }

    fn owned_players(bytes: &[u8]) -> Vec<indexmap::IndexMap<String, GffValue<'static>>> {
        let parser = GffParser::from_bytes(bytes.to_vec()).unwrap();
        let mut root = parser.read_struct_fields(0).unwrap();
        let GffValue::ListOwned(players) =
            root.swap_remove("Mod_PlayerList").unwrap().force_owned()
        else {
            panic!()
        };
        players
    }

    #[test]
    fn journal_roundtrip_preserves_types_timestamps_and_other_players() {
        let bytes = playerlist();
        let updated = write_journal(&bytes, 1, &[("test".into(), 2), ("new".into(), 100)]).unwrap();
        let original = owned_players(&bytes);
        let result = owned_players(&updated);
        assert_eq!(format!("{:?}", original[0]), format!("{:?}", result[0]));
        let GffValue::ListOwned(vars) = &result[1]["VarTable"] else {
            panic!()
        };
        assert!(matches!(vars[0]["Value"], GffValue::Int(2)));
        assert!(matches!(vars[1]["Value"], GffValue::Int(123)));
        assert!(matches!(&vars[2]["Value"], GffValue::String(s) if s == "keep"));
        assert!(matches!(vars[3]["Type"], GffValue::Dword(1)));
        assert!(matches!(vars[3]["Value"], GffValue::Int(100)));
        assert!(write_journal(&bytes, 0, &[("test".into(), u32::MAX)]).is_err());
        assert!(write_journal(&bytes, 0, &[("new".into(), u32::MAX)]).is_err());
        assert!(write_journal(&bytes, 2, &[("new".into(), 1)]).is_err());
    }

    pub(super) fn fixture_handler() -> (tempfile::TempDir, SaveGameHandler) {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(dir.path().join("resgff.zip")).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("playerlist.ifo", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&playerlist()).unwrap();
        zip.finish().unwrap();
        let handler = SaveGameHandler::new(dir.path(), false, false).unwrap();
        (dir, handler)
    }

    fn graph() -> SaveGraph {
        super::super::build_without_bridge(
            &fixture_handler().1,
            &crate::services::campaign::content::ModuleInfo::default(),
            &crate::services::campaign::content::ModuleVariables::default(),
            String::new(),
        )
    }

    fn add_quest(graph: &mut SaveGraph, tag: &str, actions: Vec<ConvoFunctor>) {
        graph.quests.push(super::super::QuestAggregate {
            tag: tag.into(),
            category: crate::services::toolset_bridge::JournalCategory {
                tag: tag.into(),
                name: tag.into(),
                priority: String::new(),
                xp: 0,
                source: String::new(),
                entries: Vec::new(),
            },
            defined_in: Vec::new(),
            live_state: None,
            transitions: vec![TransitionNode {
                module: "not_visited".into(),
                dlg: "dialogue".into(),
                node: 1,
                new_state: 20,
                text_strref: None,
                actions,
                co_authored_globals: Vec::new(),
                co_authored_locals: Vec::new(),
                gating_conditions: Vec::new(),
            }],
        });
    }

    #[test]
    fn optional_assignments_roundtrip_and_external_changes_invalidate_preview() {
        let (dir, mut handler) = fixture_handler();
        let globals = dir.path().join("globals.xml");
        std::fs::write(&globals, "<Globals><Integers><Integer><Name>progress</Name><Value>1</Value></Integer></Integers></Globals>").unwrap();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_global_int", json!(["progress", "5"])),
                action("ga_global_float", json!(["fraction", "2.5"])),
                action("ga_global_string", json!(["text", "a & b"])),
                action("ga_global_int", json!(["delta", "+3"])),
                action("ga_local_int", json!(["local", "5", "npc"])),
                action("ga_local_int", json!(["local", "5", "$MODULE"])),
            ],
        );
        let mut edits = QuestEdits::default();
        let preview = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
            .unwrap();
        assert_eq!(preview.writes.len(), 3);
        assert_eq!(preview.writes[0].saved, Some(json!(1)));
        assert_eq!(preview.effects.len(), 3);
        assert!(
            preview
                .effects
                .iter()
                .any(|e| e.reason == "unavailable_target")
        );
        let revision = preview.revision.clone();
        edits.stage(preview, &graph, &[0, 2], false).unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0).unwrap();
        assert!(
            std::fs::read_to_string(&globals)
                .unwrap()
                .contains("<Value>1</Value>")
        );
        assert_eq!(
            CampaignManager::get_campaign_variables(&handler)
                .unwrap()
                .integers["progress"],
            5
        );
        handler.commit_batch().unwrap();
        let values = CampaignManager::get_campaign_variables(&handler).unwrap();
        assert_eq!(values.integers["progress"], 5);
        assert_eq!(values.strings["text"], "a & b");
        assert!(!values.floats.contains_key("fraction"));
        assert!(!values.integers.contains_key("delta"));
        assert!(
            edits
                .validate_saved(&handler, &NWN2Paths::new(), 0)
                .is_err()
        );
        edits.clear();
        assert_ne!(
            edits
                .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
                .unwrap()
                .revision,
            revision
        );
    }

    #[test]
    fn conflicting_routes_require_explicit_replacement_and_discard_recomputes_overlay() {
        let (dir, handler) = fixture_handler();
        std::fs::write(dir.path().join("globals.xml"), "<Globals />").unwrap();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![action("ga_global_int", json!(["shared", "5"]))],
        );
        add_quest(
            &mut graph,
            "other",
            vec![action("ga_global_int", json!(["shared", "7"]))],
        );
        let mut edits = QuestEdits::default();
        let first = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
            .unwrap();
        edits.stage(first, &graph, &[0], false).unwrap();
        let second = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "other", 20, Some(0))
            .unwrap();
        assert_eq!(second.writes[0].before, Some(json!(5)));
        assert!(edits.stage(second.clone(), &graph, &[0], false).is_err());
        assert_eq!(edits.pending[0].tag, "test");
        edits.stage(second, &graph, &[0], true).unwrap();
        assert_eq!(edits.pending.len(), 1);
        assert_eq!(edits.pending[0].tag, "other");
        edits.discard(Some("other"));
        assert_eq!(
            edits
                .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
                .unwrap()
                .writes[0]
                .before,
            None
        );
    }

    #[test]
    fn staging_is_reversible_and_does_not_write_until_commit() {
        let (dir, mut handler) = fixture_handler();
        let original = std::fs::read(dir.path().join("resgff.zip")).unwrap();
        let mut edits = QuestEdits::default();
        let mut graph = graph();
        graph.quests.push(super::super::QuestAggregate {
            tag: "test".into(),
            category: crate::services::toolset_bridge::JournalCategory {
                tag: "test".into(),
                name: "Test".into(),
                priority: String::new(),
                xp: 0,
                source: String::new(),
                entries: Vec::new(),
            },
            defined_in: Vec::new(),
            live_state: Some(10),
            transitions: Vec::new(),
        });
        let preview = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, None)
            .unwrap();
        let revision = preview.revision.clone();
        edits.stage(preview, &graph, &[], false).unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("resgff.zip")).unwrap(),
            original
        );
        let next = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, None)
            .unwrap();
        assert_ne!(next.revision, revision);
        assert_eq!(next.before_state, Some(20));
        edits
            .validate_saved(&handler, &NWN2Paths::new(), 0)
            .unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0).unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("resgff.zip")).unwrap(),
            original
        );
        handler.abort_batch();
        assert_eq!(read_live_journal(&handler, 0).unwrap()[0].state, 10);
        handler.begin_batch();
        edits.flush(&mut handler, 0).unwrap();
        handler.commit_batch().unwrap();
        assert_eq!(read_live_journal(&handler, 0).unwrap()[0].state, 20);
        assert!(
            edits
                .validate_saved(&handler, &NWN2Paths::new(), 0)
                .is_err()
        );
        edits.discard(None);
        assert!(edits.pending.is_empty());
    }

    #[test]
    fn saved_module_assignments_are_staged_and_scoped_to_the_selected_module() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/saves/Classic_Campaign");
        if !fixture.exists() {
            eprintln!("Classic_Campaign fixture unavailable");
            return;
        }
        let source = std::fs::read_dir(fixture)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("z"))
            })
            .min_by_key(|e| e.metadata().unwrap().len())
            .unwrap()
            .path();
        let original = std::fs::read(source).unwrap();
        let (dir, mut handler) = fixture_handler();
        std::fs::write(dir.path().join("module_a.z"), &original).unwrap();
        std::fs::write(dir.path().join("module_b.z"), &original).unwrap();
        let paths = NWN2Paths::new();
        let (_, vars) =
            CampaignManager::get_module_info_by_id(&handler, &paths, "module_a").unwrap();
        let (name, saved) = vars.integers.iter().next().unwrap();
        let target = if *saved == 5 { 6 } else { 5 };
        let mut edits = QuestEdits::default();
        edits.pending.push(PendingQuestEdit {
            tag: "test".into(),
            target: 20,
            saved_state: Some(10),
            source: None,
            writes: vec![VariableWrite {
                key: VariableKey {
                    module: Some("module_a".into()),
                    name: name.clone(),
                    value_type: "int".into(),
                },
                saved: Some(json!(*saved)),
                before: Some(json!(*saved)),
                after: json!(target),
                action_index: 0,
            }],
        });
        edits.validate_saved(&handler, &paths, 0).unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0).unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("module_a.z")).unwrap(),
            original
        );
        handler.commit_batch().unwrap();
        assert_eq!(
            CampaignManager::get_module_info_by_id(&handler, &paths, "module_a")
                .unwrap()
                .1
                .integers[name],
            target
        );
        assert_eq!(
            std::fs::read(dir.path().join("module_b.z")).unwrap(),
            original
        );
    }

    #[test]
    fn real_save_journal_storage_matches_new_entry_schema() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/saves/Classic_Campaign");
        if !path.exists() {
            eprintln!("Classic_Campaign fixture unavailable");
            return;
        }
        let handler = SaveGameHandler::new(path, false, false).unwrap();
        let bytes = handler.extract_player_data().unwrap();
        let players = owned_players(&bytes);
        let mut count = 0;
        for player in players {
            let Some(GffValue::ListOwned(vars)) = player.get("VarTable") else {
                continue;
            };
            for var in vars {
                if matches!(var.get("Name"), Some(GffValue::String(n)) if n.starts_with("NW_JOURNAL_ENTRY"))
                {
                    assert!(matches!(var.get("Type"), Some(GffValue::Dword(1))));
                    assert!(matches!(
                        var.get("Value"),
                        Some(GffValue::Int(_) | GffValue::Dword(_))
                    ));
                    count += 1;
                }
            }
        }
        assert!(
            count > 0,
            "Fixture must contain engine-authored journal variables"
        );
    }
}
