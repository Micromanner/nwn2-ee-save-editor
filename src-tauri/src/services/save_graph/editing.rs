use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{FollowNode, SaveGraph, TransitionNode, read_live_journal};
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
    /// The wrapper script call this write was decoded from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<String>,
    /// Follow-up line (node id in `QuestPreview::followed`) this write comes from;
    /// `None` for the chosen line itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionEffect {
    pub action: ConvoFunctor,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
}

/// One dialogue line that reaches a target entry, with the outcome of choosing it
/// reduced to a signature so the UI can collapse lines with identical effects.
#[derive(Debug, Clone, Serialize)]
pub struct RouteSummary {
    pub route: usize,
    pub module: String,
    pub speaker: String,
    pub is_player: bool,
    pub text_strref: Option<i32>,
    pub debug: bool,
    /// Sorted `scope/name=value` writes: the only outcome a player can tell apart.
    pub signature: String,
    /// The flags this line (and the lines the game plays after it) would set.
    pub writes: Vec<RouteWrite>,
    pub write_count: usize,
    pub followed_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteWrite {
    pub name: String,
    pub after: Value,
}

/// A line the game would play after the chosen one without another player decision.
#[derive(Debug, Clone, Serialize)]
pub struct FollowedLine {
    pub node: i32,
    pub is_player: bool,
    pub has_text: bool,
    pub speaker: String,
    pub text_strref: Option<i32>,
}

/// Why the preview stopped following the conversation.
#[derive(Debug, Clone, Serialize)]
pub struct FollowStop {
    /// `end`, `player_choice`, `unknown_condition`, `depth`.
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<ConvoFunctor>,
    pub choices: usize,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub followed: Vec<FollowedLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_stop: Option<FollowStop>,
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
            followed: Vec::new(),
            follow_stop: None,
        };
        let mut reader = VariableReader::new(handler, paths);
        if let Some(index) = route {
            let tr = quest
                .transitions
                .get(index)
                .filter(|tr| tr.new_state == target)
                .ok_or("The selected route does not reach this state")?;
            // The chosen line first, then every line the game plays after it.
            let mut lines: Vec<(Option<i32>, &[ConvoFunctor])> = vec![(None, &tr.actions)];
            let (followed, stop) = follow_conversation(tr, &mut reader)?;
            for node in &followed {
                lines.push((Some(node.node), &node.actions));
            }
            result.followed = followed
                .iter()
                .map(|n| FollowedLine {
                    node: n.node,
                    is_player: n.is_player,
                    has_text: n.has_text,
                    speaker: n.speaker.clone(),
                    text_strref: n.text_strref,
                })
                .collect();
            result.follow_stop = Some(stop);
            let mut candidates = Vec::new();
            let mut all_actions: Vec<ConvoFunctor> = Vec::new();
            for (line, actions) in lines {
                for action in actions {
                    let action_index = all_actions.len();
                    all_actions.push(action.clone());
                    if line.is_some()
                        && let Some((other_tag, state)) = journal_transition(action)
                    {
                        if other_tag != tag || state != target {
                            result.effects.push(ActionEffect {
                                action: action.clone(),
                                reason: "follow_up_journal".into(),
                                line,
                            });
                        }
                        continue;
                    }
                    match decode_assignment(action, &tr.module) {
                        Ok((key, change)) => match reader.read(&key) {
                            Ok(saved) => {
                                let before = self
                                    .pending
                                    .iter()
                                    .flat_map(|p| &p.writes)
                                    .rfind(|w| w.key == key)
                                    .map(|w| w.after.clone())
                                    .or(saved.clone());
                                let after = match change {
                                    Change::Set(value) => value,
                                    Change::Add(delta) => Value::from(
                                        before.as_ref().and_then(Value::as_i64).unwrap_or(0)
                                            + delta,
                                    ),
                                };
                                candidates.push(VariableWrite {
                                    key,
                                    saved,
                                    before,
                                    after,
                                    action_index,
                                    derived_from: action.derived_from.clone(),
                                    line,
                                });
                            }
                            Err(_) => result.effects.push(ActionEffect {
                                action: action.clone(),
                                reason: "unavailable_target".into(),
                                line,
                            }),
                        },
                        Err(reason) => result.effects.push(ActionEffect {
                            action: action.clone(),
                            reason: reason.into(),
                            line,
                        }),
                    }
                }
            }
            // Multiple writes to one variable have ordering semantics; don't offer a subset.
            for candidate in &candidates {
                if candidates.iter().filter(|w| w.key == candidate.key).count() == 1 {
                    result.writes.push(candidate.clone());
                } else {
                    result.effects.push(ActionEffect {
                        action: all_actions[candidate.action_index].clone(),
                        reason: "multiple_assignments".into(),
                        line: candidate.line,
                    });
                }
            }
        }
        let mut hash = Sha256::new();
        hash.update(self.generation.to_le_bytes());
        hash.update(&bytes);
        hash.update(serde_json::to_vec(&result).map_err(|e| e.to_string())?);
        result.revision = hex::encode(hash.finalize());
        Ok(result)
    }

    /// Preview every line that reaches `target` and reduce each to a `RouteSummary`.
    pub fn summarize_routes(
        &self,
        graph: &SaveGraph,
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        player: usize,
        tag: &str,
        target: u32,
    ) -> Result<Vec<RouteSummary>, String> {
        let quest = graph
            .quests
            .iter()
            .find(|q| q.tag == tag)
            .ok_or("Quest is no longer available")?;
        let mut out = Vec::new();
        for (route, tr) in quest.transitions.iter().enumerate() {
            if tr.new_state != target {
                continue;
            }
            let preview = self.preview(graph, handler, paths, player, tag, target, Some(route))?;
            let mut writes: Vec<String> = preview
                .writes
                .iter()
                .map(|w| {
                    format!(
                        "{}/{}={}",
                        w.key.module.as_deref().unwrap_or(""),
                        w.key.name,
                        w.after
                    )
                })
                .collect();
            writes.sort();
            out.push(RouteSummary {
                route,
                module: tr.module.clone(),
                speaker: tr.speaker.clone(),
                is_player: tr.is_player,
                text_strref: tr.text_strref,
                debug: tr.debug,
                signature: writes.join(";"),
                writes: preview
                    .writes
                    .iter()
                    .map(|w| RouteWrite {
                        name: w.key.name.clone(),
                        after: w.after.clone(),
                    })
                    .collect(),
                write_count: preview.writes.len(),
                followed_count: preview.followed.len(),
            });
        }
        Ok(out)
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

/// `ga_journal(tag, state)` / connector quest tuple on an action.
fn journal_transition(action: &ConvoFunctor) -> Option<(String, u32)> {
    let script = action.script.to_ascii_lowercase();
    if script != "ga_journal" && script != "__connector_quest_tuple__" {
        return None;
    }
    let tag = action.params.first()?.as_str()?.to_string();
    let state = action
        .params
        .get(1)?
        .as_i64()
        .and_then(|n| u32::try_from(n).ok())?;
    Some((tag, state))
}

const FOLLOW_LIMIT: usize = 32;

/// Walk the lines the game plays after `tr` the way the engine does: after a
/// player line the first NPC line whose conditions pass; after an NPC line the
/// only offered reply. Stops at a real player choice, at a condition the editor
/// cannot evaluate, or at the end of the conversation.
fn follow_conversation<'a>(
    tr: &'a TransitionNode,
    reader: &mut VariableReader<'_>,
) -> Result<(Vec<&'a FollowNode>, FollowStop), String> {
    let by_id: BTreeMap<i32, &FollowNode> = tr.follow_up.iter().map(|n| (n.node, n)).collect();
    let mut followed: Vec<&FollowNode> = Vec::new();
    let mut children = &tr.children;
    let mut from_player = tr.is_player;
    let stop = loop {
        if followed.len() >= FOLLOW_LIMIT {
            break FollowStop {
                reason: "depth".into(),
                node: None,
                conditions: Vec::new(),
                choices: 0,
            };
        }
        let candidates: Vec<&FollowNode> = children
            .iter()
            .filter_map(|id| by_id.get(id).copied())
            .collect();
        if candidates.is_empty() {
            break FollowStop {
                reason: "end".into(),
                node: None,
                conditions: Vec::new(),
                choices: 0,
            };
        }
        let mut passing: Vec<&FollowNode> = Vec::new();
        let mut unknown: Option<&FollowNode> = None;
        for candidate in &candidates {
            match evaluate_conditions(&candidate.conditions, reader, &tr.module)? {
                Some(true) => {
                    passing.push(candidate);
                    if from_player {
                        break;
                    }
                }
                Some(false) => {}
                None => {
                    unknown = Some(candidate);
                    break;
                }
            }
        }
        if let Some(node) = unknown {
            break FollowStop {
                reason: "unknown_condition".into(),
                node: Some(node.node),
                conditions: node.conditions.clone(),
                choices: 0,
            };
        }
        let next = match passing.as_slice() {
            [] => {
                break FollowStop {
                    reason: "end".into(),
                    node: None,
                    conditions: Vec::new(),
                    choices: 0,
                };
            }
            [only] => *only,
            many => {
                break FollowStop {
                    reason: "player_choice".into(),
                    node: None,
                    conditions: Vec::new(),
                    choices: many.len(),
                };
            }
        };
        if followed.iter().any(|n| n.node == next.node) {
            break FollowStop {
                reason: "depth".into(),
                node: Some(next.node),
                conditions: Vec::new(),
                choices: 0,
            };
        }
        followed.push(next);
        children = &next.children;
        from_player = next.is_player;
    };
    Ok((followed, stop))
}

/// Evaluate a node's condition list against the save. `None` when any condition
/// uses a script the editor cannot evaluate.
fn evaluate_conditions(
    conditions: &[ConvoFunctor],
    reader: &mut VariableReader<'_>,
    module: &str,
) -> Result<Option<bool>, String> {
    let mut result: Option<bool> = None;
    let mut join_with_or = false;
    for condition in conditions {
        let Some(value) = evaluate_condition(condition, reader, module)? else {
            return Ok(None);
        };
        let value = value != condition.negate;
        result = Some(match result {
            None => value,
            Some(acc) if join_with_or => acc || value,
            Some(acc) => acc && value,
        });
        join_with_or = condition.join.eq_ignore_ascii_case("or");
    }
    Ok(Some(result.unwrap_or(true)))
}

fn evaluate_condition(
    condition: &ConvoFunctor,
    reader: &mut VariableReader<'_>,
    module: &str,
) -> Result<Option<bool>, String> {
    let script = condition.script.to_ascii_lowercase();
    let local = match script.as_str() {
        "gc_global_int" => false,
        "gc_local_int" => true,
        _ => return Ok(None),
    };
    if local && condition.params.get(2).and_then(Value::as_str) != Some("$MODULE") {
        return Ok(None);
    }
    let Some(name) = condition.params.first().and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(check) = condition.params.get(1).and_then(Value::as_str) else {
        return Ok(None);
    };
    let key = VariableKey {
        module: local.then(|| module.to_string()),
        name: name.into(),
        value_type: "int".into(),
    };
    // A module the save has never visited has no variables the editor can read;
    // treat its checks as undecidable rather than failing the whole preview.
    let Ok(value) = reader.read(&key) else {
        return Ok(None);
    };
    let current = value.and_then(|v| v.as_i64()).unwrap_or(0);
    Ok(compare_check(current, check))
}

/// `ginc_var_ops` check syntax: `"1"`, `"=1"`, `">0"`, `"<=2"`, `"!=3"`.
fn compare_check(current: i64, check: &str) -> Option<bool> {
    let check = check.trim();
    let (op, rest) = ["!=", "<=", ">=", "<", ">", "=", "!"]
        .iter()
        .find_map(|op| check.strip_prefix(op).map(|rest| (*op, rest)))
        .unwrap_or(("=", check));
    let target: i64 = rest.trim().parse().ok()?;
    Some(match op {
        "!=" | "!" => current != target,
        "<=" => current <= target,
        ">=" => current >= target,
        "<" => current < target,
        ">" => current > target,
        _ => current == target,
    })
}

/// What an assignment action does to its variable, per `ginc_var_ops`.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Set(Value),
    /// `"+3"`, `"-1"`, `"++"`: applied to the current value at preview time.
    Add(i64),
}

/// NWScript `StringToInt`: leading integer, anything else is 0.
fn string_to_int(raw: &str) -> Option<i64> {
    let raw = raw.trim_start();
    let end = raw
        .char_indices()
        .take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+')))
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    match &raw[..end] {
        "" | "+" | "-" => Some(0),
        digits => digits.parse::<i32>().ok().map(i64::from),
    }
}

pub fn decode_assignment(
    action: &ConvoFunctor,
    module: &str,
) -> Result<(VariableKey, Change), &'static str> {
    let script = action.script.to_ascii_lowercase();
    let kind = match script.as_str() {
        "ga_global_int" | "ga_local_int" => "int",
        "ga_global_float" | "ga_local_float" => "float",
        "ga_global_string" | "ga_local_string" => "string",
        "ga_journal" | "__connector_quest_tuple__" => return Err("journal_action"),
        _ if action.decoded.is_some() => return Err("decoded_script"),
        _ => return Err("unsupported_action"),
    };
    let local = script.starts_with("ga_local_");
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
        Change::Set(Value::from(raw))
    } else if raw.is_empty() {
        return Err("unsupported_expression");
    } else if let Some(delta) = raw.strip_prefix(['+', '-']) {
        // GetChangeComponents: "+3" adds 3; "+", "++", "+-" add 1 (same for "-").
        if kind != "int" {
            return Err("arithmetic_action");
        }
        let sign = if raw.starts_with('-') { -1 } else { 1 };
        let magnitude = if delta.is_empty() || delta.starts_with(['+', '-']) {
            1
        } else {
            delta.parse::<i32>().map_err(|_| "invalid_assignment")?
        };
        Change::Add(sign * i64::from(magnitude))
    } else {
        let number = raw.strip_prefix('=').unwrap_or(raw);
        // ginc_var_ops.GetChangeComponents treats a signed operand after '=' as 1.
        // Leave those surprising expressions for the engine instead of guessing intent.
        if kind == "int" {
            if number.starts_with(['+', '-']) || number.is_empty() {
                return Err("unsupported_expression");
            }
            // Non-numeric text (e.g. ">0") reaches StringToInt in-game and becomes 0.
            Change::Set(Value::from(
                string_to_int(number).ok_or("invalid_assignment")?,
            ))
        } else {
            let value = number.parse::<f32>().map_err(|_| "invalid_assignment")?;
            if !value.is_finite() {
                return Err("invalid_assignment");
            }
            Change::Set(Value::from(value))
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
            derived_from: None,
            decoded: None,
            negate: false,
            join: String::new(),
        }
    }

    #[test]
    fn assignments_follow_installed_wrapper_semantics() {
        for (script, params, expected) in [
            ("ga_global_int", json!(["x", "=5"]), Change::Set(json!(5))),
            ("ga_global_int", json!(["x", "0"]), Change::Set(json!(0))),
            ("ga_global_int", json!(["x", ">0"]), Change::Set(json!(0))),
            ("ga_global_int", json!(["x", "+4"]), Change::Add(4)),
            ("ga_global_int", json!(["x", "-4"]), Change::Add(-4)),
            ("ga_global_int", json!(["x", "++"]), Change::Add(1)),
            ("ga_global_int", json!(["x", "-"]), Change::Add(-1)),
            (
                "ga_global_float",
                json!(["x", "=-2.5"]),
                Change::Set(json!(-2.5)),
            ),
            (
                "ga_global_string",
                json!(["x", "+anything"]),
                Change::Set(json!("+anything")),
            ),
            (
                "ga_local_int",
                json!(["x", "5", "$MODULE"]),
                Change::Set(json!(5)),
            ),
        ] {
            let (key, value) = decode_assignment(&action(script, params), "module_a").unwrap();
            assert_eq!(value, expected);
            assert_eq!(key.module.is_some(), script == "ga_local_int");
        }
        assert!(decode_assignment(&action("ga_global_float", json!(["x", "+1"])), "m").is_err());
        for expression in ["", "=-4", "=", "2147483648"] {
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
            crate::services::save_graph::OrphanKind::PlatformUnsupported,
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
                speaker: String::new(),
                new_state: 20,
                text_strref: None,
                actions,
                co_authored_globals: Vec::new(),
                co_authored_locals: Vec::new(),
                gating_conditions: Vec::new(),
                is_player: true,
                debug: false,
                children: Vec::new(),
                follow_up: Vec::new(),
            }],
        });
    }

    fn follow(
        node: i32,
        is_player: bool,
        has_text: bool,
        conditions: Vec<ConvoFunctor>,
        actions: Vec<ConvoFunctor>,
        children: Vec<i32>,
    ) -> FollowNode {
        FollowNode {
            node,
            is_player,
            has_text,
            speaker: String::new(),
            text_strref: None,
            conditions,
            actions,
            children,
        }
    }

    fn condition(script: &str, params: Value, negate: bool, join: &str) -> ConvoFunctor {
        ConvoFunctor {
            negate,
            join: join.into(),
            ..action(script, params)
        }
    }

    /// Shape of Cormick's "Sign me up" branch: two NPC replies, the first gated on a
    /// global, both funnelling through an empty continue reply into the line that
    /// sets the Watch flag; then a real player choice.
    fn cormick_like(graph: &mut SaveGraph) {
        add_quest(
            graph,
            "test",
            vec![action("__connector_quest_tuple__", json!(["test", 20]))],
        );
        let tr = &mut graph.quests.last_mut().unwrap().transitions[0];
        tr.children = vec![10, 11];
        tr.follow_up = vec![
            follow(
                10,
                false,
                true,
                vec![condition(
                    "gc_global_int",
                    json!(["friendly", "1"]),
                    false,
                    "",
                )],
                Vec::new(),
                vec![20],
            ),
            follow(11, false, true, Vec::new(), Vec::new(), vec![21]),
            follow(20, true, false, Vec::new(), Vec::new(), vec![30]),
            follow(21, true, false, Vec::new(), Vec::new(), vec![30]),
            follow(
                30,
                false,
                true,
                Vec::new(),
                vec![
                    action("ga_global_int", json!(["joined", "1"])),
                    action("ga_global_int", json!(["visits", "++"])),
                    action("__connector_quest_tuple__", json!(["other", 5])),
                ],
                vec![40, 41],
            ),
            follow(
                40,
                true,
                true,
                Vec::new(),
                vec![action("ga_global_int", json!(["never", "1"]))],
                Vec::new(),
            ),
            follow(41, true, true, Vec::new(), Vec::new(), Vec::new()),
        ];
    }

    #[test]
    fn preview_follows_the_conversation_the_engine_would_play() {
        let (dir, handler) = fixture_handler();
        std::fs::write(dir.path().join("globals.xml"), "<Globals><Integers><Integer><Name>visits</Name><Value>2</Value></Integer></Integers></Globals>").unwrap();
        let mut graph = graph();
        cormick_like(&mut graph);
        let edits = QuestEdits::default();
        let preview = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
            .unwrap();
        // `friendly` is absent (0), so the ungated second reply plays.
        assert_eq!(
            preview.followed.iter().map(|l| l.node).collect::<Vec<_>>(),
            vec![11, 21, 30]
        );
        let stop = preview.follow_stop.as_ref().unwrap();
        assert_eq!(stop.reason, "player_choice");
        assert_eq!(stop.choices, 2);
        let writes: Vec<_> = preview
            .writes
            .iter()
            .map(|w| (w.key.name.as_str(), w.after.clone(), w.line))
            .collect();
        assert_eq!(
            writes,
            vec![
                ("joined", json!(1), Some(30)),
                ("visits", json!(3), Some(30))
            ]
        );
        assert!(
            preview
                .effects
                .iter()
                .any(|e| e.reason == "follow_up_journal" && e.line == Some(30))
        );

        // With the gate satisfied the first reply wins; negation flips it back.
        std::fs::write(dir.path().join("globals.xml"), "<Globals><Integers><Integer><Name>friendly</Name><Value>1</Value></Integer></Integers></Globals>").unwrap();
        let preview = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
            .unwrap();
        assert_eq!(
            preview.followed.iter().map(|l| l.node).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
        graph.quests[0].transitions[0].follow_up[0].conditions[0].negate = true;
        let preview = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
            .unwrap();
        assert_eq!(preview.followed[0].node, 11);

        // A condition the editor cannot evaluate stops the walk before that line.
        graph.quests[0].transitions[0].follow_up[0].conditions[0] =
            condition("10c_brelaina_ck", json!([20]), false, "");
        let preview = edits
            .preview(&graph, &handler, &NWN2Paths::new(), 0, "test", 20, Some(0))
            .unwrap();
        assert!(preview.followed.is_empty());
        let stop = preview.follow_stop.as_ref().unwrap();
        assert_eq!(stop.reason, "unknown_condition");
        assert_eq!(stop.node, Some(10));
        assert!(preview.writes.is_empty());
    }

    #[test]
    fn condition_checks_follow_ginc_var_ops_and_join_order() {
        assert_eq!(compare_check(3, "3"), Some(true));
        assert_eq!(compare_check(3, "=3"), Some(true));
        assert_eq!(compare_check(3, ">2"), Some(true));
        assert_eq!(compare_check(3, ">=4"), Some(false));
        assert_eq!(compare_check(3, "<4"), Some(true));
        assert_eq!(compare_check(3, "<=2"), Some(false));
        assert_eq!(compare_check(3, "!=3"), Some(false));
        assert_eq!(compare_check(3, "!3"), Some(false));
        assert_eq!(compare_check(3, "abc"), None);

        let (dir, handler) = fixture_handler();
        std::fs::write(dir.path().join("globals.xml"), "<Globals><Integers><Integer><Name>a</Name><Value>1</Value></Integer></Integers></Globals>").unwrap();
        let paths = NWN2Paths::new();
        let mut reader = VariableReader::new(&handler, &paths);
        // `a == 1 OR b == 1` then `AND c == 1`: (true || false) && false.
        let conditions = vec![
            condition("gc_global_int", json!(["a", "1"]), false, "or"),
            condition("gc_global_int", json!(["b", "1"]), false, "and"),
            condition("gc_global_int", json!(["c", "1"]), false, "and"),
        ];
        assert_eq!(
            evaluate_conditions(&conditions, &mut reader, "m").unwrap(),
            Some(false)
        );
        assert_eq!(
            evaluate_conditions(&conditions[..2], &mut reader, "m").unwrap(),
            Some(true)
        );
        assert_eq!(
            evaluate_conditions(&[], &mut reader, "m").unwrap(),
            Some(true)
        );
        let object_local = condition("gc_local_int", json!(["a", "1", "npc"]), false, "");
        assert_eq!(
            evaluate_conditions(&[object_local], &mut reader, "m").unwrap(),
            None
        );
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
        assert_eq!(preview.writes.len(), 4);
        assert_eq!(preview.writes[0].saved, Some(json!(1)));
        // "+3" on an absent variable: the engine adds to 0.
        assert_eq!(preview.writes[3].key.name, "delta");
        assert_eq!(preview.writes[3].saved, None);
        assert_eq!(preview.writes[3].after, json!(3));
        assert_eq!(preview.effects.len(), 2);
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
                derived_from: None,
                line: None,
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
