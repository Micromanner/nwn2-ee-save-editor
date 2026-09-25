use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::warn;

use super::aggregator::derived_call;
use super::area_objects::{self, AreaField, ModuleObjects};
use super::item_blueprints::{ItemBlueprint, ItemBlueprints};
use super::{FollowNode, SaveGraph, SpokenLine, TransitionNode, read_live_journal};
use crate::character::{Character, FeatId, FeatSource};
use crate::config::NWN2Paths;
use crate::loaders::GameData;
use crate::parsers::erf::ErfParser;
use crate::parsers::gff::{GffParser, GffValue, GffWriter};
use crate::parsers::tda::TDAParser;
use crate::services::campaign::{CampaignManager, content};
use crate::services::resource_manager::ResourceManager;
use crate::services::roster::{self, RosterMember};
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
    /// Display name of an `ITEM_TYPE` write's blueprint, when embedded in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// TLK strref of that name when it is not embedded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_strref: Option<i32>,
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
    pub dlg: String,
    pub speaker: String,
    pub is_player: bool,
    pub text_strref: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoken: Option<SpokenLine>,
    pub debug: bool,
    /// Sorted writes and actions: the outcome a player can tell apart.
    pub signature: String,
    /// The flags this line (and the lines the game plays after it) would set.
    pub writes: Vec<RouteWrite>,
    /// Everything else the game does on this route (rewards, alignment, doors...).
    pub effects: Vec<ConvoFunctor>,
    pub write_count: usize,
    pub followed_count: usize,
}

/// A later entry of the same quest and the lines that reach it, with whether the
/// save (after this edit) already satisfies each line's conditions.
#[derive(Debug, Clone, Serialize)]
pub struct NextStep {
    pub state: u32,
    pub lines: Vec<NextLine>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextLine {
    pub route: usize,
    pub module: String,
    pub dlg: String,
    pub speaker: String,
    pub is_player: bool,
    pub text_strref: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoken: Option<SpokenLine>,
    /// `open` (conditions pass after this edit), `closed`, or `unknown`.
    pub gate: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteWrite {
    pub name: String,
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    pub after: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_strref: Option<i32>,
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
    /// Other entries of this quest reachable through dialogue, evaluated against
    /// the save as it will be after this edit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_steps: Vec<NextStep>,
    /// Blueprints of the `ITEM_TYPE` writes by resref, carried to the staged edit.
    #[serde(skip)]
    pub blueprints: BTreeMap<String, Arc<ItemBlueprint>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingQuestEdit {
    pub tag: String,
    pub target: u32,
    pub saved_state: Option<u32>,
    pub source: Option<TransitionNode>,
    pub writes: Vec<VariableWrite>,
    #[serde(skip)]
    pub blueprints: BTreeMap<String, Arc<ItemBlueprint>>,
    /// What staging changed on the loaded player, to take back on discard; `None`
    /// when the player is not loaded and `flush` writes the player's changes.
    #[serde(skip)]
    pub applied: Option<Vec<MemoryChange>>,
}

/// One change a staged quest edit made to the loaded player.
#[derive(Debug, Clone)]
pub enum MemoryChange {
    /// Integer field moved by `delta` (after clamping).
    Field {
        name: String,
        delta: i64,
    },
    Feat {
        id: i32,
        added: bool,
    },
    ItemsGiven {
        tag: String,
        units: i64,
    },
    /// Removed items, each with the removed units as its `StackSize`.
    ItemsTaken {
        items: Vec<indexmap::IndexMap<String, GffValue<'static>>>,
    },
}

/// A staged variable value as a number; `flush` writes the same text.
fn variable_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
}

fn variable_text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

/// Write kinds staging applies to the loaded player instead of `flush`.
pub fn applies_to_player(value_type: &str) -> bool {
    matches!(
        value_type,
        CHARACTER_TYPE | FEAT_TYPE | ITEM_TYPE | ITEM_TAKE_TYPE
    )
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

    /// Remove the pending edit of `tag` (every edit when `None`); returns the removed edits.
    pub fn discard(&mut self, tag: Option<&str>) -> Vec<PendingQuestEdit> {
        let (removed, kept) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|p| tag.is_none_or(|tag| p.tag == tag));
        self.pending = kept;
        self.generation = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        removed
    }

    /// Pending story-variable writes in staging order: globals when `module` is `None`,
    /// else that module's locals.
    fn pending_variables<'a>(
        &'a self,
        module: Option<&'a str>,
    ) -> impl Iterator<Item = &'a VariableWrite> + 'a {
        self.pending
            .iter()
            .flat_map(|p| &p.writes)
            .filter(move |w| {
                matches!(w.key.value_type.as_str(), "int" | "float" | "string")
                    && match (&w.key.module, module) {
                        (None, None) => true,
                        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                        _ => false,
                    }
            })
    }

    /// `globals.xml` values as they will be once the pending edits are saved.
    pub fn overlay_globals(&self, data: &mut crate::parsers::xml::XmlData) {
        for write in self.pending_variables(None) {
            let name = write.key.name.clone();
            match write.key.value_type.as_str() {
                "int" => {
                    if let Some(v) = variable_number(&write.after) {
                        data.integers
                            .insert(name, i32::try_from(v as i64).unwrap_or_default());
                    }
                }
                "float" => {
                    if let Some(v) = variable_number(&write.after) {
                        data.floats.insert(name, v as f32);
                    }
                }
                _ => {
                    data.strings.insert(name, variable_text(&write.after));
                }
            }
        }
    }

    /// A module's variables as they will be once the pending edits are saved.
    pub fn overlay_module(&self, module: &str, vars: &mut content::ModuleVariables) {
        for write in self.pending_variables(Some(module)) {
            let name = write.key.name.clone();
            match write.key.value_type.as_str() {
                "int" => {
                    if let Some(v) = variable_number(&write.after) {
                        vars.integers
                            .insert(name, i32::try_from(v as i64).unwrap_or_default());
                    }
                }
                "float" => {
                    if let Some(v) = variable_number(&write.after) {
                        vars.floats.insert(name, v as f32);
                    }
                }
                _ => {
                    vars.strings.insert(name, variable_text(&write.after));
                }
            }
        }
    }

    /// Put back an edit taken out by undo.
    pub fn restore(&mut self, edit: PendingQuestEdit) {
        self.pending.retain(|p| p.tag != edit.tag);
        self.pending.push(edit);
        self.generation = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    }

    pub fn preview(
        &self,
        graph: &SaveGraph,
        tables: &RewardTables,
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
            next_steps: Vec::new(),
            blueprints: BTreeMap::new(),
        };
        let mut reader = VariableReader::new(handler, paths, player);
        let mut journal: BTreeMap<String, u32> = read_live_journal(handler, player)?
            .into_iter()
            .map(|q| (q.tag, q.state))
            .collect();
        for edit in &self.pending {
            journal.insert(edit.tag.clone(), edit.target);
        }
        journal.insert(tag.to_string(), target);
        reader.journal = Some(journal);
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
            let mut candidates: Vec<VariableWrite> = Vec::new();
            let mut all_actions: Vec<ConvoFunctor> = Vec::new();
            for (line, actions) in lines {
                for action in actions {
                    let action_index = all_actions.len();
                    all_actions.push(action.clone());
                    if let Some((other_tag, state)) = journal_transition(action) {
                        if other_tag == tag {
                            let reason = match line {
                                None => Some("journal_action"),
                                Some(_) if state != target => Some("follow_up_journal"),
                                Some(_) => None,
                            };
                            if let Some(reason) = reason {
                                result.effects.push(ActionEffect {
                                    action: action.clone(),
                                    reason: reason.into(),
                                    line,
                                });
                            }
                            continue;
                        }
                        // Another quest's entry on this route: the engine adds it like
                        // `AddJournalQuestEntry` without override, so it never moves back.
                        let key = VariableKey {
                            module: None,
                            name: journal_variable(&other_tag),
                            value_type: PC_LOCAL_TYPE.into(),
                        };
                        let after = Value::from(state);
                        if let Some(earlier) = candidates.iter_mut().find(|w| w.key == key) {
                            if earlier.after.as_u64() < Some(u64::from(state)) {
                                earlier.after = after;
                            }
                            continue;
                        }
                        let Ok(saved) = reader.read(&key) else {
                            result.effects.push(ActionEffect {
                                action: action.clone(),
                                reason: "unavailable_target".into(),
                                line,
                            });
                            continue;
                        };
                        // The journal map already holds the save and the pending quest targets.
                        let journal = reader.journal.get_or_insert_default();
                        let before = self
                            .pending
                            .iter()
                            .flat_map(|p| &p.writes)
                            .rfind(|w| w.key == key)
                            .map(|w| w.after.clone())
                            .or_else(|| journal.get(&other_tag).map(|s| Value::from(*s)));
                        if before.as_ref().and_then(Value::as_u64) >= Some(u64::from(state)) {
                            continue;
                        }
                        journal.insert(other_tag, state);
                        candidates.push(VariableWrite {
                            key,
                            saved,
                            before,
                            after,
                            action_index,
                            label: None,
                            label_strref: None,
                            derived_from: None,
                            line,
                        });
                        continue;
                    }
                    // Pending edits and earlier writes of this preview win over the save.
                    let mut current = |key: &VariableKey| {
                        candidates
                            .iter()
                            .rfind(|w| w.key == *key)
                            .map(|w| w.after.clone())
                            .or_else(|| {
                                self.pending
                                    .iter()
                                    .flat_map(|p| &p.writes)
                                    .rfind(|w| w.key == *key)
                                    .map(|w| w.after.clone())
                            })
                            .or_else(|| reader.read(key).ok().flatten())
                            .and_then(|v| v.as_i64())
                    };
                    let writes = match decode_writes(action, &tr.module, tables, &mut current) {
                        Ok(writes) => writes,
                        Err(reason) => {
                            result.effects.push(ActionEffect {
                                action: action.clone(),
                                reason: reason.into(),
                                line,
                            });
                            continue;
                        }
                    };
                    for (key, change) in writes {
                        let Ok(saved) = reader.read(&key) else {
                            result.effects.push(ActionEffect {
                                action: action.clone(),
                                reason: "unavailable_target".into(),
                                line,
                            });
                            continue;
                        };
                        let before = self
                            .pending
                            .iter()
                            .flat_map(|p| &p.writes)
                            .rfind(|w| w.key == key)
                            .map(|w| w.after.clone())
                            .or(saved.clone());
                        let after = change.apply(before.as_ref());
                        // Nothing of that tag in the backpack: the script is a no-op in-game too.
                        if key.value_type == ITEM_TAKE_TYPE && before == Some(after.clone()) {
                            continue;
                        }
                        let blueprint = (key.value_type == ITEM_TYPE)
                            .then(|| tables.item(&tr.module, &key.name))
                            .flatten();
                        if let Some(blueprint) = &blueprint {
                            result
                                .blueprints
                                .insert(key.name.clone(), blueprint.clone());
                        }
                        let (label, label_strref) = if key.value_type == ITEM_TAKE_TYPE {
                            reader.item_label(&key.name)
                        } else if key.value_type == FEAT_TYPE {
                            (None, tables.feat_name_strref(&key.name))
                        } else if area_field(&key.value_type).is_some() {
                            (None, reader.area_object_label(&key))
                        } else if key.value_type == COMPANION_FEAT_TYPE {
                            (
                                None,
                                split_companion_key(&key.name)
                                    .and_then(|(_, feat)| tables.feat_name_strref(feat)),
                            )
                        } else {
                            (
                                blueprint.as_ref().and_then(|b| b.name.clone()),
                                blueprint.as_ref().and_then(|b| b.name_strref),
                            )
                        };
                        candidates.push(VariableWrite {
                            key,
                            saved,
                            before,
                            after,
                            action_index,
                            label,
                            label_strref,
                            derived_from: action.derived_from.clone(),
                            line,
                        });
                    }
                }
            }
            // Items add up the way repeated CreateItemOnObject calls do.
            let mut merged: Vec<VariableWrite> = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                if candidate.key.value_type == ITEM_TYPE
                    && let Some(earlier) = merged.iter_mut().find(|w| w.key == candidate.key)
                {
                    let total =
                        earlier.after.as_i64().unwrap_or(0) + candidate.after.as_i64().unwrap_or(0);
                    earlier.after = Value::from(total);
                    continue;
                }
                merged.push(candidate);
            }
            let candidates = merged;
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
            // A wrapper applied in full (nothing undecoded, every spliced write and action
            // applied) leaves nothing to run by hand.
            let unapplied: BTreeSet<String> = result
                .effects
                .iter()
                .filter_map(|e| e.action.derived_from.clone())
                .collect();
            result.effects.retain(|e| {
                e.reason != "decoded_script"
                    || !e.action.decoded.as_ref().is_some_and(|d| {
                        d.calls.is_empty()
                            && d.conditional_writes.is_empty()
                            && d.unresolved.is_empty()
                            && !unapplied.contains(&derived_call(&e.action))
                    })
            });
        }
        // What comes next: the other entries' lines, evaluated against the save as
        // it will be once this edit and the pending ones are written.
        for write in self.pending.iter().flat_map(|p| &p.writes) {
            reader
                .overlay
                .insert(write.key.clone(), write.after.clone());
        }
        for write in &result.writes {
            reader
                .overlay
                .insert(write.key.clone(), write.after.clone());
        }
        result.next_steps = next_steps(quest, graph, target, &mut reader)?;
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
        tables: &RewardTables,
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
            let preview = self.preview(
                graph,
                tables,
                handler,
                paths,
                player,
                tag,
                target,
                Some(route),
            )?;
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
            let effects: Vec<ConvoFunctor> = preview
                .effects
                .iter()
                .filter(|e| e.reason != "journal_action")
                .map(|e| e.action.clone())
                .collect();
            let mut effect_keys: Vec<String> = effects
                .iter()
                .map(|a| {
                    format!(
                        "{}({})",
                        a.script.to_ascii_lowercase(),
                        Value::from(a.params.clone())
                    )
                })
                .collect();
            effect_keys.sort();
            writes.extend(effect_keys);
            out.push(RouteSummary {
                route,
                module: tr.module.clone(),
                dlg: tr.dlg.clone(),
                speaker: tr.speaker.clone(),
                is_player: tr.is_player,
                text_strref: tr.text_strref,
                spoken: tr.spoken.clone(),
                debug: tr.debug,
                signature: writes.join(";"),
                writes: preview
                    .writes
                    .iter()
                    .map(|w| RouteWrite {
                        name: w.key.name.clone(),
                        value_type: w.key.value_type.clone(),
                        before: w.before.clone(),
                        after: w.after.clone(),
                        label: w.label.clone(),
                        label_strref: w.label_strref,
                    })
                    .collect(),
                effects,
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
    ) -> Result<Vec<PendingQuestEdit>, String> {
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
                        writes.iter().any(|new| {
                            old.key == new.key
                                && old.after != new.after
                                && old.key.value_type != ITEM_TYPE
                                && old.key.value_type != ITEM_TAKE_TYPE
                        })
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
        let (replaced, kept) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|p| p.tag == preview.tag || conflicts.contains(&p.tag));
        self.pending = kept;
        let source = preview.route.and_then(|i| {
            graph
                .quests
                .iter()
                .find(|q| q.tag == preview.tag)?
                .transitions
                .get(i)
                .cloned()
        });
        let blueprints = preview
            .blueprints
            .into_iter()
            .filter(|(resref, _)| {
                writes
                    .iter()
                    .any(|w| w.key.value_type == ITEM_TYPE && w.key.name == *resref)
            })
            .collect();
        self.pending.push(PendingQuestEdit {
            tag: preview.tag,
            target: preview.target,
            saved_state: preview.saved_state,
            source,
            writes,
            blueprints,
            applied: None,
        });
        self.generation = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        Ok(replaced)
    }

    pub fn validate_saved(
        &self,
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        player: usize,
    ) -> Result<(), String> {
        let journal = read_live_journal(handler, player)?;
        let mut reader = VariableReader::new(handler, paths, player);
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

    /// Write every pending edit into the save. Gold and XP rewards are added to the
    /// player as already staged in this batch (so character-tab edits survive) and to
    /// `player.bic` when `primary` is set; the fields written are returned so the
    /// caller can bring an in-memory player up to date.
    pub fn flush(
        &self,
        handler: &mut SaveGameHandler,
        player: usize,
        primary: bool,
        game_data: &GameData,
    ) -> Result<FlushedWrites, String> {
        if self.pending.is_empty() {
            return Ok(FlushedWrites::default());
        }
        let mut locals: Vec<(String, i64)> = self
            .pending
            .iter()
            .map(|p| (journal_variable(&p.tag), i64::from(p.target)))
            .collect();
        let mut rewards: Vec<(String, i64)> = Vec::new();
        let mut items: Vec<(Arc<ItemBlueprint>, i64)> = Vec::new();
        let mut takes: Vec<(String, i64)> = Vec::new();
        let mut feats: Vec<(i32, bool)> = Vec::new();
        let mut party: Vec<(String, bool)> = Vec::new();
        let mut selectable: Vec<(String, bool)> = Vec::new();
        let mut companion_rewards: BTreeMap<String, Vec<(String, i64)>> = BTreeMap::new();
        let mut companion_feats: BTreeMap<String, Vec<(i32, bool)>> = BTreeMap::new();
        for edit in &self.pending {
            for write in &edit.writes {
                if edit.applied.is_some() && applies_to_player(&write.key.value_type) {
                    continue;
                }
                if write.key.value_type == COMPANION_CHARACTER_TYPE {
                    let (ros_name, field) =
                        split_companion_key(&write.key.name).ok_or("Invalid companion key")?;
                    let delta = write.after.as_i64().unwrap_or(0)
                        - write.before.as_ref().and_then(Value::as_i64).unwrap_or(0);
                    let rewards = companion_rewards.entry(ros_name.to_string()).or_default();
                    match rewards.iter_mut().find(|(name, _)| name == field) {
                        Some((_, total)) => *total += delta,
                        None => rewards.push((field.to_string(), delta)),
                    }
                } else if write.key.value_type == COMPANION_FEAT_TYPE {
                    let (ros_name, feat) =
                        split_companion_key(&write.key.name).ok_or("Invalid companion key")?;
                    let feat = feat.parse::<i32>().map_err(|_| "Invalid feat id")?;
                    let owned = write.after.as_i64().unwrap_or(0) != 0;
                    let feats = companion_feats.entry(ros_name.to_string()).or_default();
                    feats.retain(|(id, _)| *id != feat);
                    feats.push((feat, owned));
                } else if write.key.value_type == PARTY_TYPE
                    || write.key.value_type == SELECTABLE_TYPE
                {
                    let flags = if write.key.value_type == PARTY_TYPE {
                        &mut party
                    } else {
                        &mut selectable
                    };
                    let value = write.after.as_i64().unwrap_or(0) != 0;
                    flags.retain(|(name, _)| !name.eq_ignore_ascii_case(&write.key.name));
                    flags.push((write.key.name.clone(), value));
                } else if write.key.value_type == PC_LOCAL_TYPE {
                    let value = write.after.as_i64().ok_or("PC local is not an integer")?;
                    locals.push((write.key.name.clone(), value));
                } else if write.key.value_type == FEAT_TYPE {
                    let feat = write
                        .key
                        .name
                        .parse::<i32>()
                        .map_err(|_| "Invalid feat id")?;
                    let owned = write.after.as_i64().unwrap_or(0) != 0;
                    feats.retain(|(id, _)| *id != feat);
                    feats.push((feat, owned));
                } else if write.key.value_type == CHARACTER_TYPE {
                    let delta = write.after.as_i64().unwrap_or(0)
                        - write.before.as_ref().and_then(Value::as_i64).unwrap_or(0);
                    match rewards.iter_mut().find(|(name, _)| *name == write.key.name) {
                        Some((_, total)) => *total += delta,
                        None => rewards.push((write.key.name.clone(), delta)),
                    }
                } else if write.key.value_type == ITEM_TYPE {
                    let blueprint = edit
                        .blueprints
                        .get(&write.key.name)
                        .ok_or_else(|| format!("Item blueprint {} missing", write.key.name))?;
                    let quantity = write.after.as_i64().unwrap_or(0);
                    if quantity > 0 {
                        items.push((blueprint.clone(), quantity));
                    }
                } else if write.key.value_type == ITEM_TAKE_TYPE {
                    let units = write.before.as_ref().and_then(Value::as_i64).unwrap_or(0)
                        - write.after.as_i64().unwrap_or(0);
                    if units > 0 {
                        takes.push((write.key.name.clone(), units));
                    }
                }
            }
        }
        let bytes = handler.extract_player_data().map_err(|e| e.to_string())?;
        let bytes = write_pc_int_locals(&bytes, player, &locals)?;
        let (bytes, mut written) = add_player_rewards(&bytes, player, &rewards)?;
        let (bytes, item_list) = apply_player_items(&bytes, player, &items, &takes, game_data)?;
        written.extend(item_list);
        let (bytes, feat_list) = set_player_feats(&bytes, player, &feats)?;
        written.extend(feat_list);
        handler
            .update_file("playerlist.ifo", &bytes)
            .map_err(|e| e.to_string())?;
        if primary
            && (!rewards.is_empty() || !items.is_empty() || !takes.is_empty() || !feats.is_empty())
            && let Some(bic) = handler.extract_player_bic().map_err(|e| e.to_string())?
        {
            let bic = add_bic_rewards(&bic, &rewards)?;
            let bic = apply_bic_items(&bic, &items, &takes, game_data)?;
            let bic = set_bic_feats(&bic, &feats)?;
            handler
                .update_file("player.bic", &bic)
                .map_err(|e| e.to_string())?;
        }
        if !party.is_empty()
            && let Some(warning) = roster::apply_party_membership(handler, &party)?
        {
            warn!("Quest party write: {warning}");
        }
        if !selectable.is_empty() {
            roster::apply_selectable(handler, &selectable)?;
        }
        let mut companions = Vec::new();
        let ros_names: BTreeSet<&String> = companion_rewards
            .keys()
            .chain(companion_feats.keys())
            .collect();
        for ros_name in ros_names {
            let stored = handler
                .companion_stored_name(ros_name)
                .map_err(|e| e.to_string())?;
            let bytes = handler.extract_file(&stored).map_err(|e| e.to_string())?;
            let (bytes, fields) = update_companion(
                &bytes,
                companion_rewards
                    .get(ros_name)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                companion_feats
                    .get(ros_name)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            )?;
            handler
                .update_file(&stored, &bytes)
                .map_err(|e| e.to_string())?;
            companions.push((ros_name.clone(), fields));
        }
        let mut globals = Vec::new();
        let mut modules: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
        let mut area_edits: BTreeMap<String, Vec<(AreaField, String, bool)>> = BTreeMap::new();
        for write in self.pending.iter().flat_map(|p| &p.writes) {
            if let Some(field) = area_field(&write.key.value_type) {
                let module = write
                    .key
                    .module
                    .clone()
                    .ok_or("Area object without module")?;
                let edits = area_edits.entry(module).or_default();
                edits.retain(|(f, tag, _)| !(*f == field && *tag == write.key.name));
                edits.push((
                    field,
                    write.key.name.clone(),
                    write.after.as_i64().unwrap_or(0) != 0,
                ));
            }
        }
        for write in self
            .pending
            .iter()
            .flat_map(|p| &p.writes)
            .filter(|w| !writes_save_files(&w.key.value_type))
        {
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
        for module in area_edits.keys() {
            modules.entry(module.clone()).or_default();
        }
        let paths = NWN2Paths::new();
        for (module, updates) in modules {
            let edits = area_edits.remove(&module).unwrap_or_default();
            let (path, bytes) = content::prepare_module_variable_updates(
                handler,
                &updates,
                Some(&module),
                |save| {
                    if edits.is_empty() {
                        return Ok(());
                    }
                    let objects = area_objects::installed_module_objects(&paths, &module)?;
                    area_objects::write_saved(save, &objects, &edits)
                },
            )?;
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid module filename")?;
            handler
                .update_file(name, &bytes)
                .map_err(|e| e.to_string())?;
        }
        Ok(FlushedWrites {
            player: written,
            companions,
        })
    }
}

pub struct VariableReader<'a> {
    handler: &'a SaveGameHandler,
    paths: &'a NWN2Paths,
    player: usize,
    globals: Option<Result<crate::parsers::xml::XmlData, String>>,
    modules: BTreeMap<String, Result<content::ModuleVariables, String>>,
    /// The player's struct in `playerlist.ifo`, for `CHARACTER_TYPE` keys.
    player_fields: Option<Result<indexmap::IndexMap<String, GffValue<'static>>, String>>,
    /// `roster.rst` members, for `PARTY_TYPE` / `SELECTABLE_TYPE` keys.
    roster: Option<Result<Vec<RosterMember>, String>>,
    /// Root structs of companion `.ros` files by lowercase roster name.
    companions: BTreeMap<String, Result<indexmap::IndexMap<String, GffValue<'static>>, String>>,
    /// Installed doors/map pins and the saved `<module>.z`, by module, for
    /// `DOOR_LOCK_TYPE` / `MAP_NOTE_TYPE` keys.
    areas: BTreeMap<String, Result<(ModuleObjects, ErfParser), String>>,
    /// Values that take precedence over the save: pending and previewed writes.
    pub overlay: BTreeMap<VariableKey, Value>,
    /// Journal states (tag -> entry) for `gc_journal_entry`; `None` = unknown.
    pub journal: Option<BTreeMap<String, u32>>,
}

impl<'a> VariableReader<'a> {
    pub fn new(handler: &'a SaveGameHandler, paths: &'a NWN2Paths, player: usize) -> Self {
        Self {
            handler,
            paths,
            player,
            globals: None,
            modules: BTreeMap::new(),
            player_fields: None,
            roster: None,
            companions: BTreeMap::new(),
            areas: BTreeMap::new(),
            overlay: BTreeMap::new(),
            journal: None,
        }
    }

    fn player_fields(&mut self) -> Result<&indexmap::IndexMap<String, GffValue<'static>>, String> {
        self.player_fields
            .get_or_insert_with(|| {
                let bytes = self
                    .handler
                    .extract_player_data()
                    .map_err(|e| e.to_string())?;
                player_struct(&bytes, self.player)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn area_state(&mut self, module: &str) -> Result<(&ModuleObjects, &mut ErfParser), String> {
        let (handler, paths) = (self.handler, self.paths);
        match self.areas.entry(module.to_string()).or_insert_with(|| {
            let objects = area_objects::installed_module_objects(paths, module)?;
            let (_, save) = content::open_module_state(handler, module)?;
            Ok((objects, save))
        }) {
            Ok((objects, save)) => Ok((objects, save)),
            Err(e) => Err(e.clone()),
        }
    }

    /// TLK name of a door or map pin: the pin's note text, else the object's name.
    pub fn area_object_label(&mut self, key: &VariableKey) -> Option<i32> {
        let field = area_field(&key.value_type)?;
        let (objects, _) = self.area_state(key.module.as_deref()?).ok()?;
        objects.unique(field, &key.name).ok()?.label_strref
    }

    /// Display name of the first backpack item carrying `tag`: embedded text, TLK strref.
    pub fn item_label(&mut self, tag: &str) -> (Option<String>, Option<i32>) {
        let Ok(fields) = self.player_fields() else {
            return (None, None);
        };
        backpack_items(fields)
            .find(|item| item_tag_matches(item, tag))
            .map(super::item_blueprints::localized_name)
            .unwrap_or((None, None))
    }

    pub fn read(&mut self, key: &VariableKey) -> Result<Option<Value>, String> {
        if key.value_type == ITEM_TYPE {
            return Ok(None);
        }
        if let Some(value) = self.overlay.get(key) {
            return Ok(Some(value.clone()));
        }
        if key.value_type == CHARACTER_TYPE
            || key.value_type == PC_LOCAL_TYPE
            || key.value_type == FEAT_TYPE
            || key.value_type == ITEM_TAKE_TYPE
        {
            let fields = self.player_fields()?;
            if key.value_type == ITEM_TAKE_TYPE {
                let units: i64 = backpack_items(fields)
                    .filter(|item| item_tag_matches(item, &key.name))
                    .map(|item| item_stack_size(item))
                    .sum();
                return Ok(Some(Value::from(units)));
            }
            if key.value_type == PC_LOCAL_TYPE {
                return Ok(pc_int_local(fields, &key.name));
            }
            if key.value_type == FEAT_TYPE {
                let feat = key.name.parse::<i64>().map_err(|_| "Invalid feat id")?;
                return Ok(Some(Value::from(i64::from(has_feat(fields, feat)))));
            }
            return Ok(fields.get(&key.name).and_then(gff_integer).map(Value::from));
        }
        if key.value_type == COMPANION_CHARACTER_TYPE || key.value_type == COMPANION_FEAT_TYPE {
            let (ros_name, field) =
                split_companion_key(&key.name).ok_or("Invalid companion key")?;
            let fields = self
                .companions
                .entry(ros_name.to_string())
                .or_insert_with(|| {
                    let stored = self
                        .handler
                        .companion_stored_name(ros_name)
                        .map_err(|e| e.to_string())?;
                    let bytes = self
                        .handler
                        .extract_file(&stored)
                        .map_err(|e| e.to_string())?;
                    owned_root(&bytes)
                })
                .as_ref()
                .map_err(Clone::clone)?;
            if key.value_type == COMPANION_FEAT_TYPE {
                let feat = field.parse::<i64>().map_err(|_| "Invalid feat id")?;
                return Ok(Some(Value::from(i64::from(has_feat(fields, feat)))));
            }
            return Ok(fields.get(field).and_then(gff_integer).map(Value::from));
        }
        if let Some(field) = area_field(&key.value_type) {
            let module = key.module.as_deref().ok_or("Area object without module")?;
            let (objects, save) = self.area_state(module)?;
            let object = objects.unique(field, &key.name)?;
            return area_objects::read_saved(save, field, &key.name, object)
                .map(|v| Some(Value::from(v)));
        }
        if key.value_type == PARTY_TYPE || key.value_type == SELECTABLE_TYPE {
            let members = self
                .roster
                .get_or_insert_with(|| {
                    let (_, bytes) = self
                        .handler
                        .extract_roster()
                        .map_err(|e| e.to_string())?
                        .ok_or("Save has no roster file")?;
                    roster::parse_roster_members(bytes)
                })
                .as_ref()
                .map_err(Clone::clone)?;
            let member = members
                .iter()
                .find(|m| m.ros_name.eq_ignore_ascii_case(&key.name))
                .ok_or_else(|| format!("Roster member {} not found", key.name))?;
            let value = if key.value_type == PARTY_TYPE {
                !member.available
            } else {
                member.selectable
            };
            return Ok(Some(Value::from(i64::from(value))));
        }
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
const NEXT_LINES_PER_STATE: usize = 3;

/// The quest's other entries and the playable lines reaching them, each line
/// checked against the reader (which carries this edit's writes as an overlay).
/// One line per speaker and text, the current module and open gates first.
fn next_steps(
    quest: &super::QuestAggregate,
    graph: &SaveGraph,
    target: u32,
    reader: &mut VariableReader<'_>,
) -> Result<Vec<NextStep>, String> {
    let current_module = graph
        .modules
        .iter()
        .find(|m| m.is_current)
        .map(|m| m.name.as_str());
    let gate_rank = |gate: &str| match gate {
        "open" => 0,
        "unknown" => 1,
        _ => 2,
    };
    let mut by_state: BTreeMap<u32, Vec<NextLine>> = BTreeMap::new();
    for (route, tr) in quest.transitions.iter().enumerate() {
        if tr.new_state == target || tr.debug {
            continue;
        }
        let gate = match evaluate_conditions(&tr.gating_conditions, reader, &tr.module)? {
            Some(true) => "open",
            Some(false) => "closed",
            None => "unknown",
        };
        by_state.entry(tr.new_state).or_default().push(NextLine {
            route,
            module: tr.module.clone(),
            dlg: tr.dlg.clone(),
            speaker: tr.speaker.clone(),
            is_player: tr.is_player,
            text_strref: tr.text_strref,
            spoken: tr.spoken.clone(),
            gate: gate.into(),
        });
    }
    Ok(by_state
        .into_iter()
        .map(|(state, mut lines)| {
            lines.sort_by_key(|l| {
                (
                    gate_rank(&l.gate),
                    Some(l.module.as_str()) != current_module,
                    l.route,
                )
            });
            let mut seen: BTreeSet<(String, bool, Option<i32>)> = BTreeSet::new();
            let lines = lines
                .into_iter()
                .filter(|l| {
                    let spoken = l.spoken.as_ref();
                    seen.insert((
                        spoken.map_or(l.speaker.clone(), |s| s.speaker.clone()),
                        spoken.map_or(l.is_player, |s| s.is_player),
                        spoken.map_or(l.text_strref, |s| Some(s.text_strref)),
                    ))
                })
                .take(NEXT_LINES_PER_STATE)
                .collect();
            NextStep { state, lines }
        })
        .collect())
}

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
    if script == "gc_journal_entry" {
        let (Some(quest), Some(check)) = (
            condition.params.first().and_then(Value::as_str),
            condition.params.get(1).and_then(Value::as_str),
        ) else {
            return Ok(None);
        };
        let Some(journal) = &reader.journal else {
            return Ok(None);
        };
        let current = journal.get(quest).copied().unwrap_or(0);
        return Ok(compare_check(i64::from(current), check));
    }
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
    /// Influence scripts: added to the current value, then capped the way
    /// `kinc_companion` caps it (`INFLUENCE_MIN`/`INFLUENCE_MAX`).
    AddClamped {
        delta: i64,
        min: i64,
        max: i64,
    },
}

impl Change {
    pub fn apply(&self, before: Option<&Value>) -> Value {
        let current = || before.and_then(Value::as_i64).unwrap_or(0);
        match self {
            Change::Set(value) => value.clone(),
            Change::Add(delta) => Value::from(current() + delta),
            Change::AddClamped { delta, min, max } => {
                Value::from((current() + delta).clamp(*min, *max))
            }
        }
    }
}

const INFLUENCE_PREFIX: &str = "00_nInfluence";
const INFLUENCE_MIN: i64 = -100;
const INFLUENCE_MAX: i64 = 100;

/// `ginc_companion.INFLUENCE_PREFIX + tag`: the global the influence scripts write.
pub fn influence_variable(tag: &str) -> String {
    format!("{INFLUENCE_PREFIX}{tag}")
}

/// OC `kinc_companion.GetCompanionTagByNumber`, the `nCompanion` table of `ga_influence`.
fn oc_companion_tag(number: i64) -> Option<&'static str> {
    Some(match number {
        1 => "khelgar",
        2 => "neeshka",
        3 => "elanee",
        4 => "qara",
        5 => "sand",
        6 => "grobnar",
        7 => "casavir",
        8 => "bishop",
        9 => "shandra",
        10 => "construct",
        11 => "zhjaeve",
        12 => "ammon_jerro",
        13 => "npc_bevil",
        _ => return None,
    })
}

/// MotB `kinc_companion.GetCompanionTagByCode`: only the first letter of the code counts,
/// except that "one..." is One of Many rather than Okku.
fn motb_companion_tag(code: &str) -> Option<&'static str> {
    let code = code.trim().to_ascii_lowercase();
    Some(match code.chars().next()? {
        's' => "safiya",
        'g' => "gann",
        'o' if code.starts_with("one") => "oneofmany",
        'o' => "okku",
        'd' => "dove",
        _ => return None,
    })
}

/// MotB `kinc_companion.MagnitudeConversion`: magnitude 1..5 in influence points.
fn motb_magnitude(magnitude: i64) -> Option<i64> {
    let points = match magnitude.abs() {
        1 => 6,
        2 => 11,
        3 => 15,
        4 => 25,
        5 => 35,
        _ => return None,
    };
    Some(points * magnitude.signum())
}

/// MotB `SetInfluenceByCode`: Safiya never drops below disillusioned.
fn motb_influence_min(tag: &str) -> i64 {
    if tag == "safiya" { -75 } else { INFLUENCE_MIN }
}

/// Int script parameters arrive as numbers, string-typed ones through `StringToInt`.
fn int_param(action: &ConvoFunctor, index: usize) -> Option<i64> {
    match action.params.get(index)? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => string_to_int(s),
        _ => None,
    }
}

fn influence_key(tag: &str) -> VariableKey {
    VariableKey {
        module: None,
        name: influence_variable(tag),
        value_type: "int".into(),
    }
}

/// `ga_influence` (OC), `ka_influence_magnitude` / `ka_influence_set` (MotB): each ends in
/// one clamped `SetGlobalInt` on the companion's influence global. The MotB band effects
/// (feats, leaving the party) are re-derived by `DoInfluenceEffects` when the companion spawns.
fn decode_influence(
    script: &str,
    action: &ConvoFunctor,
) -> Result<(VariableKey, Change), &'static str> {
    match script {
        "ga_influence" => {
            let tag = int_param(action, 0)
                .and_then(oc_companion_tag)
                .ok_or("invalid_assignment")?;
            let delta = int_param(action, 1).ok_or("invalid_assignment")?;
            if delta == 0 {
                return Err("invalid_assignment");
            }
            Ok((
                influence_key(tag),
                Change::AddClamped {
                    delta,
                    min: INFLUENCE_MIN,
                    max: INFLUENCE_MAX,
                },
            ))
        }
        "ka_influence_magnitude" | "ka_influence_set" => {
            let tag = action
                .params
                .first()
                .and_then(Value::as_str)
                .and_then(motb_companion_tag)
                .ok_or("invalid_assignment")?;
            let amount = int_param(action, 1).ok_or("invalid_assignment")?;
            let min = motb_influence_min(tag);
            let change = if script == "ka_influence_set" {
                Change::Set(Value::from(amount.clamp(min, INFLUENCE_MAX)))
            } else {
                let delta = motb_magnitude(amount).ok_or("invalid_assignment")?;
                Change::AddClamped {
                    delta,
                    min,
                    max: INFLUENCE_MAX,
                }
            };
            Ok((influence_key(tag), change))
        }
        _ => Err("unsupported_action"),
    }
}

/// `VariableKey::value_type` of the player-struct fields reward scripts change.
pub const CHARACTER_TYPE: &str = "character";
/// `VariableKey::value_type` of an item handed to the player: `name` is the
/// blueprint resref, the value the quantity.
pub const ITEM_TYPE: &str = "item";
/// `VariableKey::value_type` of items taken out of the player's backpack
/// (`ga_take_item` / `ga_destroy_item`): `name` is the lowercase item tag, the
/// value how many units of that tag the `ItemList` holds afterwards.
pub const ITEM_TAKE_TYPE: &str = "item_take";
/// `VariableKey::value_type` of an integer local on the PC (`VarTable` of the
/// player's struct), e.g. the `<tag>_XP_QPR` share of quest XP already paid.
pub const PC_LOCAL_TYPE: &str = "pc_local";
/// `VariableKey::value_type` of a feat on the PC: `name` is the `feat.2da` row,
/// the value 1 (in `FeatList`) or 0.
pub const FEAT_TYPE: &str = "feat";
/// `VariableKey::value_type` of a companion's party membership: `name` is the
/// roster name (`RosName`, lowercase), the value 1 (in the active party) or 0
/// (benched). Written as `RosAvailable` in `roster.rst` plus the `.ros` faction.
pub const PARTY_TYPE: &str = "party";
/// `VariableKey::value_type` of a companion's `RosSelectable` flag: `name` is
/// the roster name, the value 1 (pickable on the party screen) or 0 (locked).
pub const SELECTABLE_TYPE: &str = "selectable";
/// `VariableKey::value_type` of an alignment field on a companion's `.ros`:
/// `name` is `<roster name>/<field>` (`khelgar/LawfulChaotic`).
pub const COMPANION_CHARACTER_TYPE: &str = "companion_character";
/// `VariableKey::value_type` of a feat on a companion's `.ros`: `name` is
/// `<roster name>/<feat.2da row>`, the value 1 (in `FeatList`) or 0.
pub const COMPANION_FEAT_TYPE: &str = "companion_feat";
/// `VariableKey::value_type` of a door's lock in the module's saved area state
/// (`ga_lock`, `SetLocked`): `name` is the lowercase door tag, the value 1
/// (locked) or 0. Written as `Locked` on the door in the area `.git` of `<module>.z`.
pub const DOOR_LOCK_TYPE: &str = "door_lock";
/// `VariableKey::value_type` of a map pin (`ga_mapnote`, `SetMapPinEnabled`):
/// `name` is the lowercase waypoint tag, the value 1 (shown) or 0. Written as
/// `MapNoteEnabled` on the waypoint in the area `.git` of `<module>.z`.
pub const MAP_NOTE_TYPE: &str = "map_note";
const GOLD: &str = "Gold";
const EXPERIENCE: &str = "Experience";

/// Write kinds that land in player, roster or companion files rather than in the
/// campaign's variable stores (`globals.xml`, module variables).
pub fn writes_save_files(value_type: &str) -> bool {
    matches!(
        value_type,
        CHARACTER_TYPE
            | ITEM_TYPE
            | ITEM_TAKE_TYPE
            | PC_LOCAL_TYPE
            | FEAT_TYPE
            | PARTY_TYPE
            | SELECTABLE_TYPE
            | COMPANION_CHARACTER_TYPE
            | COMPANION_FEAT_TYPE
            | DOOR_LOCK_TYPE
            | MAP_NOTE_TYPE
    )
}

fn area_field(value_type: &str) -> Option<AreaField> {
    match value_type {
        DOOR_LOCK_TYPE => Some(AreaField::DoorLocked),
        MAP_NOTE_TYPE => Some(AreaField::MapNoteEnabled),
        _ => None,
    }
}
const GOOD_EVIL: &str = "GoodEvil";
const LAWFUL_CHAOTIC: &str = "LawfulChaotic";
/// `ginc_journal.QPR_SUFFIX`.
const QPR_SUFFIX: &str = "_XP_QPR";

fn is_alignment_field(name: &str) -> bool {
    name == GOOD_EVIL || name == LAWFUL_CHAOTIC
}

fn character_key(name: &str) -> VariableKey {
    VariableKey {
        module: None,
        name: name.into(),
        value_type: CHARACTER_TYPE.into(),
    }
}

/// Key of `field` on the player, or on the companion `target` (a creature tag,
/// matched against roster names) when it is not empty.
fn creature_key(target: &str, field: &str) -> VariableKey {
    if target.is_empty() {
        character_key(field)
    } else {
        VariableKey {
            module: None,
            name: format!("{}/{field}", target.to_ascii_lowercase()),
            value_type: COMPANION_CHARACTER_TYPE.into(),
        }
    }
}

/// `<roster name>/<field>` of a companion key.
pub fn split_companion_key(name: &str) -> Option<(&str, &str)> {
    name.split_once('/')
}

/// A script target as a creature tag: empty for the PC speaker; `$`-targets other
/// than the PC (`$OWNER`, `$OBJECT_SELF`, ...) are not creatures we can reach.
fn target_tag(target: &str) -> Result<&str, &'static str> {
    let target = target.trim();
    if target.is_empty() || target == "$PC_SPEAKER" {
        return Ok("");
    }
    if target.starts_with('$') {
        return Err("object_target");
    }
    Ok(target)
}

/// Gold is a DWORD, XP an INT; neither drops below 0 in the engine.
fn reward(name: &str, delta: i64) -> Result<(VariableKey, Change), &'static str> {
    if delta == 0 {
        return Err("no_effect");
    }
    let max = if name == GOLD {
        i64::from(u32::MAX)
    } else {
        i64::from(i32::MAX)
    };
    Ok((
        character_key(name),
        Change::AddClamped { delta, min: 0, max },
    ))
}

/// `ga_give_gold` / `ga_take_gold` / `ga_give_xp` on the PC speaker. The party flag only
/// reaches other PCs in multiplayer; companions get their XP from the engine's party share.
fn decode_reward(
    script: &str,
    action: &ConvoFunctor,
) -> Result<(VariableKey, Change), &'static str> {
    let amount = int_param(action, 0).ok_or("invalid_assignment")?;
    if amount == 0 {
        return Err("no_effect");
    }
    match script {
        "ga_give_gold" if amount > 0 => reward(GOLD, amount),
        "ga_take_gold" if amount > 0 => reward(GOLD, -amount),
        "ga_give_gold" | "ga_take_gold" => Err("invalid_assignment"),
        "ga_give_xp" => reward(EXPERIENCE, amount),
        _ => Err("unsupported_action"),
    }
}

/// `ga_alignment(iActType, bLawChaosAxis)` per `ginc_alignment`: act types 1..4 shift
/// by 1, 3, 10 or 70 points towards good/lawful (negative towards evil/chaotic); the
/// engine keeps both axes within 0..100.
fn decode_alignment(action: &ConvoFunctor) -> Result<(VariableKey, Change), &'static str> {
    let act = int_param(action, 0).ok_or("invalid_assignment")?;
    let magnitude = match act.abs() {
        1 => 1,
        2 => 3,
        3 => 10,
        4 => 70,
        _ => return Err("no_effect"),
    };
    let field = if int_param(action, 1).unwrap_or(0) == 0 {
        GOOD_EVIL
    } else {
        LAWFUL_CHAOTIC
    };
    Ok((
        character_key(field),
        Change::AddClamped {
            delta: magnitude * act.signum(),
            min: 0,
            max: 100,
        },
    ))
}

/// `ga_give_feat(sTarg, nFeat, bCheckReq, bAllPartyMembers)` / `ga_remove_feat(sTarg,
/// nFeat, bAllPartyMembers)`: an empty target is the PC speaker; the party flag only
/// reaches other PCs in multiplayer. The prerequisite check is not reproduced.
fn decode_feat(script: &str, action: &ConvoFunctor) -> Result<(VariableKey, Change), &'static str> {
    let target = target_tag(action.params.first().and_then(Value::as_str).unwrap_or(""))?;
    let feat = int_param(action, 1).ok_or("invalid_assignment")?;
    if feat < 0 {
        return Err("invalid_assignment");
    }
    let owned = i64::from(script == "ga_give_feat");
    let key = if target.is_empty() {
        VariableKey {
            module: None,
            name: feat.to_string(),
            value_type: FEAT_TYPE.into(),
        }
    } else {
        VariableKey {
            module: None,
            name: format!("{}/{feat}", target.to_ascii_lowercase()),
            value_type: COMPANION_FEAT_TYPE.into(),
        }
    };
    Ok((key, Change::Set(Value::from(owned))))
}

/// `ga_roster_party_add` / `ga_party_add` (join the PC's party), `ga_roster_party_remove`
/// (bench) and `ga_roster_selectable` (party-screen lock). Roster names are matched
/// case-insensitively against `roster.rst`; an unknown member fails at read time.
/// The party limit (`RosPartyLimit`) is not enforced, exactly like `ga_party_add`.
fn decode_party(
    script: &str,
    action: &ConvoFunctor,
) -> Result<(VariableKey, Change), &'static str> {
    let name = action
        .params
        .first()
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("invalid_assignment")?;
    let (value_type, value) = match script {
        "ga_roster_party_add" | "ga_party_add" => (PARTY_TYPE, 1),
        "ga_roster_party_remove" => (PARTY_TYPE, 0),
        "ga_roster_selectable" => (
            SELECTABLE_TYPE,
            i64::from(int_param(action, 1).ok_or("invalid_assignment")? != 0),
        ),
        _ => return Err("unsupported_action"),
    };
    Ok((
        VariableKey {
            module: None,
            name: name.to_ascii_lowercase(),
            value_type: value_type.into(),
        },
        Change::Set(Value::from(value)),
    ))
}

/// `ga_lock(tag, bLock)`, `ga_door_open(tag)` (unlocks; the swing itself is not
/// saved state), `ga_door_close(tag, 1)` (locks) and `ga_mapnote(tag, bActive)` on
/// an object of the line's module. `$OWNER`-style and empty targets are not tags.
fn decode_area_object(
    script: &str,
    action: &ConvoFunctor,
    module: &str,
) -> Result<(VariableKey, Change), &'static str> {
    let tag = action
        .params
        .first()
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("invalid_assignment")?;
    if tag.starts_with('$') {
        return Err("object_target");
    }
    let flag = |index: usize| {
        int_param(action, index)
            .map(|n| i64::from(n != 0))
            .ok_or("invalid_assignment")
    };
    let (value_type, value) = match script {
        "ga_lock" => (DOOR_LOCK_TYPE, flag(1)?),
        "ga_door_open" => (DOOR_LOCK_TYPE, 0),
        "ga_door_close" if int_param(action, 1) == Some(1) => (DOOR_LOCK_TYPE, 1),
        "ga_mapnote" => (MAP_NOTE_TYPE, flag(1)?),
        _ => return Err("unsupported_action"),
    };
    Ok((
        VariableKey {
            module: Some(module.to_string()),
            name: tag.to_ascii_lowercase(),
            value_type: value_type.into(),
        },
        Change::Set(Value::from(value)),
    ))
}

/// nwscript `AdjustAlignment(oSubject, nAlignment, nShift)` decoded out of a wrapper
/// script as `(nAlignment, nShift[, tag])`: `ALIGNMENT_LAWFUL/CHAOTIC/GOOD/EVIL` shift
/// one axis, `ALIGNMENT_ALL` adds the shift to both and `ALIGNMENT_NEUTRAL` moves
/// both towards 50, never past it. A third parameter names the companion instead
/// of the player.
fn decode_alignment_points(
    action: &ConvoFunctor,
    current: &mut dyn FnMut(&VariableKey) -> Option<i64>,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    let alignment = int_param(action, 0).ok_or("invalid_assignment")?;
    let shift = int_param(action, 1).ok_or("invalid_assignment")?;
    if shift <= 0 {
        return Err("no_effect");
    }
    let target = target_tag(action.params.get(2).and_then(Value::as_str).unwrap_or(""))?;
    let clamped = |field: &str, delta: i64| {
        (
            creature_key(target, field),
            Change::AddClamped {
                delta,
                min: 0,
                max: 100,
            },
        )
    };
    let writes = match alignment {
        0 => vec![clamped(LAWFUL_CHAOTIC, shift), clamped(GOOD_EVIL, shift)],
        1 => [LAWFUL_CHAOTIC, GOOD_EVIL]
            .into_iter()
            .filter_map(|field| {
                let value = current(&creature_key(target, field)).unwrap_or(50);
                let delta = (50 - value).clamp(-shift, shift);
                (delta != 0).then(|| clamped(field, delta))
            })
            .collect(),
        2 => vec![clamped(LAWFUL_CHAOTIC, shift)],
        3 => vec![clamped(LAWFUL_CHAOTIC, -shift)],
        4 => vec![clamped(GOOD_EVIL, shift)],
        5 => vec![clamped(GOOD_EVIL, -shift)],
        _ => return Err("invalid_assignment"),
    };
    if writes.is_empty() {
        return Err("no_effect");
    }
    Ok(writes)
}

/// `ga_give_partial_quest_xp(tag, percent)` per `ginc_journal.RewardCappedQuestXP`: the
/// share already paid is the PC local `<tag>_XP_QPR`; the award is capped at 100% in
/// total, the XP is `quest XP * share / 100` and the share paid is tracked.
fn decode_partial_quest_xp(
    action: &ConvoFunctor,
    tables: &RewardTables,
    rewarded: i64,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    let tag = action
        .params
        .first()
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("invalid_assignment")?;
    let quest_xp = tables.quest_xp(tag).ok_or("unavailable_target")?;
    let mut percent = int_param(action, 1).ok_or("invalid_assignment")?;
    if rewarded + percent > 100 {
        percent = 100 - rewarded;
        if percent < 0 {
            return Err("no_effect");
        }
    }
    let xp = quest_xp * percent / 100;
    let mut writes = Vec::new();
    if xp != 0 {
        writes.push(reward(EXPERIENCE, xp)?);
    }
    if percent != 0 {
        writes.push((
            VariableKey {
                module: None,
                name: format!("{tag}{QPR_SUFFIX}"),
                value_type: PC_LOCAL_TYPE.into(),
            },
            Change::Add(percent),
        ));
    }
    if writes.is_empty() {
        return Err("no_effect");
    }
    Ok(writes)
}

/// `decode_action` for every write of an action: `ga_give_partial_quest_xp` changes
/// XP and the paid share together and needs the share's current value from `current`.
pub fn decode_writes(
    action: &ConvoFunctor,
    module: &str,
    tables: &RewardTables,
    current: &mut dyn FnMut(&VariableKey) -> Option<i64>,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    if action.script.eq_ignore_ascii_case("AdjustAlignment") {
        return decode_alignment_points(action, current);
    }
    if action
        .script
        .eq_ignore_ascii_case("ga_give_partial_quest_xp")
    {
        let tag = action
            .params
            .first()
            .and_then(Value::as_str)
            .ok_or("invalid_assignment")?;
        let rewarded = current(&VariableKey {
            module: None,
            name: format!("{tag}{QPR_SUFFIX}"),
            value_type: PC_LOCAL_TYPE.into(),
        })
        .unwrap_or(0);
        return decode_partial_quest_xp(action, tables, rewarded);
    }
    if action.script.eq_ignore_ascii_case("ga_take_item")
        || action.script.eq_ignore_ascii_case("ga_destroy_item")
    {
        return decode_item_take(action).map(|write| write.into_iter().collect());
    }
    decode_action(action, module, tables).map(|write| vec![write])
}

/// Campaign-side data the reward scripts look up at runtime.
pub struct RewardTables<'a> {
    /// Journal category XP by lowercase quest tag (`GetJournalQuestExperience`).
    quest_xp: BTreeMap<String, i64>,
    /// Campaign folder holding `k_xp_awards.2da` (`ga_xp_award_2da_entry`).
    campaign_path: Option<PathBuf>,
    xp_awards: OnceCell<Option<TDAParser>>,
    /// Item blueprints for `ga_give_item`.
    items: ItemBlueprints<'a>,
    /// `feat.2da` with overrides, for the names of `ga_give_feat` writes.
    resources: Option<&'a ResourceManager>,
    feats: OnceCell<Option<Arc<TDAParser>>>,
}

impl<'a> RewardTables<'a> {
    pub fn from_graph(graph: &SaveGraph, resources: Option<&'a ResourceManager>) -> Self {
        Self {
            quest_xp: graph
                .quests
                .iter()
                .map(|q| (q.tag.to_ascii_lowercase(), i64::from(q.category.xp)))
                .collect(),
            campaign_path: graph.campaign.campaign_path.as_deref().map(PathBuf::from),
            xp_awards: OnceCell::new(),
            items: ItemBlueprints::new(graph, resources),
            resources,
            feats: OnceCell::new(),
        }
    }

    #[cfg(test)]
    fn empty() -> Self {
        Self {
            quest_xp: BTreeMap::new(),
            campaign_path: None,
            xp_awards: OnceCell::new(),
            items: ItemBlueprints::detached(),
            resources: None,
            feats: OnceCell::new(),
        }
    }

    fn item(&self, module: &str, resref: &str) -> Option<Arc<ItemBlueprint>> {
        self.items.get(module, resref)
    }

    /// `feat.2da` `FEAT` name strref of a feat row.
    fn feat_name_strref(&self, feat: &str) -> Option<i32> {
        let table = self
            .feats
            .get_or_init(|| self.resources?.get_2da_with_overrides("feat").ok())
            .as_ref()?;
        let row = feat.parse::<usize>().ok()?;
        table
            .get_cell_by_name(row, "FEAT")
            .ok()
            .flatten()?
            .parse()
            .ok()
    }

    fn quest_xp(&self, tag: &str) -> Option<i64> {
        self.quest_xp.get(&tag.to_ascii_lowercase()).copied()
    }

    /// `k_xp_awards.2da` row `XP`, read the way `ginc_journal.AwardXP` does (`StringToInt`).
    fn xp_award(&self, row: i64) -> Option<i64> {
        let table = self
            .xp_awards
            .get_or_init(|| load_2da(self.campaign_path.as_deref()?.join("k_xp_awards.2da")))
            .as_ref()?;
        let row = usize::try_from(row).ok()?;
        let cell = table.get_cell_by_name(row, "XP").ok()?;
        Some(string_to_int(cell.unwrap_or("")).unwrap_or(0))
    }
}

fn load_2da(path: PathBuf) -> Option<TDAParser> {
    let mut parser = TDAParser::new();
    parser.parse_from_file(&path).ok()?;
    Some(parser)
}

/// `decode_assignment` plus the reward scripts whose amount lives outside the dialogue:
/// `ga_give_quest_xp(tag)` awards the journal category XP, `ga_xp_award_2da_entry(row)`
/// the campaign's `k_xp_awards.2da` entry.
pub fn decode_action(
    action: &ConvoFunctor,
    module: &str,
    tables: &RewardTables,
) -> Result<(VariableKey, Change), &'static str> {
    let script = action.script.to_ascii_lowercase();
    match script.as_str() {
        // `ga_give_item(sTemplate, nQuantity, bAllPartyMembers)`: nQuantity below 1 is 1;
        // the party flag only reaches other PCs in multiplayer.
        "ga_give_item" => {
            let resref = action
                .params
                .first()
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or("invalid_assignment")?
                .to_ascii_lowercase();
            let quantity = int_param(action, 1).unwrap_or(1).max(1);
            if tables.item(module, &resref).is_none() {
                return Err("unknown_item");
            }
            Ok((
                VariableKey {
                    module: None,
                    name: resref,
                    value_type: ITEM_TYPE.into(),
                },
                Change::Set(Value::from(quantity)),
            ))
        }
        "ga_give_quest_xp" => {
            let tag = action
                .params
                .first()
                .and_then(Value::as_str)
                .ok_or("invalid_assignment")?;
            let xp = tables.quest_xp(tag).ok_or("unavailable_target")?;
            reward(EXPERIENCE, xp)
        }
        "ga_xp_award_2da_entry" => {
            let row = int_param(action, 0).ok_or("invalid_assignment")?;
            let xp = tables.xp_award(row).ok_or("unavailable_target")?;
            reward(EXPERIENCE, xp)
        }
        _ => decode_assignment(action, module),
    }
}

/// `ga_take_item(sTag, nQuantity, bAllPartyMembers)` / `ga_destroy_item(sTag, nQuantity,
/// bPCFaction)`: both match the tag in the backpack (`GetFirstItemInInventory`, never the
/// equipped slots) and a negative quantity means every unit. `ga_take_item` reads 0 as 1,
/// `ga_destroy_item` does nothing for 0 (`None`). The party flags only reach other PCs
/// (`ga_take_item`) or companions once the PC runs out (`ga_destroy_item`); the PC's
/// backpack is the only one written. Units are removed rather than `ga_take_item`'s whole
/// stacks; both agree on unstackable quest items.
fn decode_item_take(action: &ConvoFunctor) -> Result<Option<(VariableKey, Change)>, &'static str> {
    let tag = action
        .params
        .first()
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("invalid_assignment")?
        .to_ascii_lowercase();
    let default = i64::from(action.script.eq_ignore_ascii_case("ga_take_item"));
    let change = match int_param(action, 1).filter(|q| *q != 0).unwrap_or(default) {
        0 => return Ok(None),
        q if q < 0 => Change::Set(Value::from(0)),
        q => Change::AddClamped {
            delta: -q,
            min: 0,
            max: i64::MAX,
        },
    };
    Ok(Some((
        VariableKey {
            module: None,
            name: tag,
            value_type: ITEM_TAKE_TYPE.into(),
        },
        change,
    )))
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
        "ga_influence" | "ka_influence_magnitude" | "ka_influence_set" => {
            return decode_influence(&script, action);
        }
        "ga_give_gold" | "ga_take_gold" | "ga_give_xp" => return decode_reward(&script, action),
        "ga_alignment" => return decode_alignment(action),
        "ga_give_feat" | "ga_remove_feat" => return decode_feat(&script, action),
        "ga_roster_party_add"
        | "ga_party_add"
        | "ga_roster_party_remove"
        | "ga_roster_selectable" => return decode_party(&script, action),
        "ga_lock" | "ga_door_open" | "ga_door_close" | "ga_mapnote" => {
            return decode_area_object(&script, action, module);
        }
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

fn gff_integer(value: &GffValue) -> Option<i64> {
    Some(match value {
        GffValue::Byte(v) => i64::from(*v),
        GffValue::Word(v) => i64::from(*v),
        GffValue::Short(v) => i64::from(*v),
        GffValue::Dword(v) => i64::from(*v),
        GffValue::Int(v) => i64::from(*v),
        GffValue::Dword64(v) => i64::try_from(*v).ok()?,
        GffValue::Int64(v) => *v,
        _ => return None,
    })
}

fn owned_root(bytes: &[u8]) -> Result<indexmap::IndexMap<String, GffValue<'static>>, String> {
    let parser = GffParser::from_bytes(bytes.to_vec()).map_err(|e| e.to_string())?;
    Ok(parser
        .read_struct_fields(0)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect())
}

/// The player's own struct out of `playerlist.ifo` (`Mod_PlayerList[player]`).
fn player_struct(
    bytes: &[u8],
    player: usize,
) -> Result<indexmap::IndexMap<String, GffValue<'static>>, String> {
    let mut root = owned_root(bytes)?;
    let Some(GffValue::ListOwned(mut players)) = root.swap_remove("Mod_PlayerList") else {
        return Err("Mod_PlayerList missing".into());
    };
    if players.len() <= player {
        return Err("Primary player missing".into());
    }
    Ok(players.swap_remove(player))
}

/// Add `delta` to an integer field, floored at 0 and kept in the field's own GFF type.
fn add_to_field(
    fields: &mut indexmap::IndexMap<String, GffValue<'static>>,
    name: &str,
    delta: i64,
) -> Result<GffValue<'static>, String> {
    let current = fields.get(name);
    let before = match current {
        Some(value) => {
            gff_integer(value).ok_or_else(|| format!("{name} is not an integer field"))?
        }
        None => 0,
    };
    let max = if is_alignment_field(name) {
        100
    } else {
        i64::MAX
    };
    let after = (before + delta).clamp(0, max);
    let value = match current {
        Some(GffValue::Byte(_)) => GffValue::Byte(u8::try_from(after).unwrap_or(u8::MAX)),
        Some(GffValue::Word(_)) => GffValue::Word(u16::try_from(after).unwrap_or(u16::MAX)),
        Some(GffValue::Short(_)) => GffValue::Short(i16::try_from(after).unwrap_or(i16::MAX)),
        Some(GffValue::Int(_)) => GffValue::Int(i32::try_from(after).unwrap_or(i32::MAX)),
        Some(GffValue::Dword(_)) => GffValue::Dword(u32::try_from(after).unwrap_or(u32::MAX)),
        Some(GffValue::Int64(_)) => GffValue::Int64(after),
        Some(GffValue::Dword64(_)) => GffValue::Dword64(u64::try_from(after).unwrap_or(0)),
        None if name == GOLD => GffValue::Dword(u32::try_from(after).unwrap_or(u32::MAX)),
        None if is_alignment_field(name) => GffValue::Byte(u8::try_from(after).unwrap_or(100)),
        None => GffValue::Int(i32::try_from(after).unwrap_or(i32::MAX)),
        Some(_) => return Err(format!("{name} has an unexpected storage type")),
    };
    fields.insert(name.into(), value.clone());
    Ok(value)
}

/// Player fields as written by a reward flush: `(field name, value)`.
pub type WrittenFields = Vec<(String, GffValue<'static>)>;

/// Everything a quest flush wrote onto creatures, for mirroring into loaded characters.
#[derive(Debug, Default)]
pub struct FlushedWrites {
    pub player: WrittenFields,
    /// Per companion (lowercase roster name) fields written to its `.ros`.
    pub companions: Vec<(String, WrittenFields)>,
}

/// Apply reward deltas to the player's struct in `playerlist.ifo`; returns the new bytes
/// and the fields as written.
pub fn add_player_rewards(
    bytes: &[u8],
    player: usize,
    rewards: &[(String, i64)],
) -> Result<(Vec<u8>, WrittenFields), String> {
    if rewards.is_empty() {
        return Ok((bytes.to_vec(), Vec::new()));
    }
    let mut root = owned_root(bytes)?;
    let Some(GffValue::ListOwned(players)) = root.get_mut("Mod_PlayerList") else {
        return Err("Mod_PlayerList missing".into());
    };
    let fields = players.get_mut(player).ok_or("Primary player missing")?;
    let mut written = Vec::with_capacity(rewards.len());
    for (name, delta) in rewards {
        written.push((name.clone(), add_to_field(fields, name, *delta)?));
    }
    let bytes = GffWriter::new("IFO ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())?;
    Ok((bytes, written))
}

/// Same deltas on the root struct of `player.bic`.
pub fn add_bic_rewards(bytes: &[u8], rewards: &[(String, i64)]) -> Result<Vec<u8>, String> {
    let mut root = owned_root(bytes)?;
    for (name, delta) in rewards {
        add_to_field(&mut root, name, *delta)?;
    }
    GffWriter::new("BIC ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())
}

/// Give the reward items and take the turn-ins in the player's `ItemList` in
/// `playerlist.ifo`; returns the new bytes and the list as written.
pub fn apply_player_items(
    bytes: &[u8],
    player: usize,
    items: &[(Arc<ItemBlueprint>, i64)],
    takes: &[(String, i64)],
    game_data: &GameData,
) -> Result<(Vec<u8>, WrittenFields), String> {
    if items.is_empty() && takes.is_empty() {
        return Ok((bytes.to_vec(), Vec::new()));
    }
    let mut root = owned_root(bytes)?;
    let Some(GffValue::ListOwned(players)) = root.get_mut("Mod_PlayerList") else {
        return Err("Mod_PlayerList missing".into());
    };
    let slot = players.get_mut(player).ok_or("Primary player missing")?;
    let mut fields = give_items(std::mem::take(slot), items, game_data)?;
    take_items(&mut fields, takes);
    let list = fields
        .get("ItemList")
        .cloned()
        .ok_or("ItemList missing after changing items")?;
    *slot = fields;
    let bytes = GffWriter::new("IFO ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())?;
    Ok((bytes, vec![("ItemList".into(), list)]))
}

/// Same item changes on the root struct of `player.bic`.
pub fn apply_bic_items(
    bytes: &[u8],
    items: &[(Arc<ItemBlueprint>, i64)],
    takes: &[(String, i64)],
    game_data: &GameData,
) -> Result<Vec<u8>, String> {
    if items.is_empty() && takes.is_empty() {
        return Ok(bytes.to_vec());
    }
    let mut root = give_items(owned_root(bytes)?, items, game_data)?;
    take_items(&mut root, takes);
    GffWriter::new("BIC ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())
}

/// `CreateItemOnObject` one at a time, the way `ga_give_item` loops: stackable items
/// fill stacks up to `baseitems.Stacking`, everything else is one entry per copy. Uses
/// the Inventory tab's add path so identification and stacking match.
fn give_items(
    fields: indexmap::IndexMap<String, GffValue<'static>>,
    items: &[(Arc<ItemBlueprint>, i64)],
    game_data: &GameData,
) -> Result<indexmap::IndexMap<String, GffValue<'static>>, String> {
    let mut character = Character::from_gff(fields);
    give_items_to(&mut character, items, game_data)?;
    Ok(character.into_gff())
}

fn give_items_to(
    character: &mut Character,
    items: &[(Arc<ItemBlueprint>, i64)],
    game_data: &GameData,
) -> Result<(), String> {
    for (blueprint, quantity) in items {
        let base_item = blueprint
            .fields
            .get("BaseItem")
            .and_then(gff_integer)
            .and_then(|v| i32::try_from(v).ok())
            .unwrap_or(0);
        let max_stack = character
            .get_base_item_data(base_item, game_data)
            .map_or(1, |d| d.max_stack)
            .max(1);
        let mut remaining = *quantity;
        while remaining > 0 {
            let count = remaining.min(i64::from(max_stack));
            let mut item = blueprint.fields.clone();
            if max_stack > 1 {
                item.insert(
                    "StackSize".into(),
                    crate::character::typed_stack_size(
                        item.get("StackSize"),
                        i32::try_from(count).unwrap_or(i32::MAX),
                    ),
                );
            }
            character
                .add_item_from_struct(item, game_data)
                .map_err(|e| e.to_string())?;
            remaining -= count;
        }
    }
    Ok(())
}

fn backpack_items<'f>(
    fields: &'f indexmap::IndexMap<String, GffValue<'static>>,
) -> impl Iterator<Item = &'f indexmap::IndexMap<String, GffValue<'static>>> {
    match fields.get("ItemList") {
        Some(GffValue::ListOwned(list)) => list.iter(),
        _ => [].iter(),
    }
}

fn item_tag_matches(item: &indexmap::IndexMap<String, GffValue<'static>>, tag: &str) -> bool {
    matches!(item.get("Tag"), Some(GffValue::String(t)) if t.eq_ignore_ascii_case(tag))
}

fn item_stack_size(item: &indexmap::IndexMap<String, GffValue<'static>>) -> i64 {
    item.get("StackSize")
        .and_then(gff_integer)
        .unwrap_or(1)
        .max(1)
}

/// `ga_destroy_item`'s loop: walk the backpack in order, drop whole stacks of the tag
/// while they fit into the remaining count, shrink the stack that does not.
fn take_items(fields: &mut indexmap::IndexMap<String, GffValue<'static>>, takes: &[(String, i64)]) {
    for (tag, units) in takes {
        take_units(fields, tag, *units);
    }
}

/// Take `units` of `tag` out of the backpack; returns what was removed, each item
/// carrying the removed units as its `StackSize`.
fn take_units(
    fields: &mut indexmap::IndexMap<String, GffValue<'static>>,
    tag: &str,
    units: i64,
) -> Vec<indexmap::IndexMap<String, GffValue<'static>>> {
    let mut removed = Vec::new();
    let Some(GffValue::ListOwned(list)) = fields.get_mut("ItemList") else {
        return removed;
    };
    let mut remaining = units;
    list.retain_mut(|item| {
        if remaining <= 0 || !item_tag_matches(item, tag) {
            return true;
        }
        let stack = item_stack_size(item);
        if stack <= remaining {
            remaining -= stack;
            removed.push(item.clone());
            return false;
        }
        let mut taken = item.clone();
        taken.insert(
            "StackSize".into(),
            crate::character::typed_stack_size(
                item.get("StackSize"),
                i32::try_from(remaining).unwrap_or(i32::MAX),
            ),
        );
        removed.push(taken);
        let left = i32::try_from(stack - remaining).unwrap_or(i32::MAX);
        item.insert(
            "StackSize".into(),
            crate::character::typed_stack_size(item.get("StackSize"), left),
        );
        remaining = 0;
        true
    });
    removed
}

/// Apply the player's writes of a staged edit (`applies_to_player`) to the loaded
/// player the way `flush` would write them; returns the changes for `revert_on_player`.
pub fn apply_to_player(
    edit: &PendingQuestEdit,
    character: &mut Character,
    game_data: &GameData,
) -> Result<Vec<MemoryChange>, String> {
    let mut changes = Vec::new();
    for write in &edit.writes {
        match write.key.value_type.as_str() {
            CHARACTER_TYPE => {
                let delta = write.after.as_i64().unwrap_or(0)
                    - write.before.as_ref().and_then(Value::as_i64).unwrap_or(0);
                let fields = character.gff_mut();
                let old = fields
                    .get(&write.key.name)
                    .and_then(gff_integer)
                    .unwrap_or(0);
                let new = add_to_field(fields, &write.key.name, delta)?;
                changes.push(MemoryChange::Field {
                    name: write.key.name.clone(),
                    delta: gff_integer(&new).unwrap_or(old) - old,
                });
            }
            FEAT_TYPE => {
                let id = write
                    .key
                    .name
                    .parse::<i32>()
                    .map_err(|_| "Invalid feat id")?;
                let owned = write.after.as_i64().unwrap_or(0) != 0;
                if owned != has_feat(character.gff(), i64::from(id)) {
                    set_feat(character, id, owned);
                    changes.push(MemoryChange::Feat { id, added: owned });
                }
            }
            ITEM_TYPE => {
                let blueprint = edit
                    .blueprints
                    .get(&write.key.name)
                    .ok_or_else(|| format!("Item blueprint {} missing", write.key.name))?;
                let units = write.after.as_i64().unwrap_or(0);
                if units > 0 {
                    give_items_to(character, &[(blueprint.clone(), units)], game_data)?;
                    let tag = match blueprint.fields.get("Tag") {
                        Some(GffValue::String(t)) => t.to_string(),
                        _ => String::new(),
                    };
                    changes.push(MemoryChange::ItemsGiven { tag, units });
                }
            }
            ITEM_TAKE_TYPE => {
                let units = write.before.as_ref().and_then(Value::as_i64).unwrap_or(0)
                    - write.after.as_i64().unwrap_or(0);
                if units > 0 {
                    let items = take_units(character.gff_mut(), &write.key.name, units);
                    changes.push(MemoryChange::ItemsTaken { items });
                }
            }
            _ => {}
        }
    }
    Ok(changes)
}

/// Take back what `apply_to_player` changed, newest first.
pub fn revert_on_player(changes: &[MemoryChange], character: &mut Character) -> Result<(), String> {
    for change in changes.iter().rev() {
        match change {
            MemoryChange::Field { name, delta } => {
                add_to_field(character.gff_mut(), name, -delta)?;
            }
            MemoryChange::Feat { id, added } => set_feat(character, *id, !added),
            MemoryChange::ItemsGiven { tag, units } => {
                take_units(character.gff_mut(), tag, *units);
            }
            MemoryChange::ItemsTaken { items } => {
                let fields = character.gff_mut();
                match fields
                    .entry("ItemList".to_string())
                    .or_insert_with(|| GffValue::ListOwned(Vec::new()))
                {
                    GffValue::ListOwned(list) => list.extend(items.iter().cloned()),
                    _ => return Err("ItemList is not a list".into()),
                }
            }
        }
    }
    Ok(())
}

fn set_feat(character: &mut Character, id: i32, owned: bool) {
    if owned {
        let _ = character.add_feat_with_source(FeatId(id), FeatSource::Unknown);
    } else {
        let _ = character.remove_feat(FeatId(id));
    }
}

fn has_feat(fields: &indexmap::IndexMap<String, GffValue<'static>>, feat: i64) -> bool {
    match fields.get("FeatList") {
        Some(GffValue::ListOwned(list)) => list
            .iter()
            .any(|entry| entry.get("Feat").and_then(gff_integer) == Some(feat)),
        _ => false,
    }
}

/// `FeatAdd` / `FeatRemove` on the player's struct in `playerlist.ifo`; returns the
/// new bytes and the list as written.
pub fn set_player_feats(
    bytes: &[u8],
    player: usize,
    feats: &[(i32, bool)],
) -> Result<(Vec<u8>, WrittenFields), String> {
    if feats.is_empty() {
        return Ok((bytes.to_vec(), Vec::new()));
    }
    let mut root = owned_root(bytes)?;
    let Some(GffValue::ListOwned(players)) = root.get_mut("Mod_PlayerList") else {
        return Err("Mod_PlayerList missing".into());
    };
    let slot = players.get_mut(player).ok_or("Primary player missing")?;
    let fields = set_feats(std::mem::take(slot), feats);
    let list = fields
        .get("FeatList")
        .cloned()
        .ok_or("FeatList missing after changing feats")?;
    *slot = fields;
    let bytes = GffWriter::new("IFO ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())?;
    Ok((bytes, vec![("FeatList".into(), list)]))
}

/// Alignment deltas and feat changes on a companion's `.ros` root, keeping the
/// file's type, version and root struct id; returns the new bytes and the fields
/// as written.
fn update_companion(
    bytes: &[u8],
    rewards: &[(String, i64)],
    feats: &[(i32, bool)],
) -> Result<(Vec<u8>, WrittenFields), String> {
    let parser = GffParser::from_bytes(bytes.to_vec()).map_err(|e| e.to_string())?;
    let file_type = parser.file_type.clone();
    let file_version = parser.file_version.clone();
    let root_struct_id = parser.get_struct_id(0).map_err(|e| e.to_string())?;
    let mut root: indexmap::IndexMap<String, GffValue<'static>> = parser
        .read_struct_fields(0)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    let mut written = Vec::new();
    for (name, delta) in rewards {
        let value = add_to_field(&mut root, name, *delta)?;
        written.push((name.clone(), value));
    }
    if !feats.is_empty() {
        root = set_feats(root, feats);
        let list = root
            .get("FeatList")
            .cloned()
            .ok_or("FeatList missing after changing feats")?;
        written.push(("FeatList".into(), list));
    }
    let bytes = GffWriter::new(&file_type, &file_version)
        .write_with_struct_id(root, root_struct_id)
        .map_err(|e| e.to_string())?;
    Ok((bytes, written))
}

/// Same feat changes on the root struct of `player.bic`.
pub fn set_bic_feats(bytes: &[u8], feats: &[(i32, bool)]) -> Result<Vec<u8>, String> {
    if feats.is_empty() {
        return Ok(bytes.to_vec());
    }
    let root = set_feats(owned_root(bytes)?, feats);
    GffWriter::new("BIC ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())
}

/// Uses the Feats tab's add/remove path; a feat already in (or absent from) the list
/// is left alone, as `FeatAdd` / `FeatRemove` do.
fn set_feats(
    fields: indexmap::IndexMap<String, GffValue<'static>>,
    feats: &[(i32, bool)],
) -> indexmap::IndexMap<String, GffValue<'static>> {
    let mut character = Character::from_gff(fields);
    for (feat, owned) in feats {
        set_feat(&mut character, *feat, *owned);
    }
    let mut fields = character.into_gff();
    fields
        .entry("FeatList".into())
        .or_insert_with(|| GffValue::ListOwned(Vec::new()));
    fields
}

/// The PC local the journal keeps a quest's state in.
fn journal_variable(tag: &str) -> String {
    format!("NW_JOURNAL_ENTRY{tag}")
}

/// Value of the integer local `name` on the player's struct, `None` when unset.
fn pc_int_local(
    fields: &indexmap::IndexMap<String, GffValue<'static>>,
    name: &str,
) -> Option<Value> {
    let GffValue::ListOwned(vars) = fields.get("VarTable")? else {
        return None;
    };
    vars.iter()
        .find(|v| matches!(v.get("Name"), Some(GffValue::String(n)) if n.as_ref() == name))
        .filter(|v| matches!(v.get("Type"), Some(GffValue::Dword(1) | GffValue::Int(1))))
        .and_then(|v| v.get("Value"))
        .and_then(gff_integer)
        .map(Value::from)
}

pub fn write_journal(
    bytes: &[u8],
    player: usize,
    edits: &[(String, u32)],
) -> Result<Vec<u8>, String> {
    let locals: Vec<_> = edits
        .iter()
        .map(|(tag, state)| (journal_variable(tag), i64::from(*state)))
        .collect();
    if edits.iter().any(|(tag, _)| tag.is_empty()) {
        return Err("Empty quest tag".into());
    }
    write_pc_int_locals(bytes, player, &locals)
}

/// Set integer locals on the player's struct in `playerlist.ifo`, keeping the storage
/// type of existing entries and creating missing ones the way the engine does
/// (`Type` 1, `Value` INT).
pub fn write_pc_int_locals(
    bytes: &[u8],
    player: usize,
    edits: &[(String, i64)],
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
    for (name, state) in edits {
        let indices: Vec<_> = vars
            .iter()
            .enumerate()
            .filter(
                |(_, v)| matches!(v.get("Name"), Some(GffValue::String(n)) if n.as_ref() == name),
            )
            .map(|(i, _)| i)
            .collect();
        if indices.len() > 1 {
            return Err(format!("Duplicate player variable: {name}"));
        }
        if let Some(&index) = indices.first() {
            let var = &mut vars[index];
            if !matches!(var.get("Type"), Some(GffValue::Dword(1) | GffValue::Int(1))) {
                return Err(format!("Player variable {name} is not an integer local"));
            }
            let value = match var.get("Value") {
                Some(GffValue::Int(_)) => {
                    GffValue::Int(i32::try_from(*state).map_err(|_| "Value exceeds INT range")?)
                }
                Some(GffValue::Dword(_)) => {
                    GffValue::Dword(u32::try_from(*state).map_err(|_| "Value exceeds DWORD range")?)
                }
                _ => return Err(format!("Unsupported storage type for {name}")),
            };
            var.insert("Value".into(), value);
        } else {
            let value = i32::try_from(*state).map_err(|_| "New value exceeds INT range")?;
            vars.push(indexmap::indexmap! {
                "Name".into() => GffValue::String(name.clone().into()),
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

    fn game_data() -> GameData {
        GameData::new(Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::new(),
        )))
    }

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
    fn influence_scripts_write_the_capped_companion_global() {
        let (key, change) =
            decode_assignment(&action("ga_influence", json!([12, 5])), "m").unwrap();
        assert_eq!(key.module, None);
        assert_eq!(key.name, "00_nInfluenceammon_jerro");
        assert_eq!(key.value_type, "int");
        assert_eq!(change.apply(Some(&json!(97))), json!(100));
        assert_eq!(change.apply(None), json!(5));
        let (_, change) =
            decode_assignment(&action("ga_influence", json!(["1", "-30"])), "m").unwrap();
        assert_eq!(change.apply(Some(&json!(-80))), json!(-100));
        assert!(decode_assignment(&action("ga_influence", json!([1, 0])), "m").is_err());
        assert!(decode_assignment(&action("ga_influence", json!([14, 1])), "m").is_err());

        let (key, change) = decode_assignment(
            &action("ka_influence_magnitude", json!(["Safiya", -5])),
            "m",
        )
        .unwrap();
        assert_eq!(key.name, "00_nInfluencesafiya");
        assert_eq!(change.apply(Some(&json!(-50))), json!(-75));
        let (key, change) = decode_assignment(
            &action("ka_influence_magnitude", json!(["oneofmany", 3])),
            "m",
        )
        .unwrap();
        assert_eq!(key.name, "00_nInfluenceoneofmany");
        assert_eq!(change.apply(Some(&json!(10))), json!(25));
        let (key, _) =
            decode_assignment(&action("ka_influence_magnitude", json!(["o", 1])), "m").unwrap();
        assert_eq!(key.name, "00_nInfluenceokku");
        assert!(
            decode_assignment(&action("ka_influence_magnitude", json!(["gann", 6])), "m").is_err()
        );

        let (key, change) =
            decode_assignment(&action("ka_influence_set", json!(["dove", 150])), "m").unwrap();
        assert_eq!(key.name, "00_nInfluencedove");
        assert_eq!(change, Change::Set(json!(100)));
    }

    #[test]
    fn reward_scripts_add_gold_and_xp_to_the_player() {
        let (key, change) =
            decode_assignment(&action("ga_give_gold", json!([500, 0])), "m").unwrap();
        assert_eq!(key, character_key("Gold"));
        assert_eq!(change.apply(Some(&json!(100))), json!(600));
        assert_eq!(change.apply(None), json!(500));
        let (key, change) =
            decode_assignment(&action("ga_take_gold", json!(["500", 1])), "m").unwrap();
        assert_eq!(key.name, "Gold");
        assert_eq!(change.apply(Some(&json!(200))), json!(0));
        let (key, change) = decode_assignment(&action("ga_give_xp", json!(["250"])), "m").unwrap();
        assert_eq!(key, character_key("Experience"));
        assert_eq!(change.apply(Some(&json!(1000))), json!(1250));
        let (_, change) = decode_assignment(&action("ga_give_xp", json!([-5000])), "m").unwrap();
        assert_eq!(change.apply(Some(&json!(1000))), json!(0));
        assert_eq!(
            decode_assignment(&action("ga_give_gold", json!([0])), "m"),
            Err("no_effect")
        );
        assert!(decode_assignment(&action("ga_give_gold", json!([-5])), "m").is_err());
        assert!(decode_assignment(&action("ga_take_gold", json!([])), "m").is_err());

        let mut graph = graph();
        add_quest(&mut graph, "Paid", Vec::new());
        graph.quests[0].category.xp = 300;
        let tables = RewardTables::from_graph(&graph, None);
        let (key, change) =
            decode_action(&action("ga_give_quest_xp", json!(["paid"])), "m", &tables).unwrap();
        assert_eq!(key, character_key("Experience"));
        assert_eq!(change.apply(Some(&json!(10))), json!(310));
        assert_eq!(
            decode_action(&action("ga_give_quest_xp", json!(["other"])), "m", &tables),
            Err("unavailable_target")
        );
        assert_eq!(
            decode_action(
                &action("ga_xp_award_2da_entry", json!([3, 0])),
                "m",
                &tables
            ),
            Err("unavailable_target")
        );
        assert!(
            decode_action(
                &action("ga_global_int", json!(["x", "5"])),
                "m",
                &RewardTables::empty()
            )
            .is_ok()
        );
    }

    #[test]
    fn xp_awards_come_from_the_campaign_2da() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("k_xp_awards.2da"),
            "2DA V2.0\n\n\tXP\tDescStrRef\tDescriptionText\n0\t100\t****\t****\n1\t1000\t178881\ttext\n",
        )
        .unwrap();
        let mut graph = graph();
        graph.campaign.campaign_path = Some(dir.path().to_string_lossy().into_owned());
        let tables = RewardTables::from_graph(&graph, None);
        let (key, change) = decode_action(
            &action("ga_xp_award_2da_entry", json!([1, 0])),
            "m",
            &tables,
        )
        .unwrap();
        assert_eq!(key, character_key("Experience"));
        assert_eq!(change.apply(Some(&json!(5))), json!(1005));
        assert_eq!(
            decode_action(
                &action("ga_xp_award_2da_entry", json!([7, 0])),
                "m",
                &tables
            ),
            Err("unavailable_target")
        );
    }

    fn rich_fixture_handler(gold: u32, xp: i32) -> (tempfile::TempDir, SaveGameHandler) {
        fixture_handler_with(|player| {
            player.insert("Gold".into(), GffValue::Dword(gold));
            player.insert("Experience".into(), GffValue::Int(xp));
        })
    }

    /// The fixture player with `edit` applied, written as `playerlist.ifo` + `player.bic`.
    fn fixture_handler_with(
        edit: impl FnOnce(&mut indexmap::IndexMap<String, GffValue<'static>>),
    ) -> (tempfile::TempDir, SaveGameHandler) {
        use std::io::Write;
        let mut player = owned_players(&playerlist()).swap_remove(0);
        edit(&mut player);
        let playerlist = GffWriter::new("IFO ", "V3.2")
            .write(
                indexmap! { "Mod_PlayerList".into() => GffValue::ListOwned(vec![player.clone()]) },
            )
            .unwrap();
        player.swap_remove("VarTable");
        let bic = GffWriter::new("BIC ", "V3.2").write(player).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(dir.path().join("resgff.zip")).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("playerlist.ifo", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&playerlist).unwrap();
        zip.start_file("player.bic", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&bic).unwrap();
        zip.finish().unwrap();
        let handler = SaveGameHandler::new(dir.path(), false, false).unwrap();
        (dir, handler)
    }

    /// The rich fixture plus a two-member `roster.rst` (khelgar in the party and
    /// selectable, neeshka benched) and a `.ros` per member carrying the faction
    /// the roster state implies. The PC's party faction is set to 9.
    fn roster_fixture_handler() -> (tempfile::TempDir, SaveGameHandler) {
        use std::io::Write;
        let mut player = owned_players(&playerlist()).swap_remove(0);
        player.insert("FactionID".into(), GffValue::Word(9));
        let playerlist = GffWriter::new("IFO ", "V3.2")
            .write(indexmap! { "Mod_PlayerList".into() => GffValue::ListOwned(vec![player]) })
            .unwrap();
        let member = |name: &str,
                      available: i32,
                      selectable: i32|
         -> indexmap::IndexMap<String, GffValue<'static>> {
            indexmap! {
                "RosName".into() => GffValue::String(name.to_string().into()),
                "RosAvailable".into() => GffValue::Int(available),
                "RosSelectable".into() => GffValue::Int(selectable),
            }
        };
        let roster = GffWriter::new("RST ", "V3.2")
            .write(indexmap! {
                "RosMembers".into() => GffValue::ListOwned(vec![
                    member("khelgar", 0, 1),
                    member("neeshka", 1, 1),
                ]),
            })
            .unwrap();
        let ros = |faction: u16| {
            GffWriter::new("ROS ", "V3.2")
                .write_with_struct_id(
                    indexmap! {
                        "FactionID".into() => GffValue::Word(faction),
                        "LawfulChaotic".into() => GffValue::Byte(20),
                        "GoodEvil".into() => GffValue::Byte(60),
                        "FeatList".into() => GffValue::ListOwned(vec![
                            indexmap! { "Feat".into() => GffValue::Word(45) },
                        ]),
                    },
                    0xFFFF_FFFF,
                )
                .unwrap()
        };
        let dir = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(dir.path().join("resgff.zip")).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, bytes) in [
            ("playerlist.ifo", playerlist),
            ("roster.rst", roster),
            ("khelgar.ros", ros(9)),
            ("neeshka.ros", ros(roster::BENCHED_FACTION)),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(&bytes).unwrap();
        }
        zip.finish().unwrap();
        let handler = SaveGameHandler::new(dir.path(), false, false).unwrap();
        (dir, handler)
    }

    fn ros_faction(handler: &SaveGameHandler, ros_name: &str) -> u16 {
        let stored = handler.companion_stored_name(ros_name).unwrap();
        let root = owned_root(&handler.extract_file(&stored).unwrap()).unwrap();
        match root["FactionID"] {
            GffValue::Word(v) => v,
            ref other => panic!("FactionID is {other:?}"),
        }
    }

    /// The OC Khelgar monk line (`AdjustAlignment(oKhelgar, ALIGNMENT_LAWFUL, 50)`) and
    /// MotB's `FeatRemove(oOkku, FEAT_WEAPON_PROFICIENCY_MARTIAL)` reach the `.ros`;
    /// the write survives the file type, version and root struct id.
    #[test]
    fn companion_alignment_and_feats_are_written_to_the_ros() {
        let (_dir, mut handler) = roster_fixture_handler();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("AdjustAlignment", json!([2, 50, "Khelgar"])),
                action("ga_remove_feat", json!(["neeshka", 45])),
                action("ga_give_feat", json!(["neeshka", 1705, 0])),
                action("ga_give_feat", json!(["nobody", 1705, 0])),
                action("ga_give_feat", json!(["$OWNER", 1705, 0])),
            ],
        );
        let tables = RewardTables::from_graph(&graph, None);
        let mut edits = QuestEdits::default();
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        let reasons: Vec<&str> = preview.effects.iter().map(|e| e.reason.as_str()).collect();
        assert_eq!(reasons, vec!["unavailable_target", "object_target"]);
        let write = |value_type: &str, name: &str| {
            preview
                .writes
                .iter()
                .find(|w| w.key.value_type == value_type && w.key.name == name)
                .unwrap()
        };
        let lawful = write(COMPANION_CHARACTER_TYPE, "khelgar/LawfulChaotic");
        assert_eq!(
            (lawful.before.clone(), lawful.after.clone()),
            (Some(json!(20)), json!(70))
        );
        let removed = write(COMPANION_FEAT_TYPE, "neeshka/45");
        assert_eq!(
            (removed.before.clone(), removed.after.clone()),
            (Some(json!(1)), json!(0))
        );
        let added = write(COMPANION_FEAT_TYPE, "neeshka/1705");
        assert_eq!(
            (added.before.clone(), added.after.clone()),
            (Some(json!(0)), json!(1))
        );

        let all: Vec<usize> = (0..preview.writes.len()).collect();
        edits.stage(preview, &graph, &all, false).unwrap();
        handler.begin_batch();
        let written = edits.flush(&mut handler, 0, true, &game_data()).unwrap();
        handler.commit_batch().unwrap();
        assert!(written.player.is_empty());
        assert_eq!(written.companions.len(), 2);
        assert_eq!(written.companions[0].0, "khelgar");
        assert_eq!(written.companions[0].1[0].0, "LawfulChaotic");
        assert_eq!(gff_integer(&written.companions[0].1[0].1), Some(70));

        let ros = |name: &str| {
            let stored = handler.companion_stored_name(name).unwrap();
            let bytes = handler.extract_file(&stored).unwrap();
            let parser = GffParser::from_bytes(bytes.clone()).unwrap();
            assert_eq!(parser.file_type, "ROS ");
            assert_eq!(parser.get_struct_id(0).unwrap(), 0xFFFF_FFFF);
            owned_root(&bytes).unwrap()
        };
        let khelgar = ros("khelgar");
        assert_eq!(gff_integer(&khelgar["LawfulChaotic"]), Some(70));
        assert!(matches!(khelgar["LawfulChaotic"], GffValue::Byte(_)));
        assert_eq!(gff_integer(&khelgar["GoodEvil"]), Some(60));
        assert_eq!(gff_integer(&khelgar["FactionID"]), Some(9));
        let neeshka = ros("neeshka");
        assert!(!has_feat(&neeshka, 45) && has_feat(&neeshka, 1705));
        assert_eq!(gff_integer(&neeshka["LawfulChaotic"]), Some(20));
    }

    #[test]
    fn party_scripts_write_the_roster_and_mirror_the_ros_faction() {
        let (_dir, mut handler) = roster_fixture_handler();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_roster_party_add", json!(["Neeshka"])),
                action("ga_roster_party_remove", json!(["khelgar", ""])),
                action("ga_roster_selectable", json!(["khelgar", 0])),
                action("ga_roster_party_add", json!(["nobody"])),
            ],
        );
        let tables = RewardTables::from_graph(&graph, None);
        let mut edits = QuestEdits::default();
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.effects.len(), 1, "{:?}", preview.effects);
        assert_eq!(preview.effects[0].reason, "unavailable_target");
        let write = |value_type: &str, name: &str| {
            preview
                .writes
                .iter()
                .find(|w| w.key.value_type == value_type && w.key.name == name)
                .unwrap()
        };
        let joins = write(PARTY_TYPE, "neeshka");
        assert_eq!(
            (joins.before.clone(), joins.after.clone()),
            (Some(json!(0)), json!(1))
        );
        let leaves = write(PARTY_TYPE, "khelgar");
        assert_eq!(
            (leaves.before.clone(), leaves.after.clone()),
            (Some(json!(1)), json!(0))
        );
        let locked = write(SELECTABLE_TYPE, "khelgar");
        assert_eq!(
            (locked.before.clone(), locked.after.clone()),
            (Some(json!(1)), json!(0))
        );

        let all: Vec<usize> = (0..preview.writes.len()).collect();
        edits.stage(preview, &graph, &all, false).unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0, true, &game_data()).unwrap();
        handler.commit_batch().unwrap();

        let (_, bytes) = handler.extract_roster().unwrap().unwrap();
        let members = roster::parse_roster_members(bytes).unwrap();
        let member = |name: &str| members.iter().find(|m| m.ros_name == name).unwrap();
        assert!(member("khelgar").available && !member("khelgar").selectable);
        assert!(!member("neeshka").available && member("neeshka").selectable);
        assert_eq!(ros_faction(&handler, "khelgar"), roster::BENCHED_FACTION);
        assert_eq!(ros_faction(&handler, "neeshka"), 9);
    }

    fn player_gold_and_xp(handler: &SaveGameHandler) -> (i64, i64, i64, i64) {
        let list = player_struct(&handler.extract_player_data().unwrap(), 0).unwrap();
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        (
            gff_integer(&list["Gold"]).unwrap(),
            gff_integer(&list["Experience"]).unwrap(),
            gff_integer(&bic["Gold"]).unwrap(),
            gff_integer(&bic["Experience"]).unwrap(),
        )
    }

    #[test]
    fn rewards_are_previewed_from_the_save_and_added_on_top_of_the_staged_player() {
        let (_dir, mut handler) = rich_fixture_handler(100, 1000);
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_give_gold", json!([500, 0])),
                action("ga_give_xp", json!([250, 0])),
            ],
        );
        let mut edits = QuestEdits::default();
        let preview = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert!(preview.effects.is_empty(), "{:?}", preview.effects);
        let gold = preview
            .writes
            .iter()
            .find(|w| w.key.name == "Gold")
            .unwrap();
        assert_eq!(gold.key.value_type, CHARACTER_TYPE);
        assert_eq!(
            (gold.saved.clone(), gold.before.clone(), gold.after.clone()),
            (Some(json!(100)), Some(json!(100)), json!(600))
        );
        let xp = preview
            .writes
            .iter()
            .find(|w| w.key.name == "Experience")
            .unwrap();
        assert_eq!(xp.after, json!(1250));
        let all: Vec<usize> = (0..preview.writes.len()).collect();
        edits.stage(preview, &graph, &all, false).unwrap();
        edits
            .validate_saved(&handler, &NWN2Paths::new(), 0)
            .unwrap();

        // The character tabs saved their own gold edit first in the same batch.
        handler.begin_batch();
        let mut player = owned_players(&handler.extract_player_data().unwrap()).swap_remove(0);
        player.insert("Gold".into(), GffValue::Dword(150));
        let staged = GffWriter::new("IFO ", "V3.2")
            .write(indexmap! { "Mod_PlayerList".into() => GffValue::ListOwned(vec![player]) })
            .unwrap();
        handler.update_file("playerlist.ifo", &staged).unwrap();
        let written = edits
            .flush(&mut handler, 0, true, &game_data())
            .unwrap()
            .player;
        handler.commit_batch().unwrap();
        assert_eq!(
            format!("{written:?}"),
            format!(
                "{:?}",
                vec![
                    ("Gold".to_string(), GffValue::Dword(650)),
                    ("Experience".to_string(), GffValue::Int(1250)),
                ]
            )
        );
        assert_eq!(player_gold_and_xp(&handler), (650, 1250, 600, 1250));
        assert_eq!(read_live_journal(&handler, 0).unwrap()[0].state, 20);
    }

    #[test]
    fn taking_gold_never_goes_below_zero() {
        let (_dir, mut handler) = rich_fixture_handler(200, 0);
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![action("ga_take_gold", json!([500, 0]))],
        );
        let mut edits = QuestEdits::default();
        let preview = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.writes[0].after, json!(0));
        edits.stage(preview, &graph, &[0], false).unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0, true, &game_data()).unwrap();
        handler.commit_batch().unwrap();
        assert_eq!(player_gold_and_xp(&handler), (0, 0, 0, 0));
    }

    #[test]
    fn items_are_given_from_blueprints_into_both_player_files() {
        use crate::parsers::gff::{LocalizedString, LocalizedSubstring};
        let (_dir, mut handler) = rich_fixture_handler(0, 0);
        let campaign = tempfile::tempdir().unwrap();
        let blueprint = GffWriter::new("UTI ", "V3.2")
            .write(indexmap! {
                "BaseItem".into() => GffValue::Int(12),
                "Tag".into() => GffValue::String("qe_sword".into()),
                "Identified".into() => GffValue::Byte(0),
                "LocalizedName".into() => GffValue::LocString(LocalizedString {
                    string_ref: -1,
                    substrings: vec![LocalizedSubstring {
                        string: "Quest Sword".into(),
                        language: 0,
                        gender: 0,
                    }],
                }),
            })
            .unwrap();
        std::fs::write(campaign.path().join("qe_sword.uti"), blueprint).unwrap();
        let mut graph = graph();
        graph.campaign.campaign_path = Some(campaign.path().display().to_string());
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_give_item", json!(["QE_Sword", 2, 0])),
                action("ga_give_item", json!(["qe_sword", 0, 0])),
                action("ga_give_item", json!(["missing_item", 1, 0])),
            ],
        );
        let mut edits = QuestEdits::default();
        let tables = RewardTables::from_graph(&graph, None);
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.effects.len(), 1, "{:?}", preview.effects);
        assert_eq!(preview.effects[0].reason, "unknown_item");
        assert_eq!(preview.writes.len(), 1, "{:?}", preview.writes);
        let item = &preview.writes[0];
        assert_eq!(item.key.value_type, ITEM_TYPE);
        assert_eq!(item.key.name, "qe_sword");
        assert_eq!((item.before.clone(), item.after.clone()), (None, json!(3)));
        assert_eq!(item.label.as_deref(), Some("Quest Sword"));
        assert_eq!(item.label_strref, None);
        edits.stage(preview, &graph, &[0], false).unwrap();
        assert!(edits.pending[0].blueprints.contains_key("qe_sword"));
        edits
            .validate_saved(&handler, &NWN2Paths::new(), 0)
            .unwrap();

        handler.begin_batch();
        let written = edits
            .flush(&mut handler, 0, true, &game_data())
            .unwrap()
            .player;
        handler.commit_batch().unwrap();
        let swords = |list: &GffValue<'static>| match list {
            GffValue::ListOwned(items) => items
                .iter()
                .filter(|i| {
                    matches!(i.get("Tag"), Some(GffValue::String(t)) if t == "qe_sword")
                        && matches!(i.get("Identified"), Some(GffValue::Byte(1)))
                })
                .count(),
            _ => 0,
        };
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].0, "ItemList");
        assert_eq!(swords(&written[0].1), 3);
        let player = owned_players(&handler.extract_player_data().unwrap()).swap_remove(0);
        assert_eq!(swords(&player["ItemList"]), 3);
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        assert_eq!(swords(&bic["ItemList"]), 3);
        assert_eq!(read_live_journal(&handler, 0).unwrap()[0].state, 20);
    }

    #[test]
    fn alignment_acts_shift_the_axis_by_the_ginc_alignment_table() {
        for (act, axis, field, delta) in [
            (1, 0, "GoodEvil", 1),
            (-2, 0, "GoodEvil", -3),
            (3, 1, "LawfulChaotic", 10),
            (-4, 1, "LawfulChaotic", -70),
        ] {
            let (key, change) =
                decode_assignment(&action("ga_alignment", json!([act, axis])), "m").unwrap();
            assert_eq!(key, character_key(field));
            assert_eq!(
                change,
                Change::AddClamped {
                    delta,
                    min: 0,
                    max: 100
                }
            );
        }
        let (_, change) = decode_assignment(&action("ga_alignment", json!([4, 0])), "m").unwrap();
        assert_eq!(change.apply(Some(&json!(50))), json!(100));
    }

    #[test]
    fn feat_scripts_write_the_feat_on_the_player() {
        let (key, change) =
            decode_assignment(&action("ga_give_feat", json!(["", 1705, 0, 0])), "m").unwrap();
        assert_eq!(
            (key.module, key.name.as_str(), key.value_type.as_str()),
            (None, "1705", FEAT_TYPE)
        );
        assert_eq!(change.apply(Some(&json!(0))), json!(1));
        let (key, change) = decode_assignment(
            &action("ga_remove_feat", json!(["$PC_SPEAKER", "1705"])),
            "m",
        )
        .unwrap();
        assert_eq!(key.name, "1705");
        assert_eq!(change.apply(Some(&json!(1))), json!(0));
        let (key, change) =
            decode_assignment(&action("ga_give_feat", json!(["Khelgar", 1833, 1])), "m").unwrap();
        assert_eq!(
            (key.value_type.as_str(), key.name.as_str()),
            (COMPANION_FEAT_TYPE, "khelgar/1833")
        );
        assert_eq!(change.apply(None), json!(1));
        assert_eq!(
            decode_assignment(&action("ga_give_feat", json!(["$OWNER", 1833, 1])), "m"),
            Err("object_target")
        );
        assert_eq!(
            decode_assignment(&action("ga_give_feat", json!(["", -1])), "m"),
            Err("invalid_assignment")
        );
    }

    /// nwscript `AdjustAlignment` out of a wrapper script: axis constants, both axes,
    /// and the neutral shift that stops at 50.
    #[test]
    fn adjust_alignment_points_shift_the_axes() {
        let tables = RewardTables::empty();
        let mut current = |key: &VariableKey| match key.name.as_str() {
            "LawfulChaotic" => Some(10),
            "GoodEvil" => Some(80),
            _ => None,
        };
        let writes = decode_writes(
            &action("AdjustAlignment", json!([4, 5])),
            "m",
            &tables,
            &mut current,
        )
        .unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, character_key("GoodEvil"));
        assert_eq!(writes[0].1.apply(Some(&json!(98))), json!(100));
        let writes = decode_writes(
            &action("AdjustAlignment", json!([3, 12])),
            "m",
            &tables,
            &mut current,
        )
        .unwrap();
        assert_eq!(writes[0].0, character_key("LawfulChaotic"));
        assert_eq!(writes[0].1.apply(Some(&json!(10))), json!(0));
        let writes = decode_writes(
            &action("AdjustAlignment", json!([0, 3])),
            "m",
            &tables,
            &mut current,
        )
        .unwrap();
        assert_eq!(
            writes
                .iter()
                .map(|(k, _)| k.name.as_str())
                .collect::<Vec<_>>(),
            vec!["LawfulChaotic", "GoodEvil"]
        );
        let writes = decode_writes(
            &action("AdjustAlignment", json!([1, 15])),
            "m",
            &tables,
            &mut current,
        )
        .unwrap();
        assert_eq!(writes[0].1.apply(Some(&json!(10))), json!(25));
        assert_eq!(writes[1].1.apply(Some(&json!(80))), json!(65));
        let mut nearly_neutral = |key: &VariableKey| match key.name.as_str() {
            "LawfulChaotic" => Some(40),
            "GoodEvil" => Some(50),
            _ => None,
        };
        let writes = decode_writes(
            &action("AdjustAlignment", json!([1, 15])),
            "m",
            &tables,
            &mut nearly_neutral,
        )
        .unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].1.apply(Some(&json!(40))), json!(50));
        assert_eq!(
            decode_writes(
                &action("AdjustAlignment", json!([4, 0])),
                "m",
                &tables,
                &mut current
            ),
            Err("no_effect")
        );
    }

    #[test]
    fn staged_player_changes_are_applied_in_memory_and_taken_back_exactly() {
        let write = |value_type: &str, name: &str, before: Option<i64>, after: i64| VariableWrite {
            key: VariableKey {
                module: None,
                name: name.into(),
                value_type: value_type.into(),
            },
            saved: before.map(Value::from),
            before: before.map(Value::from),
            after: Value::from(after),
            action_index: 0,
            label: None,
            label_strref: None,
            derived_from: None,
            line: None,
        };
        let item = |tag: &str, stack: u16| -> indexmap::IndexMap<String, GffValue<'static>> {
            indexmap! {
                "BaseItem".into() => GffValue::Int(12),
                "Tag".into() => GffValue::String(tag.to_string().into()),
                "StackSize".into() => GffValue::Word(stack),
            }
        };
        let units = |character: &Character, tag: &str| -> i64 {
            backpack_items(character.gff())
                .filter(|i| item_tag_matches(i, tag))
                .map(item_stack_size)
                .sum()
        };
        let mut character = Character::from_gff(indexmap! {
            "Gold".into() => GffValue::Dword(100),
            "GoodEvil".into() => GffValue::Byte(80),
            "FeatList".into() => GffValue::ListOwned(vec![indexmap! {
                "Feat".into() => GffValue::Word(10),
            }]),
            "ItemList".into() => GffValue::ListOwned(vec![item("qe_head", 4), item("other", 1)]),
        });
        let blueprint = Arc::new(ItemBlueprint {
            fields: item("reward", 1),
            name: None,
            name_strref: None,
        });
        let edit = PendingQuestEdit {
            tag: "test".into(),
            target: 20,
            saved_state: None,
            source: None,
            writes: vec![
                write(CHARACTER_TYPE, "Gold", Some(100), 150),
                // Saved at 50, the tab moved it to 80: +45 clamps at 100.
                write(CHARACTER_TYPE, "GoodEvil", Some(50), 95),
                write(FEAT_TYPE, "20", Some(0), 1),
                write(FEAT_TYPE, "10", Some(1), 0),
                write(ITEM_TAKE_TYPE, "qe_head", Some(4), 1),
                write(ITEM_TYPE, "reward", None, 2),
                // Not the player's: left to `flush`.
                write("int", "SomeGlobal", Some(0), 1),
            ],
            blueprints: BTreeMap::from([("reward".to_string(), blueprint)]),
            applied: None,
        };
        let data = game_data();
        let changes = apply_to_player(&edit, &mut character, &data).unwrap();
        let int = |c: &Character, name: &str| c.gff().get(name).and_then(gff_integer);
        assert_eq!(int(&character, "Gold"), Some(150));
        assert_eq!(int(&character, "GoodEvil"), Some(100));
        assert!(has_feat(character.gff(), 20) && !has_feat(character.gff(), 10));
        assert_eq!(units(&character, "qe_head"), 1);
        assert_eq!(units(&character, "reward"), 2);
        assert!(character.is_modified());

        revert_on_player(&changes, &mut character).unwrap();
        assert_eq!(int(&character, "Gold"), Some(100));
        assert_eq!(int(&character, "GoodEvil"), Some(80));
        assert!(!has_feat(character.gff(), 20) && has_feat(character.gff(), 10));
        assert_eq!(units(&character, "qe_head"), 4);
        assert_eq!(units(&character, "reward"), 0);
        assert_eq!(units(&character, "other"), 1);
    }

    #[test]
    fn items_are_taken_by_tag_from_both_player_files() {
        use crate::parsers::gff::{LocalizedString, LocalizedSubstring};
        let item =
            |tag: &str, stack: Option<u16>| -> indexmap::IndexMap<String, GffValue<'static>> {
                let mut item = indexmap! {
                    "BaseItem".into() => GffValue::Int(12),
                    "Tag".into() => GffValue::String(tag.to_string().into()),
                    "LocalizedName".into() => GffValue::LocString(LocalizedString {
                        string_ref: 4242,
                        substrings: vec![LocalizedSubstring {
                            string: "Bandit Head".into(),
                            language: 0,
                            gender: 0,
                        }],
                    }),
                };
                if let Some(stack) = stack {
                    item.insert("StackSize".into(), GffValue::Word(stack));
                }
                item
            };
        let (_dir, mut handler) = fixture_handler_with(|player| {
            player.insert(
                "ItemList".into(),
                GffValue::ListOwned(vec![
                    item("QE_Head", None),
                    item("other", Some(3)),
                    item("qe_head", Some(4)),
                    item("qe_head", Some(2)),
                ]),
            );
        });
        let mut graph = graph();
        add_quest(
            &mut graph,
            "take",
            vec![
                action("ga_take_item", json!(["QE_Head", 3, 0])),
                action("ga_destroy_item", json!(["nothing_here", -1, 0])),
                action("ga_destroy_item", json!(["other", 0, 0])),
            ],
        );
        add_quest(
            &mut graph,
            "destroy_all",
            vec![action("ga_destroy_item", json!(["other", -1, 1]))],
        );
        let mut edits = QuestEdits::default();
        let tables = RewardTables::from_graph(&graph, None);
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "take",
                20,
                Some(0),
            )
            .unwrap();
        assert!(preview.effects.is_empty(), "{:?}", preview.effects);
        assert_eq!(preview.writes.len(), 1, "{:?}", preview.writes);
        let take = &preview.writes[0];
        assert_eq!(take.key.value_type, ITEM_TAKE_TYPE);
        assert_eq!(take.key.name, "qe_head");
        assert_eq!(
            (take.before.clone(), take.after.clone()),
            (Some(json!(7)), json!(4))
        );
        assert_eq!(take.label.as_deref(), Some("Bandit Head"));
        assert_eq!(take.label_strref, Some(4242));
        edits.stage(preview, &graph, &[0], false).unwrap();
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "destroy_all",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.writes.len(), 1, "{:?}", preview.writes);
        assert_eq!(
            (
                preview.writes[0].before.clone(),
                preview.writes[0].after.clone()
            ),
            (Some(json!(3)), json!(0))
        );
        edits.stage(preview, &graph, &[0], false).unwrap();
        edits
            .validate_saved(&handler, &NWN2Paths::new(), 0)
            .unwrap();

        handler.begin_batch();
        let written = edits
            .flush(&mut handler, 0, true, &game_data())
            .unwrap()
            .player;
        handler.commit_batch().unwrap();
        let stacks = |list: &GffValue<'static>| match list {
            GffValue::ListOwned(items) => items
                .iter()
                .map(|i| {
                    (
                        match i.get("Tag") {
                            Some(GffValue::String(t)) => t.to_string(),
                            _ => String::new(),
                        },
                        format!("{:?}", i.get("StackSize")),
                    )
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let expected = vec![
            (
                "qe_head".to_string(),
                format!("{:?}", Some(GffValue::Word(2))),
            ),
            (
                "qe_head".to_string(),
                format!("{:?}", Some(GffValue::Word(2))),
            ),
        ];
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].0, "ItemList");
        assert_eq!(stacks(&written[0].1), expected);
        let player = owned_players(&handler.extract_player_data().unwrap()).swap_remove(0);
        assert_eq!(stacks(&player["ItemList"]), expected);
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        assert_eq!(stacks(&bic["ItemList"]), expected);
        let journal = read_live_journal(&handler, 0).unwrap();
        assert!(
            ["take", "destroy_all"]
                .iter()
                .all(|tag| journal.iter().any(|e| e.tag == *tag && e.state == 20)),
            "{journal:?}"
        );
    }

    #[test]
    fn feats_are_granted_and_removed_in_both_player_files() {
        let (_dir, mut handler) = rich_fixture_handler(0, 0);
        let mut grant = graph();
        add_quest(
            &mut grant,
            "test",
            vec![
                action("ga_give_feat", json!(["", 1705, 0, 0])),
                action("ga_give_feat", json!(["", 1714, 0, 0])),
            ],
        );
        let tables = RewardTables::from_graph(&grant, None);
        let mut edits = QuestEdits::default();
        let preview = edits
            .preview(
                &grant,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert!(preview.effects.is_empty(), "{:?}", preview.effects);
        let feat = preview
            .writes
            .iter()
            .find(|w| w.key.value_type == FEAT_TYPE && w.key.name == "1705")
            .unwrap();
        assert_eq!(
            (feat.before.clone(), feat.after.clone()),
            (Some(json!(0)), json!(1))
        );
        let all: Vec<usize> = (0..preview.writes.len()).collect();
        edits.stage(preview, &grant, &all, false).unwrap();
        handler.begin_batch();
        let written = edits
            .flush(&mut handler, 0, true, &game_data())
            .unwrap()
            .player;
        handler.commit_batch().unwrap();
        assert!(written.iter().any(|(name, _)| name == "FeatList"));
        let player = player_struct(&handler.extract_player_data().unwrap(), 0).unwrap();
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        assert!(has_feat(&player, 1705) && has_feat(&player, 1714));
        assert!(has_feat(&bic, 1705) && has_feat(&bic, 1714));

        // Removing it again reads the saved feat as owned and drops it from both files.
        let mut removal = graph();
        add_quest(
            &mut removal,
            "test",
            vec![action("ga_remove_feat", json!(["", 1705, 0]))],
        );
        let mut edits = QuestEdits::default();
        let preview = edits
            .preview(
                &removal,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.writes[0].before, Some(json!(1)));
        assert_eq!(preview.writes[0].after, json!(0));
        edits.stage(preview, &removal, &[0], false).unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0, true, &game_data()).unwrap();
        handler.commit_batch().unwrap();
        let player = player_struct(&handler.extract_player_data().unwrap(), 0).unwrap();
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        assert!(!has_feat(&player, 1705) && has_feat(&player, 1714));
        assert!(!has_feat(&bic, 1705) && has_feat(&bic, 1714));
        assert_eq!(
            decode_assignment(&action("ga_alignment", json!([0, 0])), "m"),
            Err("no_effect")
        );

        let (_dir, mut handler) = rich_fixture_handler(0, 0);
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![action("ga_alignment", json!([3, 1]))],
        );
        let mut edits = QuestEdits::default();
        let tables = RewardTables::from_graph(&graph, None);
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        let write = &preview.writes[0];
        let saved = write.saved.as_ref().and_then(Value::as_i64).unwrap_or(0);
        assert_eq!(write.after, json!((saved + 10).min(100)));
        let bic_before = owned_root(&handler.extract_player_bic().unwrap().unwrap())
            .unwrap()
            .get("LawfulChaotic")
            .and_then(gff_integer)
            .unwrap_or(0);
        edits.stage(preview, &graph, &[0], false).unwrap();
        handler.begin_batch();
        let written = edits
            .flush(&mut handler, 0, true, &game_data())
            .unwrap()
            .player;
        handler.commit_batch().unwrap();
        assert_eq!(written[0].0, "LawfulChaotic");
        assert!(
            matches!(written[0].1, GffValue::Byte(_)),
            "{:?}",
            written[0].1
        );
        let player = owned_players(&handler.extract_player_data().unwrap()).swap_remove(0);
        assert_eq!(
            gff_integer(&player["LawfulChaotic"]).unwrap(),
            (saved + 10).min(100)
        );
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        assert_eq!(
            gff_integer(&bic["LawfulChaotic"]).unwrap(),
            (bic_before + 10).min(100)
        );
    }

    #[test]
    fn partial_quest_xp_is_capped_by_the_share_already_paid() {
        let (_dir, mut handler) = rich_fixture_handler(0, 1000);
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_give_partial_quest_xp", json!(["test", 60])),
                action("ga_give_partial_quest_xp", json!(["test", 60])),
            ],
        );
        graph.quests[0].category.xp = 500;
        let mut edits = QuestEdits::default();
        let tables = RewardTables::from_graph(&graph, None);
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        // Both calls write XP and the paid share, so neither variable is offered alone.
        assert!(preview.writes.is_empty(), "{:?}", preview.writes);
        assert_eq!(preview.effects.len(), 4);
        assert!(
            preview
                .effects
                .iter()
                .all(|e| e.reason == "multiple_assignments")
        );

        graph.quests[0].transitions[0].actions.pop();
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        let xp = preview
            .writes
            .iter()
            .find(|w| w.key.name == "Experience")
            .unwrap();
        assert_eq!(xp.after, json!(1300));
        let share = preview
            .writes
            .iter()
            .find(|w| w.key.value_type == PC_LOCAL_TYPE)
            .unwrap();
        assert_eq!(
            (
                share.key.name.as_str(),
                share.before.clone(),
                share.after.clone()
            ),
            ("test_XP_QPR", None, json!(60))
        );
        let all: Vec<usize> = (0..preview.writes.len()).collect();
        edits.stage(preview, &graph, &all, false).unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0, true, &game_data()).unwrap();
        handler.commit_batch().unwrap();
        assert_eq!(player_gold_and_xp(&handler).1, 1300);

        // A second 60% after saving is capped at the remaining 40%.
        let edits = QuestEdits::default();
        let preview = edits
            .preview(
                &graph,
                &tables,
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        let xp = preview
            .writes
            .iter()
            .find(|w| w.key.name == "Experience")
            .unwrap();
        assert_eq!(xp.after, json!(1500));
        let share = preview
            .writes
            .iter()
            .find(|w| w.key.value_type == PC_LOCAL_TYPE)
            .unwrap();
        assert_eq!(
            (share.before.clone(), share.after.clone()),
            (Some(json!(60)), json!(100))
        );
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
                spoken: None,
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
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
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
                ("visits", json!(3), Some(30)),
                ("NW_JOURNAL_ENTRYother", json!(5), Some(30))
            ]
        );
        assert!(
            !preview
                .effects
                .iter()
                .any(|e| e.reason == "follow_up_journal")
        );

        // With the gate satisfied the first reply wins; negation flips it back.
        std::fs::write(dir.path().join("globals.xml"), "<Globals><Integers><Integer><Name>friendly</Name><Value>1</Value></Integer></Integers></Globals>").unwrap();
        let preview = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(
            preview.followed.iter().map(|l| l.node).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
        graph.quests[0].transitions[0].follow_up[0].conditions[0].negate = true;
        let preview = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.followed[0].node, 11);

        // A condition the editor cannot evaluate stops the walk before that line.
        graph.quests[0].transitions[0].follow_up[0].conditions[0] =
            condition("10c_brelaina_ck", json!([20]), false, "");
        let preview = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
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
        let mut reader = VariableReader::new(&handler, &paths, 0);
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
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
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
        edits.flush(&mut handler, 0, false, &game_data()).unwrap();
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
                .preview(
                    &graph,
                    &RewardTables::from_graph(&graph, None),
                    &handler,
                    &NWN2Paths::new(),
                    0,
                    "test",
                    20,
                    Some(0)
                )
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
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        edits.stage(first, &graph, &[0], false).unwrap();
        let second = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "other",
                20,
                Some(0),
            )
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
                .preview(
                    &graph,
                    &RewardTables::from_graph(&graph, None),
                    &handler,
                    &NWN2Paths::new(),
                    0,
                    "test",
                    20,
                    Some(0)
                )
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
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                None,
            )
            .unwrap();
        let revision = preview.revision.clone();
        edits.stage(preview, &graph, &[], false).unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("resgff.zip")).unwrap(),
            original
        );
        let next = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                None,
            )
            .unwrap();
        assert_ne!(next.revision, revision);
        assert_eq!(next.before_state, Some(20));
        edits
            .validate_saved(&handler, &NWN2Paths::new(), 0)
            .unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0, false, &game_data()).unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("resgff.zip")).unwrap(),
            original
        );
        handler.abort_batch();
        assert_eq!(read_live_journal(&handler, 0).unwrap()[0].state, 10);
        handler.begin_batch();
        edits.flush(&mut handler, 0, false, &game_data()).unwrap();
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
                label: None,
                label_strref: None,
                derived_from: None,
                line: None,
            }],
            blueprints: BTreeMap::new(),
            applied: None,
        });
        edits.validate_saved(&handler, &paths, 0).unwrap();
        handler.begin_batch();
        edits.flush(&mut handler, 0, false, &game_data()).unwrap();
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

    #[test]
    fn next_steps_are_gated_on_the_save_after_the_edit() {
        let (dir, handler) = fixture_handler();
        std::fs::write(
            dir.path().join("globals.xml"),
            "<Globals><Integers></Integers></Globals>",
        )
        .unwrap();
        let mut graph = graph();
        cormick_like(&mut graph);
        let quest = graph.quests.last_mut().unwrap();
        let base = quest.transitions[0].clone();
        // Entry 30 needs the flag this edit's follow-up sets; entry 40 needs entry 30
        // in the journal; entry 50 is a debug line and stays hidden.
        quest.transitions.push(TransitionNode {
            new_state: 30,
            actions: vec![action("__connector_quest_tuple__", json!(["test", 30]))],
            gating_conditions: vec![condition(
                "gc_global_int",
                json!(["joined", "1"]),
                false,
                "",
            )],
            spoken: Some(SpokenLine {
                node: 7,
                is_player: false,
                speaker: "cormick".into(),
                text_strref: 123,
                conditions: Vec::new(),
            }),
            follow_up: Vec::new(),
            children: Vec::new(),
            ..base.clone()
        });
        quest.transitions.push(TransitionNode {
            new_state: 40,
            actions: vec![action("__connector_quest_tuple__", json!(["test", 40]))],
            gating_conditions: vec![condition(
                "gc_journal_entry",
                json!(["test", ">=30"]),
                false,
                "",
            )],
            follow_up: Vec::new(),
            children: Vec::new(),
            ..base.clone()
        });
        quest.transitions.push(TransitionNode {
            new_state: 50,
            debug: true,
            actions: vec![action("__connector_quest_tuple__", json!(["test", 50]))],
            follow_up: Vec::new(),
            children: Vec::new(),
            ..base.clone()
        });
        let edits = QuestEdits::default();
        let preview = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                Some(0),
            )
            .unwrap();
        let steps: Vec<(u32, &str)> = preview
            .next_steps
            .iter()
            .map(|s| (s.state, s.lines[0].gate.as_str()))
            .collect();
        assert_eq!(steps, vec![(30, "open"), (40, "closed")]);
        assert_eq!(
            preview.next_steps[0].lines[0]
                .spoken
                .as_ref()
                .map(|s| s.text_strref),
            Some(123)
        );
        // Without the route nothing sets `joined`, so entry 30 stays closed.
        let journal_only = edits
            .preview(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
                None,
            )
            .unwrap();
        assert_eq!(journal_only.next_steps[0].lines[0].gate, "closed");
    }

    #[test]
    fn route_summaries_carry_effects_in_the_signature() {
        let (_dir, handler) = fixture_handler();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("__connector_quest_tuple__", json!(["test", 20])),
                action("ga_destroy", json!(["door_a", 0])),
            ],
        );
        let edits = QuestEdits::default();
        let routes = edits
            .summarize_routes(
                &graph,
                &RewardTables::from_graph(&graph, None),
                &handler,
                &NWN2Paths::new(),
                0,
                "test",
                20,
            )
            .unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].effects.len(), 1);
        assert_eq!(routes[0].effects[0].script, "ga_destroy");
        assert!(routes[0].signature.contains("ga_destroy([\"door_a\",0])"));
    }
}
