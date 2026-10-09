use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::aggregator::derived_call;
use super::area_objects::{self, AreaField, ModuleObjects};
use super::item_blueprints::{ItemBlueprint, ItemBlueprints};
use super::journal_reader::player_journals;
use super::{FollowNode, SaveGraph, SpokenLine, TransitionNode, read_live_journal};
use crate::character::{Character, FeatId, FeatSource};
use crate::config::NWN2Paths;
use crate::loaders::GameData;
use crate::parsers::erf::ErfParser;
use crate::parsers::gff::{GffParser, GffValue, GffWriter, gff_integer, rewrite_gff};
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VariableWrite {
    pub key: VariableKey,
    /// The value on disk, checked again before saving.
    pub saved: Option<Value>,
    /// The live value (loaded characters, pending edits) the preview started from.
    pub before: Option<Value>,
    /// `change` applied to `before`; staging and saving apply `change` again to the
    /// value as it is then.
    pub after: Value,
    pub change: Change,
    pub action_index: usize,
    /// Display name of an item write's blueprint or taken item, when embedded in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// TLK strref of that name when it is not embedded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_strref: Option<i32>,
    /// The wrapper script call this write was decoded from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<String>,
    /// Lead-in or follow-up line (node id in `QuestPreview::lead_in` / `followed`) this
    /// write comes from; `None` for the chosen line itself.
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
    /// Lines the game plays before the chosen one on every way to it; writes and effects
    /// from them carry their node id in `line` like the followed ones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lead_in: Vec<FollowedLine>,
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

type GffStruct = indexmap::IndexMap<String, GffValue<'static>>;

/// One change a staged quest edit made to the loaded player.
#[derive(Debug, Clone)]
pub enum MemoryChange {
    /// Integer field moved by `delta` (after clamping).
    Field {
        name: String,
        delta: i64,
    },
    FeatAdded {
        id: i32,
    },
    FeatRemoved(RemovedFeat),
    ItemsGiven(GivenItems),
    /// Backpack entries the take touched, in the order it touched them.
    ItemsTaken {
        items: Vec<TakenItem>,
    },
}

/// A removed feat's entries and where they sat, for putting them back in place.
#[derive(Debug, Clone)]
pub struct RemovedFeat {
    id: i32,
    /// `(index in FeatList, entry)`.
    entries: Vec<(usize, GffStruct)>,
    /// `(LvlStatList index, index in its FeatList, entry)`.
    levels: Vec<(usize, usize, GffStruct)>,
}

/// What a hand-out did to the `ItemList`: entries it appended and stacks it grew.
#[derive(Debug, Clone)]
pub struct GivenItems {
    tag: String,
    units: i64,
    /// `ItemList` indices of the new entries.
    added: Vec<usize>,
    /// `(ItemList index, StackSize before)` of stacks it filled up.
    grown: Vec<(usize, i64)>,
}

/// A backpack entry a take shrank or removed.
#[derive(Debug, Clone)]
pub struct TakenItem {
    /// `ItemList` index at the time of the take.
    index: usize,
    /// The entry before the take.
    item: GffStruct,
    /// Units taken from it.
    units: i64,
    removed: bool,
}

/// Unsaved state the preview reads instead of the save on disk: the loaded player when
/// it is the quest target, companions loaded in this session and the staged party.
#[derive(Default)]
pub struct LiveCharacters<'a> {
    pub player: Option<std::borrow::Cow<'a, GffStruct>>,
    /// By lowercase roster name.
    pub companions: BTreeMap<String, &'a GffStruct>,
    /// Staged party membership by lowercase roster name.
    pub party: BTreeMap<String, bool>,
    /// Changes whenever the characters above do; part of the preview revision.
    pub revision: u64,
}

/// A staged variable value as a number; `flush` writes the same text.
fn variable_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
}

/// A variable store's `int`/`float`/`string` maps by `VariableKey::value_type`.
trait VariableStore {
    fn get(&self, kind: &str, name: &str) -> Option<Value>;
    fn set(&mut self, kind: &str, name: String, value: &Value);
}

macro_rules! variable_store {
    ($store:ty) => {
        impl VariableStore for $store {
            fn get(&self, kind: &str, name: &str) -> Option<Value> {
                match kind {
                    "int" => self.integers.get(name).map(|v| Value::from(*v)),
                    "float" => self.floats.get(name).map(|v| Value::from(*v)),
                    _ => self.strings.get(name).map(|v| Value::from(v.as_str())),
                }
            }

            fn set(&mut self, kind: &str, name: String, value: &Value) {
                match kind {
                    "int" => {
                        self.integers.insert(name, variable_int(value));
                    }
                    "float" => {
                        self.floats.insert(name, variable_float(value));
                    }
                    _ => {
                        self.strings.insert(name, variable_text(value));
                    }
                }
            }
        }
    };
}

variable_store!(crate::parsers::xml::XmlData);
variable_store!(content::ModuleVariables);

fn variable_int(value: &Value) -> i32 {
    variable_number(value)
        .and_then(|v| i32::try_from(v as i64).ok())
        .unwrap_or_default()
}

fn variable_float(value: &Value) -> f32 {
    variable_number(value).unwrap_or_default() as f32
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
    /// Changes whenever the pending edits do.
    pub fn generation(&self) -> u64 {
        self.generation
    }

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

    /// The story variables (`int`/`float`/`string`) the pending edits write in `module`
    /// (globals for `None`), each with the value it takes once they are applied to its
    /// current value in `store`, in staging order.
    fn pending_variables(
        &self,
        module: Option<&str>,
        store: &impl VariableStore,
    ) -> Vec<(VariableKey, Value)> {
        let mut keys: Vec<&VariableKey> = Vec::new();
        for write in self.pending.iter().flat_map(|p| &p.writes) {
            let in_scope = match (&write.key.module, module) {
                (None, None) => true,
                (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                _ => false,
            };
            if in_scope
                && matches!(write.key.value_type.as_str(), "int" | "float" | "string")
                && !keys.contains(&&write.key)
            {
                keys.push(&write.key);
            }
        }
        keys.into_iter()
            .filter_map(|key| {
                let base = store.get(&key.value_type, &key.name);
                Some((key.clone(), self.pending_value(key, base, None)?))
            })
            .collect()
    }

    /// `store` (globals when `module` is `None`) as it will be once the pending edits
    /// are saved.
    fn overlay_variables(&self, module: Option<&str>, store: &mut impl VariableStore) {
        for (key, value) in self.pending_variables(module, store) {
            store.set(&key.value_type, key.name, &value);
        }
    }

    /// `globals.xml` values as they will be once the pending edits are saved.
    pub fn overlay_globals(&self, data: &mut crate::parsers::xml::XmlData) {
        self.overlay_variables(None, data);
    }

    /// A module's variables as they will be once the pending edits are saved.
    pub fn overlay_module(&self, module: &str, vars: &mut content::ModuleVariables) {
        self.overlay_variables(Some(module), vars);
    }

    /// Put back an edit taken out by undo.
    pub fn restore(&mut self, edit: PendingQuestEdit) {
        self.pending.retain(|p| p.tag != edit.tag);
        self.pending.push(edit);
        self.generation = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    }

    /// Put back edits a failed stage or discard took out.
    pub fn restore_all(&mut self, edits: Vec<PendingQuestEdit>) {
        for edit in edits {
            self.restore(edit);
        }
    }

    /// The pending writes not yet in the live values, in staging order, without the
    /// edit of `exclude` (the quest being previewed, which its new edit replaces). Player
    /// writes staging applied to the loaded player are already part of it.
    fn unapplied_writes<'s>(
        &'s self,
        exclude: Option<&'s str>,
    ) -> impl Iterator<Item = &'s VariableWrite> + 's {
        self.pending
            .iter()
            .filter(move |p| Some(p.tag.as_str()) != exclude)
            .flat_map(|p| {
                p.writes
                    .iter()
                    .filter(|w| p.applied.is_none() || !applies_to_player(&w.key.value_type))
            })
    }

    /// The value `key` takes once the pending edits (but `exclude`'s) are applied to its
    /// live value `base`, in staging order.
    pub fn pending_value(
        &self,
        key: &VariableKey,
        base: Option<Value>,
        exclude: Option<&str>,
    ) -> Option<Value> {
        self.unapplied_writes(exclude)
            .filter(|w| w.key == *key)
            .fold(base, |value, w| Some(w.change.apply(value.as_ref())))
    }

    /// Journal states once the pending edits (but `exclude`'s) are applied: each edit's
    /// own entry is set, entries its lines add to other quests only move up.
    fn pending_journal(
        &self,
        saved: &BTreeMap<String, u32>,
        exclude: &str,
    ) -> BTreeMap<String, u32> {
        let mut journal = saved.clone();
        for edit in self.pending.iter().filter(|p| p.tag != exclude) {
            journal.insert(edit.tag.clone(), edit.target);
            for write in &edit.writes {
                if write.key.value_type != PC_LOCAL_TYPE {
                    continue;
                }
                let Some(tag) = write.key.name.strip_prefix(JOURNAL_PREFIX) else {
                    continue;
                };
                let before = journal.get(tag).map(|s| Value::from(*s));
                if let Some(state) = write
                    .change
                    .apply(before.as_ref())
                    .as_u64()
                    .and_then(|s| u32::try_from(s).ok())
                {
                    journal.insert(tag.to_string(), state);
                }
            }
        }
        journal
    }

    #[allow(clippy::too_many_arguments)]
    pub fn preview(
        &self,
        graph: &SaveGraph,
        tables: &RewardTables,
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        player: usize,
        characters: LiveCharacters<'_>,
        tag: &str,
        target: u32,
        route: Option<usize>,
    ) -> Result<QuestPreview, String> {
        let live = LiveSave::read(handler, player, tag, target)?;
        let mut reader = VariableReader::new(handler, paths, player, characters);
        self.preview_with(graph, tables, &live, &mut reader, tag, target, route)
    }

    /// `preview` against a save read once and a reader shared between previews.
    #[allow(clippy::too_many_arguments)]
    fn preview_with(
        &self,
        graph: &SaveGraph,
        tables: &RewardTables,
        live: &LiveSave,
        reader: &mut VariableReader<'_>,
        tag: &str,
        target: u32,
        route: Option<usize>,
    ) -> Result<QuestPreview, String> {
        let quest = graph
            .quests
            .iter()
            .find(|q| q.tag == tag)
            .ok_or("Quest is no longer available")?;
        let saved_state = live.journal.get(tag).copied();
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
            lead_in: Vec::new(),
            followed: Vec::new(),
            follow_stop: None,
            next_steps: Vec::new(),
            blueprints: BTreeMap::new(),
        };
        let route_line = route
            .map(|index| {
                quest
                    .transitions
                    .get(index)
                    .filter(|tr| tr.new_state == target)
                    .ok_or("The selected route does not reach this state")
            })
            .transpose()?;
        let mut journal = self.pending_journal(&live.journal, tag);
        // Before this edit's own target counts as reached.
        let reached = |quest: &str, state: u32| {
            [journal.get(quest), live.reached.get(quest)]
                .into_iter()
                .flatten()
                .any(|saved| *saved >= state)
        };
        let lead_in = route_line.map_or(&[][..], |tr| {
            &tr.lead_in[unplayed_lead_in(&tr.lead_in, reached)..]
        });
        journal.insert(tag.to_string(), target);
        reader.journal = Some(journal);
        // The other quests' pending writes, as this quest's lines will find them.
        reader.overlay.clear();
        let keys: BTreeSet<&VariableKey> = self
            .unapplied_writes(Some(tag))
            .map(|w| &w.key)
            .filter(|key| given_resref(key).is_none())
            .collect();
        for key in keys {
            if let Ok(base) = reader.read_base(key)
                && let Some(value) = self.pending_value(key, base, Some(tag))
            {
                reader.overlay.insert(key.clone(), value);
            }
        }
        if let Some(tr) = route_line {
            let mut played = PlayedLines {
                tables,
                module: &tr.module,
                tag,
                target,
                candidates: Vec::new(),
                actions: Vec::new(),
                effects: Vec::new(),
                blueprints: BTreeMap::new(),
            };
            // What the game plays: the lines leading to the chosen one, the chosen line,
            // then every line it plays after it, each seeing the writes of the ones before.
            for node in lead_in {
                played.line(LineKind::LeadIn(node.node), &node.actions, reader);
            }
            played.line(LineKind::Chosen, &tr.actions, reader);
            let (followed, stop) = follow_conversation(tr, reader, |node, reader| {
                played.line(LineKind::FollowUp(node.node), &node.actions, reader);
            })?;
            result.lead_in = lead_in.iter().map(followed_line).collect();
            result.followed = followed.into_iter().map(followed_line).collect();
            result.follow_stop = Some(stop);
            let PlayedLines {
                candidates,
                actions: all_actions,
                effects,
                blueprints,
                ..
            } = played;
            result.effects = effects;
            result.blueprints = blueprints;
            // Items add up the way repeated CreateItemOnObject calls do.
            let mut merged: Vec<VariableWrite> = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                if given_resref(&candidate.key).is_some()
                    && let Some(earlier) = merged.iter_mut().find(|w| w.key == candidate.key)
                {
                    let total =
                        earlier.after.as_i64().unwrap_or(0) + candidate.after.as_i64().unwrap_or(0);
                    earlier.after = Value::from(total);
                    earlier.change = Change::Set(Value::from(total));
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
            // applied, its journal entry being this edit's target) leaves nothing to run by hand.
            let unapplied: BTreeSet<String> = result
                .effects
                .iter()
                .filter(|e| {
                    e.reason != "journal_action"
                        || journal_transition(&e.action).map(|(_, state)| state) != Some(target)
                })
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
        // What comes next: the other entries' lines, evaluated against the save as it
        // will be once this edit and the pending ones are written.
        result.next_steps = next_steps(quest, graph, target, reader)?;
        let mut hash = Sha256::new();
        hash.update(self.generation.to_le_bytes());
        hash.update(&live.playerlist);
        hash.update(reader.live.revision.to_le_bytes());
        for (name, in_party) in &reader.live.party {
            hash.update(name.as_bytes());
            hash.update([u8::from(*in_party)]);
        }
        hash.update(serde_json::to_vec(&result).map_err(|e| e.to_string())?);
        result.revision = hex::encode(hash.finalize());
        Ok(result)
    }

    /// Preview every line that reaches `target` and reduce each to a `RouteSummary`.
    #[allow(clippy::too_many_arguments)]
    pub fn summarize_routes(
        &self,
        graph: &SaveGraph,
        tables: &RewardTables,
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        player: usize,
        characters: LiveCharacters<'_>,
        tag: &str,
        target: u32,
    ) -> Result<Vec<RouteSummary>, String> {
        let quest = graph
            .quests
            .iter()
            .find(|q| q.tag == tag)
            .ok_or("Quest is no longer available")?;
        let live = LiveSave::read(handler, player, tag, target)?;
        let mut reader = VariableReader::new(handler, paths, player, characters);
        let mut out = Vec::new();
        for (route, tr) in quest.transitions.iter().enumerate() {
            if tr.new_state != target {
                continue;
            }
            let preview =
                self.preview_with(graph, tables, &live, &mut reader, tag, target, Some(route))?;
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
        // Changes relative to the current value apply in staging order; only two quests
        // setting one variable to different values exclude each other.
        let conflicts: BTreeSet<String> = self
            .pending
            .iter()
            .filter(|p| {
                p.tag != preview.tag
                    && p.writes.iter().any(|old| {
                        writes.iter().any(|new| {
                            old.key == new.key
                                && !accumulates(&old.key.value_type)
                                && matches!(
                                    (&old.change, &new.change),
                                    (Change::Set(a), Change::Set(b)) if a != b
                                )
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
                    .any(|w| given_resref(&w.key) == Some(resref.as_str()))
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
        let mut reader = VariableReader::new(handler, paths, player, LiveCharacters::default());
        for edit in &self.pending {
            if journal.iter().find(|j| j.tag == edit.tag).map(|j| j.state) != edit.saved_state {
                return Err(format!(
                    "Saved quest {} changed. Discard and review its pending edit again.",
                    edit.tag
                ));
            }
            for write in &edit.writes {
                if reader.read_saved(&write.key)? != write.saved {
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
        paths: &NWN2Paths,
        player: usize,
        primary: bool,
        game_data: &GameData,
    ) -> Result<FlushedWrites, String> {
        if self.pending.is_empty() {
            return Ok(FlushedWrites::default());
        }
        let bytes = handler.extract_player_data().map_err(|e| e.to_string())?;
        let saved_player = player_struct(&bytes, player)?;
        // PC locals in staging order: each edit sets its own journal entry, the locals its
        // lines write (other quests' entries, the paid quest XP share) change the value
        // as it is by then.
        let mut locals: Vec<(String, Value)> = Vec::new();
        let mut other_locals = false;
        let mut set_local = |name: String, change: &Change| {
            if let Some((_, value)) = locals.iter_mut().find(|(n, _)| *n == name) {
                *value = change.apply(Some(value));
            } else {
                let base = pc_int_local(&saved_player, &name);
                locals.push((name, change.apply(base.as_ref())));
            }
        };
        let mut rewards: Vec<(String, Change)> = Vec::new();
        let mut items: Vec<ItemOp> = Vec::new();
        let mut feats: Vec<(i32, bool)> = Vec::new();
        let mut party: Vec<(String, bool)> = Vec::new();
        let mut selectable: Vec<(String, bool)> = Vec::new();
        let mut companions: BTreeMap<String, CompanionChanges> = BTreeMap::new();
        for edit in &self.pending {
            set_local(
                journal_variable(&edit.tag),
                &Change::Set(Value::from(edit.target)),
            );
            for write in &edit.writes {
                if edit.applied.is_some() && applies_to_player(&write.key.value_type) {
                    continue;
                }
                if matches!(
                    write.key.value_type.as_str(),
                    COMPANION_ITEM_TYPE
                        | COMPANION_ITEM_TAKE_TYPE
                        | COMPANION_CHARACTER_TYPE
                        | COMPANION_FEAT_TYPE
                ) {
                    let (ros_name, field) =
                        split_companion_key(&write.key.name).ok_or("Invalid companion key")?;
                    let changes = companions.entry(ros_name.to_string()).or_default();
                    match write.key.value_type.as_str() {
                        COMPANION_ITEM_TYPE => {
                            let blueprint = edit
                                .blueprints
                                .get(field)
                                .ok_or_else(|| format!("Item blueprint {field} missing"))?;
                            changes.items.push(ItemOp::Give(
                                blueprint.clone(),
                                write.after.as_i64().unwrap_or(0),
                            ));
                        }
                        COMPANION_ITEM_TAKE_TYPE => {
                            changes
                                .items
                                .push(ItemOp::Take(field.to_string(), write.change.clone()));
                        }
                        COMPANION_CHARACTER_TYPE => {
                            changes
                                .rewards
                                .push((field.to_string(), write.change.clone()));
                        }
                        _ => {
                            let feat = field.parse::<i32>().map_err(|_| "Invalid feat id")?;
                            changes.feats.retain(|(id, _)| *id != feat);
                            changes
                                .feats
                                .push((feat, write.after.as_i64().unwrap_or(0) != 0));
                        }
                    }
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
                    set_local(write.key.name.clone(), &write.change);
                    other_locals = true;
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
                    rewards.push((write.key.name.clone(), write.change.clone()));
                } else if write.key.value_type == ITEM_TYPE {
                    let blueprint = edit
                        .blueprints
                        .get(&write.key.name)
                        .ok_or_else(|| format!("Item blueprint {} missing", write.key.name))?;
                    let quantity = write.after.as_i64().unwrap_or(0);
                    if quantity > 0 {
                        items.push(ItemOp::Give(blueprint.clone(), quantity));
                    }
                } else if write.key.value_type == ITEM_TAKE_TYPE {
                    items.push(ItemOp::Take(write.key.name.clone(), write.change.clone()));
                }
            }
        }
        let locals = locals
            .into_iter()
            .map(|(name, value)| {
                value
                    .as_i64()
                    .map(|v| (name, v))
                    .ok_or("PC local is not an integer")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let bytes = write_pc_int_locals(&bytes, player, &locals)?;
        let (bytes, mut written) = add_player_rewards(&bytes, player, &rewards)?;
        // Locals besides the journal (which the player save keeps from disk) as written,
        // or the loaded player's next save puts the old ones back.
        if other_locals {
            let var_table = player_struct(&bytes, player)?
                .swap_remove("VarTable")
                .ok_or("VarTable missing after writing locals")?;
            written.push(("VarTable".into(), var_table));
        }
        let (bytes, item_list) = apply_player_items(&bytes, player, &items, game_data)?;
        written.extend(item_list);
        let (bytes, feat_list) = set_player_feats(&bytes, player, &feats)?;
        written.extend(feat_list);
        handler
            .update_file("playerlist.ifo", &bytes)
            .map_err(|e| e.to_string())?;
        if primary
            && (!rewards.is_empty() || !items.is_empty() || !feats.is_empty())
            && let Some(bic) = handler.extract_player_bic().map_err(|e| e.to_string())?
        {
            let bic = add_bic_rewards(&bic, &rewards)?;
            let bic = apply_bic_items(&bic, &items, game_data)?;
            let bic = set_bic_feats(&bic, &feats)?;
            handler
                .update_file("player.bic", &bic)
                .map_err(|e| e.to_string())?;
        }
        let mut warnings = Vec::new();
        if !party.is_empty() {
            warnings.extend(roster::apply_party_membership(handler, &party)?);
        }
        if !selectable.is_empty() {
            roster::apply_selectable(handler, &selectable)?;
        }
        let mut written_companions = Vec::new();
        for (ros_name, changes) in &companions {
            let stored = handler
                .companion_stored_name(ros_name)
                .map_err(|e| e.to_string())?;
            let bytes = handler.extract_file(&stored).map_err(|e| e.to_string())?;
            let (bytes, fields) = update_companion(bytes, changes, game_data)?;
            handler
                .update_file(&stored, &bytes)
                .map_err(|e| e.to_string())?;
            written_companions.push((ros_name.clone(), fields));
        }
        let mut area_edits: BTreeMap<String, Vec<(AreaField, String, bool)>> = BTreeMap::new();
        let mut modules: BTreeSet<String> = BTreeSet::new();
        let mut globals = false;
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
            } else if !writes_save_files(&write.key.value_type) {
                match &write.key.module {
                    Some(module) => {
                        modules.insert(module.clone());
                    }
                    None => globals = true,
                }
            }
        }
        if globals {
            let xml = handler.extract_globals_xml().map_err(|e| e.to_string())?;
            let mut parser = crate::parsers::xml::RustXmlParser::from_string(&xml)?;
            self.overlay_globals(&mut parser.data);
            handler
                .update_file("globals.xml", parser.to_xml_string()?.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        let variable_modules = modules.clone();
        modules.extend(area_edits.keys().cloned());
        for module in modules {
            let edits = area_edits.remove(&module).unwrap_or_default();
            let updates: Vec<(String, String, String)> = if variable_modules.contains(&module) {
                let (_, vars) = CampaignManager::get_module_info_by_id(handler, paths, &module)?;
                self.pending_variables(Some(&module), &vars)
                    .into_iter()
                    .map(|(key, value)| (key.name, variable_text(&value), key.value_type))
                    .collect()
            } else {
                Vec::new()
            };
            let (path, bytes) = content::prepare_module_variable_updates(
                handler,
                &updates,
                Some(&module),
                |save| {
                    if edits.is_empty() {
                        return Ok(());
                    }
                    let objects = area_objects::installed_module_objects(paths, &module)?;
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
            companions: written_companions,
            warnings,
        })
    }
}

/// The player's `playerlist.ifo` and journal, read once per command.
struct LiveSave {
    playerlist: Vec<u8>,
    /// Journal states by quest tag.
    journal: BTreeMap<String, u32>,
    /// The highest state any character in the save reached, by quest tag: in a
    /// multiplayer save the story is shared, the journal is per character.
    reached: BTreeMap<String, u32>,
}

impl LiveSave {
    /// Also checks that `tag` can be set to `target` (storage and range, absent
    /// entries included) before a preview offers Apply.
    fn read(
        handler: &SaveGameHandler,
        player: usize,
        tag: &str,
        target: u32,
    ) -> Result<Self, String> {
        let playerlist = handler.extract_player_data().map_err(|e| e.to_string())?;
        write_journal(&playerlist, player, &[(tag.to_string(), target)])?;
        let journals = player_journals(&playerlist)?;
        let journal = journals
            .get(player)
            .ok_or_else(|| format!("player_index {player} out of range ({})", journals.len()))?
            .iter()
            .map(|q| (q.tag.clone(), q.state))
            .collect();
        let mut reached: BTreeMap<String, u32> = BTreeMap::new();
        for entry in journals.into_iter().flatten() {
            let state = reached.entry(entry.tag).or_default();
            *state = (*state).max(entry.state);
        }
        Ok(Self {
            playerlist,
            journal,
            reached,
        })
    }
}

/// Where a played line sits relative to the line the edit picks.
#[derive(Clone, Copy)]
enum LineKind {
    /// On every way to the chosen line, before it.
    LeadIn(i32),
    Chosen,
    /// Played after it without a player decision.
    FollowUp(i32),
}

impl LineKind {
    fn node(self) -> Option<i32> {
        match self {
            LineKind::LeadIn(node) | LineKind::FollowUp(node) => Some(node),
            LineKind::Chosen => None,
        }
    }
}

/// Index of the first lead-in line the save has not played yet. A line whose journal
/// entry the save already `reached` was played before (journal entries only move up),
/// and so was every line before it: replaying those would roll later progress back.
fn unplayed_lead_in(lead_in: &[FollowNode], reached: impl Fn(&str, u32) -> bool) -> usize {
    lead_in
        .iter()
        .rposition(|node| {
            node.actions
                .iter()
                .filter_map(journal_transition)
                .any(|(quest, state)| reached(&quest, state))
        })
        .map_or(0, |played| played + 1)
}

fn followed_line(node: &FollowNode) -> FollowedLine {
    FollowedLine {
        node: node.node,
        is_player: node.is_player,
        has_text: node.has_text,
        speaker: node.speaker.clone(),
        text_strref: node.text_strref,
    }
}

/// The writes and effects of the lines a route plays, collected line by line.
struct PlayedLines<'p> {
    tables: &'p RewardTables<'p>,
    module: &'p str,
    tag: &'p str,
    target: u32,
    candidates: Vec<VariableWrite>,
    /// Every action seen, indexed by `VariableWrite::action_index`.
    actions: Vec<ConvoFunctor>,
    effects: Vec<ActionEffect>,
    blueprints: BTreeMap<String, Arc<ItemBlueprint>>,
}

impl PlayedLines<'_> {
    fn effect(&mut self, action: &ConvoFunctor, reason: &str, line: Option<i32>) {
        self.effects.push(ActionEffect {
            action: action.clone(),
            reason: reason.into(),
            line,
        });
    }

    /// Decode one line's actions into candidate writes and put them in the reader's
    /// overlay, so the lines played after it see them.
    fn line(&mut self, kind: LineKind, actions: &[ConvoFunctor], reader: &mut VariableReader<'_>) {
        let line = kind.node();
        for action in actions {
            let action_index = self.actions.len();
            self.actions.push(action.clone());
            if let Some((other_tag, state)) = journal_transition(action) {
                if other_tag == self.tag {
                    let reason = match kind {
                        LineKind::Chosen => Some("journal_action"),
                        // The edit sets the entry itself; entries on the way there are moot.
                        LineKind::LeadIn(_) => None,
                        LineKind::FollowUp(_) if state != self.target => Some("follow_up_journal"),
                        LineKind::FollowUp(_) => None,
                    };
                    if let Some(reason) = reason {
                        self.effect(action, reason, line);
                    }
                } else {
                    self.journal_advance(action, action_index, other_tag, state, line, reader);
                }
                continue;
            }
            let writes = match decode_writes(action, self.module, self.tables, reader) {
                Ok(writes) => writes,
                // Does nothing in-game either.
                Err("no_effect") => continue,
                Err(reason) => {
                    self.effect(action, reason, line);
                    continue;
                }
            };
            for (key, change) in writes {
                // `before` carries the other quests' pending writes and this route's
                // earlier lines (the reader's overlay).
                let Ok((saved, before)) = reader.read_saved_and_current(&key) else {
                    self.effect(action, "unavailable_target", line);
                    continue;
                };
                let after = change.apply(before.as_ref());
                let taken = taken_tag(&key);
                let resref = given_resref(&key);
                // No-ops in-game too: nothing of that tag in the backpack, or a value the
                // save already holds with no pending edit in between (a door already
                // unlocked, a feat already missing). Item gives count, not set.
                if before.as_ref() == Some(&after)
                    && (taken.is_some() || (resref.is_none() && saved == before))
                {
                    continue;
                }
                let blueprint = resref.and_then(|resref| self.tables.item(self.module, resref));
                if let (Some(resref), Some(blueprint)) = (resref, &blueprint) {
                    self.blueprints
                        .insert(resref.to_string(), blueprint.clone());
                }
                let (label, label_strref) = if let Some((owner, tag)) = taken {
                    reader.item_label(owner, tag)
                } else if key.value_type == FEAT_TYPE {
                    (None, self.tables.feat_name_strref(&key.name))
                } else if area_field(&key.value_type).is_some() {
                    (None, reader.area_object_label(&key))
                } else if key.value_type == COMPANION_FEAT_TYPE {
                    (
                        None,
                        split_companion_key(&key.name)
                            .and_then(|(_, feat)| self.tables.feat_name_strref(feat)),
                    )
                } else {
                    (
                        blueprint.as_ref().and_then(|b| b.name.clone()),
                        blueprint.as_ref().and_then(|b| b.name_strref),
                    )
                };
                if resref.is_none() {
                    reader.overlay.insert(key.clone(), after.clone());
                }
                self.candidates.push(VariableWrite {
                    key,
                    saved,
                    before,
                    after,
                    change,
                    action_index,
                    label,
                    label_strref,
                    derived_from: action.derived_from.clone(),
                    line,
                });
            }
        }
    }

    /// Another quest's entry on this route: the engine adds it like `AddJournalQuestEntry`
    /// without override, so it never moves back.
    fn journal_advance(
        &mut self,
        action: &ConvoFunctor,
        action_index: usize,
        other_tag: String,
        state: u32,
        line: Option<i32>,
        reader: &mut VariableReader<'_>,
    ) {
        let key = VariableKey {
            module: None,
            name: journal_variable(&other_tag),
            value_type: PC_LOCAL_TYPE.into(),
        };
        // The journal map holds the save, the pending quest targets and this route so far.
        let before = reader
            .journal
            .get_or_insert_default()
            .get(&other_tag)
            .map(|s| Value::from(*s));
        if before.as_ref().and_then(Value::as_u64) >= Some(u64::from(state)) {
            return;
        }
        let after = Value::from(state);
        if let Some(earlier) = self.candidates.iter_mut().find(|w| w.key == key) {
            earlier.after = after.clone();
            earlier.change = Change::Raise(i64::from(state));
        } else {
            let Ok(saved) = reader.read_saved(&key) else {
                self.effect(action, "unavailable_target", line);
                return;
            };
            self.candidates.push(VariableWrite {
                key: key.clone(),
                saved,
                before,
                after: after.clone(),
                change: Change::Raise(i64::from(state)),
                action_index,
                label: None,
                label_strref: None,
                derived_from: None,
                line,
            });
        }
        reader
            .journal
            .get_or_insert_default()
            .insert(other_tag, state);
        reader.overlay.insert(key, after);
    }
}

/// `SaveState` of a preview: the overlay holds the pending edits and the writes of the
/// lines played so far, over the live characters and the save.
impl SaveState for VariableReader<'_> {
    fn current(&mut self, key: &VariableKey) -> Option<i64> {
        self.read(key).ok().flatten().and_then(|v| v.as_i64())
    }

    fn stacks(&mut self, owner: Option<&str>, tag: &str) -> Vec<i64> {
        VariableReader::stacks(self, owner, tag)
    }

    fn party(&mut self) -> Vec<String> {
        self.companion_names()
            .into_iter()
            .filter(|name| {
                let key = VariableKey {
                    module: None,
                    name: name.clone(),
                    value_type: PARTY_TYPE.into(),
                };
                self.current(&key) == Some(1)
            })
            .collect()
    }
}

pub struct VariableReader<'a> {
    handler: &'a SaveGameHandler,
    paths: &'a NWN2Paths,
    player: usize,
    /// Unsaved characters and party, read instead of the save (`read_base`, `read`).
    live: LiveCharacters<'a>,
    globals: Option<Result<crate::parsers::xml::XmlData, String>>,
    modules: BTreeMap<String, Result<content::ModuleVariables, String>>,
    /// The player's struct in `playerlist.ifo`, for `CHARACTER_TYPE` keys.
    player_fields: Option<Result<GffStruct, String>>,
    /// `roster.rst` members, for `PARTY_TYPE` / `SELECTABLE_TYPE` keys.
    roster: Option<Result<Vec<RosterMember>, String>>,
    /// Lowercase roster names that have a `.ros` file.
    companion_names: Option<Vec<String>>,
    /// Root structs of companion `.ros` files by lowercase roster name.
    companions: BTreeMap<String, Result<GffStruct, String>>,
    /// Installed doors/map pins and the saved `<module>.z`, by module, for
    /// `DOOR_LOCK_TYPE` / `MAP_NOTE_TYPE` keys.
    areas: BTreeMap<String, Result<(ModuleObjects, ErfParser), String>>,
    /// Values that take precedence over the live ones: pending and previewed writes.
    pub overlay: BTreeMap<VariableKey, Value>,
    /// Journal states (tag -> entry) for `gc_journal_entry`; `None` = unknown.
    pub journal: Option<BTreeMap<String, u32>>,
}

impl<'a> VariableReader<'a> {
    pub fn new(
        handler: &'a SaveGameHandler,
        paths: &'a NWN2Paths,
        player: usize,
        live: LiveCharacters<'a>,
    ) -> Self {
        Self {
            handler,
            paths,
            player,
            live,
            globals: None,
            modules: BTreeMap::new(),
            player_fields: None,
            roster: None,
            companion_names: None,
            companions: BTreeMap::new(),
            areas: BTreeMap::new(),
            overlay: BTreeMap::new(),
            journal: None,
        }
    }

    /// The player's struct: the loaded player when `live` and it is the quest target,
    /// else `playerlist.ifo`.
    fn player_fields(&mut self, live: bool) -> Result<&GffStruct, String> {
        if live && let Some(player) = &self.live.player {
            return Ok(player);
        }
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

    /// Root struct of a companion by roster name: the one loaded in this session when
    /// `live`, else its `.ros`.
    fn companion_fields(&mut self, ros_name: &str, live: bool) -> Result<&GffStruct, String> {
        if live && let Some(fields) = self.live.companions.get(&ros_name.to_ascii_lowercase()) {
            return Ok(fields);
        }
        let handler = self.handler;
        self.companions
            .entry(ros_name.to_string())
            .or_insert_with(|| {
                let stored = handler
                    .companion_stored_name(ros_name)
                    .map_err(|e| e.to_string())?;
                let bytes = handler.extract_file(&stored).map_err(|e| e.to_string())?;
                owned_root(&bytes)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Backpack owner's live fields: the player for `None`, else that companion.
    fn owner_fields(&mut self, owner: Option<&str>) -> Result<&GffStruct, String> {
        match owner {
            None => self.player_fields(true),
            Some(ros_name) => self.companion_fields(ros_name, true),
        }
    }

    /// Display name of the first backpack item carrying `tag`: embedded text, TLK strref.
    pub fn item_label(&mut self, owner: Option<&str>, tag: &str) -> (Option<String>, Option<i32>) {
        let Ok(fields) = self.owner_fields(owner) else {
            return (None, None);
        };
        backpack_items(fields)
            .find(|item| item_tag_matches(item, tag))
            .map(super::item_blueprints::localized_name)
            .unwrap_or((None, None))
    }

    /// `StackSize` of each backpack item tagged `tag`, live; empty when unreadable.
    pub fn stacks(&mut self, owner: Option<&str>, tag: &str) -> Vec<i64> {
        let Ok(fields) = self.owner_fields(owner) else {
            return Vec::new();
        };
        backpack_items(fields)
            .filter(|item| item_tag_matches(item, tag))
            .map(item_stack_size)
            .collect()
    }

    /// Lowercase names of the roster members with a `.ros`, in `roster.rst` order: a
    /// retired follower can keep its party flag without one and has no inventory.
    fn companion_names(&mut self) -> Vec<String> {
        let handler = self.handler;
        self.companion_names
            .get_or_insert_with(|| {
                roster::companion_members(handler)
                    .map(|members| {
                        members
                            .iter()
                            .map(|m| m.ros_name.to_ascii_lowercase())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .clone()
    }

    fn roster_members(&mut self) -> Result<&Vec<RosterMember>, String> {
        let handler = self.handler;
        self.roster
            .get_or_insert_with(|| {
                let (_, bytes) = handler
                    .extract_roster()
                    .map_err(|e| e.to_string())?
                    .ok_or("Save has no roster file")?;
                roster::parse_roster_members(bytes)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The value with the pending and previewed writes (`overlay`) over the live one.
    pub fn read(&mut self, key: &VariableKey) -> Result<Option<Value>, String> {
        if let Some(value) = self.overlay.get(key) {
            return Ok(Some(value.clone()));
        }
        self.lookup(key, true)
    }

    /// The live value: unsaved characters and party, else the save.
    pub fn read_base(&mut self, key: &VariableKey) -> Result<Option<Value>, String> {
        self.lookup(key, true)
    }

    /// The value in the save on disk.
    pub fn read_saved(&mut self, key: &VariableKey) -> Result<Option<Value>, String> {
        self.lookup(key, false)
    }

    /// `read_saved` and `read` together, reading the save once when no unsaved
    /// character or party state stands in for it.
    pub fn read_saved_and_current(
        &mut self,
        key: &VariableKey,
    ) -> Result<(Option<Value>, Option<Value>), String> {
        let saved = self.lookup(key, false)?;
        let current = match self.overlay.get(key) {
            Some(value) => Some(value.clone()),
            None if self.has_live_value(key) => self.lookup(key, true)?,
            None => saved.clone(),
        };
        Ok((saved, current))
    }

    /// Whether `live` holds the value of `key` instead of the save.
    fn has_live_value(&self, key: &VariableKey) -> bool {
        match key.value_type.as_str() {
            CHARACTER_TYPE | PC_LOCAL_TYPE | FEAT_TYPE | ITEM_TAKE_TYPE => {
                self.live.player.is_some()
            }
            COMPANION_CHARACTER_TYPE | COMPANION_FEAT_TYPE | COMPANION_ITEM_TAKE_TYPE => {
                split_companion_key(&key.name).is_some_and(|(ros_name, _)| {
                    self.live
                        .companions
                        .contains_key(&ros_name.to_ascii_lowercase())
                })
            }
            PARTY_TYPE => self.live.party.contains_key(&key.name.to_ascii_lowercase()),
            _ => false,
        }
    }

    fn lookup(&mut self, key: &VariableKey, live: bool) -> Result<Option<Value>, String> {
        if given_resref(key).is_some() {
            return Ok(None);
        }
        if key.value_type == CHARACTER_TYPE
            || key.value_type == PC_LOCAL_TYPE
            || key.value_type == FEAT_TYPE
            || key.value_type == ITEM_TAKE_TYPE
        {
            let fields = self.player_fields(live)?;
            if key.value_type == ITEM_TAKE_TYPE {
                return Ok(Some(Value::from(backpack_units(fields, &key.name))));
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
        if matches!(
            key.value_type.as_str(),
            COMPANION_CHARACTER_TYPE | COMPANION_FEAT_TYPE | COMPANION_ITEM_TAKE_TYPE
        ) {
            let (ros_name, field) =
                split_companion_key(&key.name).ok_or("Invalid companion key")?;
            let fields = self.companion_fields(ros_name, live)?;
            if key.value_type == COMPANION_ITEM_TAKE_TYPE {
                return Ok(Some(Value::from(backpack_units(fields, field))));
            }
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
            if live
                && key.value_type == PARTY_TYPE
                && let Some(in_party) = self.live.party.get(&key.name.to_ascii_lowercase())
            {
                return Ok(Some(Value::from(i64::from(*in_party))));
            }
            let members = self.roster_members()?;
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
/// only offered reply. `played` runs on each followed line before the walk looks
/// past it, so later conditions see its writes. Stops at a real player choice, at a
/// condition the editor cannot evaluate, at lines the graph did not collect, or at
/// the end of the conversation.
fn follow_conversation<'a>(
    tr: &'a TransitionNode,
    reader: &mut VariableReader<'_>,
    mut played: impl FnMut(&'a FollowNode, &mut VariableReader<'_>),
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
        let mut passing: Vec<&FollowNode> = Vec::new();
        let mut unknown: Option<&FollowNode> = None;
        // A line the graph did not collect, needed to decide where the game goes.
        let mut missing: Option<i32> = None;
        for id in children {
            let Some(candidate) = by_id.get(id).copied() else {
                missing = Some(*id);
                break;
            };
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
        if missing.is_some() {
            break FollowStop {
                reason: "depth".into(),
                node: missing,
                conditions: Vec::new(),
                choices: 0,
            };
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
        played(next, reader);
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
    if local && !is_module_target(condition) {
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

/// `ginc_var_ops.CompareInts`: the first character is the operator (`>`, `<`, `=`, `!`)
/// and the rest goes through `StringToInt`, so `">=4"` compares `> 0`; without an
/// operator the whole check is the value (`"1"`).
fn compare_check(current: i64, check: &str) -> Option<bool> {
    let (op, rest) = match check.chars().next() {
        Some(op @ ('>' | '<' | '=' | '!')) => (op, &check[1..]),
        _ => ('=', check),
    };
    let target = string_to_int(rest)?;
    Some(match op {
        '>' => current > target,
        '<' => current < target,
        '!' => current != target,
        _ => current == target,
    })
}

/// What an assignment action does to its variable, per `ginc_var_ops`. Staged writes keep
/// it and apply it again to the live value when staged and saved, so edits made after
/// the preview (character tabs, other quests) are built on, not overwritten.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Change {
    Set(Value),
    /// `"+3"`, `"-1"`, `"++"`: added to the current value, wrapping like NWScript's
    /// 32-bit `int`.
    Add(i64),
    /// Added to the current value, then kept within `min..=max`: influence the way
    /// `kinc_companion` caps it, gold/XP and backpack units at 0.
    AddClamped {
        delta: i64,
        min: i64,
        max: i64,
    },
    /// Engine `AdjustAlignment` on one axis: kept within 0..100, and a shift that
    /// lands in another band (0-30, 31-69, 70-100) snaps to that band's middle
    /// (15, 50, 85). Verified in-game 2026-10-08: GoodEvil 69 + 1 = 85.
    AdjustAlignment(i64),
    /// `AddJournalQuestEntry` without override: the entry only ever moves up.
    Raise(i64),
}

impl Change {
    pub fn apply(&self, before: Option<&Value>) -> Value {
        let current = || before.and_then(Value::as_i64).unwrap_or(0);
        match self {
            Change::Raise(value) => Value::from(current().max(*value)),
            Change::Set(value) => value.clone(),
            Change::Add(delta) => Value::from((current() as i32).wrapping_add(*delta as i32)),
            Change::AddClamped { delta, min, max } => {
                Value::from((current() + delta).clamp(*min, *max))
            }
            Change::AdjustAlignment(delta) => {
                let before = current().clamp(0, 100);
                let after = (before + delta).clamp(0, 100);
                let band = alignment_band(after);
                Value::from(if band == alignment_band(before) {
                    after
                } else {
                    [15, 50, 85][band]
                })
            }
        }
    }
}

fn alignment_band(value: i64) -> usize {
    match value {
        ..=30 => 0,
        31..=69 => 1,
        _ => 2,
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

/// MotB `kinc_companion.GetCompanionTagByCode`: the first two letters of the code
/// count (`GetFirstChars`), lowercased.
fn motb_companion_tag(code: &str) -> Option<&'static str> {
    let code = code.to_ascii_lowercase();
    Some(match code.get(..2)? {
        "sa" => "safiya",
        "ga" => "gann",
        "ok" => "okku",
        "do" => "dove",
        "on" => "oneofmany",
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
                return Err("no_effect");
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
                // `MagnitudeConversion` turns 0 and anything beyond 5 into no change.
                let delta = motb_magnitude(amount).ok_or("no_effect")?;
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
/// (`ga_take_item` / `ga_destroy_item`): `name` is the item tag (case-sensitive,
/// like `GetTag`), the value how many units of that tag the `ItemList` holds afterwards.
pub const ITEM_TAKE_TYPE: &str = "item_take";
/// `VariableKey::value_type` of an item handed to a companion (party-wide
/// `ga_give_item`): `name` is `<roster name>/<blueprint resref>`, the value the quantity.
pub const COMPANION_ITEM_TYPE: &str = "companion_item";
/// `VariableKey::value_type` of items taken out of a companion's backpack (party-wide
/// `ga_destroy_item`): `name` is `<roster name>/<item tag>`, the value the units left.
pub const COMPANION_ITEM_TAKE_TYPE: &str = "companion_item_take";
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
            | COMPANION_ITEM_TYPE
            | COMPANION_ITEM_TAKE_TYPE
            | DOOR_LOCK_TYPE
            | MAP_NOTE_TYPE
    )
}

/// Write kinds that move a quantity rather than set a value: two quests writing
/// one of them both apply, so they never conflict.
fn accumulates(value_type: &str) -> bool {
    matches!(
        value_type,
        CHARACTER_TYPE
            | COMPANION_CHARACTER_TYPE
            | ITEM_TYPE
            | ITEM_TAKE_TYPE
            | COMPANION_ITEM_TYPE
            | COMPANION_ITEM_TAKE_TYPE
    )
}

/// Backpack owner (`None` = the player) and item tag of an item take write.
fn taken_tag(key: &VariableKey) -> Option<(Option<&str>, &str)> {
    match key.value_type.as_str() {
        ITEM_TAKE_TYPE => Some((None, &key.name)),
        COMPANION_ITEM_TAKE_TYPE => {
            split_companion_key(&key.name).map(|(owner, tag)| (Some(owner), tag))
        }
        _ => None,
    }
}

/// Blueprint resref of an item hand-out write.
fn given_resref(key: &VariableKey) -> Option<&str> {
    match key.value_type.as_str() {
        ITEM_TYPE => Some(&key.name),
        COMPANION_ITEM_TYPE => split_companion_key(&key.name).map(|(_, resref)| resref),
        _ => None,
    }
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
        companion_key(target, field, COMPANION_CHARACTER_TYPE)
    }
}

/// `<roster name>/<field>` of a companion key.
pub fn split_companion_key(name: &str) -> Option<(&str, &str)> {
    name.split_once('/')
}

/// A script target as a creature tag: empty for the player (`ginc_param_const.GetTarget`
/// upper-cases `$PC`, `$PC_SPEAKER`, `$PC_LEADER`, `$PC_NEAREST`, all the PC in single
/// player); other `$`-targets (`$OWNER`, `$OBJECT_SELF`, ...) are not creatures we can reach.
fn target_tag(target: &str) -> Result<&str, &'static str> {
    let target = target.trim();
    if target.is_empty()
        || ["$PC_SPEAKER", "$PC", "$PC_LEADER", "$PC_NEAREST"]
            .iter()
            .any(|pc| target.eq_ignore_ascii_case(pc))
    {
        return Ok("");
    }
    if target.starts_with('$') {
        return Err("object_target");
    }
    Ok(target)
}

/// `ga_local_*` / `gc_local_*` aimed at the module (`GetTarget` upper-cases `$MODULE`).
fn is_module_target(functor: &ConvoFunctor) -> bool {
    functor
        .params
        .get(2)
        .and_then(Value::as_str)
        .is_some_and(|t| t.trim().eq_ignore_ascii_case("$MODULE"))
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
/// by 1, 3, 10 or 70 points towards good/lawful (negative towards evil/chaotic).
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
        Change::AdjustAlignment(magnitude * act.signum()),
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
        companion_key(target, &feat.to_string(), COMPANION_FEAT_TYPE)
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
        // Only closes the door: the swing is not saved state.
        "ga_door_close" => return Err("no_effect"),
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
    state: &mut dyn SaveState,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    let alignment = int_param(action, 0).ok_or("invalid_assignment")?;
    let shift = int_param(action, 1).ok_or("invalid_assignment")?;
    if shift <= 0 {
        return Err("no_effect");
    }
    let target = target_tag(action.params.get(2).and_then(Value::as_str).unwrap_or(""))?;
    let shift_axis =
        |field: &str, delta: i64| (creature_key(target, field), Change::AdjustAlignment(delta));
    let writes = match alignment {
        0 => vec![
            shift_axis(LAWFUL_CHAOTIC, shift),
            shift_axis(GOOD_EVIL, shift),
        ],
        1 => [LAWFUL_CHAOTIC, GOOD_EVIL]
            .into_iter()
            .filter_map(|field| {
                let value = state.current(&creature_key(target, field)).unwrap_or(50);
                let delta = (50 - value).clamp(-shift, shift);
                (delta != 0).then(|| shift_axis(field, delta))
            })
            .collect(),
        2 => vec![shift_axis(LAWFUL_CHAOTIC, shift)],
        3 => vec![shift_axis(LAWFUL_CHAOTIC, -shift)],
        4 => vec![shift_axis(GOOD_EVIL, shift)],
        5 => vec![shift_axis(GOOD_EVIL, -shift)],
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
        writes.push((qpr_key(tag), Change::Add(percent)));
    }
    if writes.is_empty() {
        return Err("no_effect");
    }
    Ok(writes)
}

/// `ga_give_quest_xp(tag)` per `ginc_journal.RewardPartyQuestXP`: the journal category XP,
/// uncapped, and 100 added to the paid share `<tag>_XP_QPR` (even for a 0-XP quest), so
/// a later `ga_give_partial_quest_xp` pays nothing more.
fn decode_quest_xp(
    action: &ConvoFunctor,
    tables: &RewardTables,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    let tag = action
        .params
        .first()
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("invalid_assignment")?;
    let xp = tables.quest_xp(tag).ok_or("unavailable_target")?;
    let mut writes = Vec::new();
    if xp != 0 {
        writes.push(reward(EXPERIENCE, xp)?);
    }
    writes.push((qpr_key(tag), Change::Add(100)));
    Ok(writes)
}

fn qpr_key(tag: &str) -> VariableKey {
    VariableKey {
        module: None,
        name: format!("{tag}{QPR_SUFFIX}"),
        value_type: PC_LOCAL_TYPE.into(),
    }
}

/// What the reward decoders read from the save as staged so far.
pub trait SaveState {
    /// Integer value of `key`, with the pending and earlier writes applied.
    fn current(&mut self, key: &VariableKey) -> Option<i64>;
    /// `StackSize` of each backpack item tagged `tag` of the player (`owner` `None`)
    /// or a companion (roster name), in `ItemList` order, as saved.
    fn stacks(&mut self, owner: Option<&str>, tag: &str) -> Vec<i64>;
    /// Roster names of the companions in the party, in roster order.
    fn party(&mut self) -> Vec<String>;
}

/// `decode_action` for every write of an action: `ga_give_partial_quest_xp` changes
/// XP and the paid share together and needs the share's current value; the item
/// scripts need the backpacks and, with their party flag, the party.
pub fn decode_writes(
    action: &ConvoFunctor,
    module: &str,
    tables: &RewardTables,
    state: &mut dyn SaveState,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    if action.script.eq_ignore_ascii_case("AdjustAlignment") {
        return decode_alignment_points(action, state);
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
        let rewarded = state.current(&qpr_key(tag)).unwrap_or(0);
        return decode_partial_quest_xp(action, tables, rewarded);
    }
    if action.script.eq_ignore_ascii_case("ga_give_quest_xp") {
        return decode_quest_xp(action, tables);
    }
    if action.script.eq_ignore_ascii_case("ga_take_item") {
        return decode_take_item(action, state);
    }
    if action.script.eq_ignore_ascii_case("ga_destroy_item") {
        return decode_destroy_item(action, state);
    }
    let write = decode_action(action, module, tables)?;
    // The party flag of `ga_give_item` reaches companions only where the campaign's
    // own copy asks for every faction member (SoZ), not the stock one's PCs.
    if write.0.value_type == ITEM_TYPE
        && int_param(action, 2).unwrap_or(0) != 0
        && tables.party_gives.contains(module)
    {
        let mut writes = vec![write.clone()];
        for ros_name in state.party() {
            writes.push((
                companion_key(&ros_name, &write.0.name, COMPANION_ITEM_TYPE),
                write.1.clone(),
            ));
        }
        return Ok(writes);
    }
    Ok(vec![write])
}

/// Key of `field` on a companion (`<lowercase roster name>/<field>`), the counterpart
/// of `split_companion_key`.
fn companion_key(ros_name: &str, field: &str, value_type: &str) -> VariableKey {
    VariableKey {
        module: None,
        name: format!("{}/{field}", ros_name.to_ascii_lowercase()),
        value_type: value_type.into(),
    }
}

/// Campaign-side data the reward scripts look up at runtime.
pub struct RewardTables<'a> {
    /// Journal category XP by lowercase quest tag (`GetJournalQuestExperience`).
    quest_xp: BTreeMap<String, i64>,
    /// Campaign folder holding `k_xp_awards.2da` (`ga_xp_award_2da_entry`) when there
    /// is no resource manager (whose override chain indexes the campaign folder).
    campaign_path: Option<PathBuf>,
    xp_awards: OnceCell<Option<Arc<TDAParser>>>,
    /// Item blueprints for `ga_give_item`, and the module's own item scripts.
    items: ItemBlueprints<'a>,
    /// `feat.2da` with overrides, for the names of `ga_give_feat` writes.
    resources: Option<&'a ResourceManager>,
    feats: OnceCell<Option<Arc<TDAParser>>>,
    /// Modules whose `ga_give_item` gives to companions too (`party_item_gifts`).
    party_gives: BTreeSet<String>,
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
            party_gives: graph
                .modules
                .iter()
                .filter(|m| m.party_item_gifts)
                .map(|m| m.name.clone())
                .collect(),
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
            party_gives: BTreeSet::new(),
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
            .get_or_init(|| {
                if let Some(resources) = self.resources {
                    return resources.get_2da_with_overrides("k_xp_awards").ok();
                }
                let path = self.campaign_path.as_deref()?.join("k_xp_awards.2da");
                let mut parser = TDAParser::new();
                parser.parse_from_file(&path).ok()?;
                Some(Arc::new(parser))
            })
            .as_ref()?;
        let row = usize::try_from(row).ok()?;
        let cell = table.get_cell_by_name(row, "XP").ok()?;
        Some(string_to_int(cell.unwrap_or("")).unwrap_or(0))
    }
}

const MAX_GIVEN_ITEMS: i64 = 9999;

/// `decode_assignment` plus the reward scripts whose amount lives outside the dialogue:
/// `ga_xp_award_2da_entry(row)` awards the campaign's `k_xp_awards.2da` entry.
pub fn decode_action(
    action: &ConvoFunctor,
    module: &str,
    tables: &RewardTables,
) -> Result<(VariableKey, Change), &'static str> {
    let script = action.script.to_ascii_lowercase();
    match script.as_str() {
        // `ga_give_item(sTemplate, nQuantity, bAllPartyMembers)`: nQuantity below 1 is 1.
        // The player's copy; `decode_writes` adds the party's.
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
            // The script creates them one by one; no dialogue hands out more.
            if quantity > MAX_GIVEN_ITEMS {
                return Err("invalid_assignment");
            }
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
        "ga_xp_award_2da_entry" => {
            let row = int_param(action, 0).ok_or("invalid_assignment")?;
            let xp = tables.xp_award(row).ok_or("unavailable_target")?;
            reward(EXPERIENCE, xp)
        }
        _ => decode_assignment(action, module),
    }
}

/// Tag of an item script (`sItemTag`); matched exactly, like `GetTag(oItem) == sItem`.
fn item_tag_param(action: &ConvoFunctor) -> Result<String, &'static str> {
    action
        .params
        .first()
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or("invalid_assignment")
}

fn take_units_change(units: i64) -> Change {
    Change::AddClamped {
        delta: -units,
        min: 0,
        max: i64::MAX,
    }
}

/// `ga_take_item(sItemTag, nQuantity, bAllPartyMembers)`: `TakeNumItems` takes whole
/// backpack items (one stack per count, never the equipped slots), 0 reads as 1 and a
/// negative count takes everything. The party flag only reaches other PCs
/// (`GetFirstFactionMember` default).
fn decode_take_item(
    action: &ConvoFunctor,
    state: &mut dyn SaveState,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    let tag = item_tag_param(action)?;
    let key = VariableKey {
        module: None,
        name: tag.clone(),
        value_type: ITEM_TAKE_TYPE.into(),
    };
    let count = int_param(action, 1).filter(|q| *q != 0).unwrap_or(1);
    if count < 0 {
        return Ok(vec![(key, Change::Set(Value::from(0)))]);
    }
    // The items before this one are already gone: skip the units taken so far.
    let stacks = state.stacks(None, &tag);
    let saved: i64 = stacks.iter().sum();
    let mut skip = (saved - state.current(&key).unwrap_or(saved)).max(0);
    let mut units = 0;
    let mut items = 0;
    for stack in stacks {
        let left = stack - skip.min(stack);
        skip -= stack - left;
        if left > 0 && items < count {
            units += left;
            items += 1;
        }
    }
    Ok(vec![(key, take_units_change(units))])
}

/// `ga_destroy_item(sItemTag, nQuantity, bPCFaction)`: destroys units of the tag from
/// the backpack, nothing for 0, everything for a negative count. With `bPCFaction` it
/// walks the whole faction, the PC first, then the party in roster order, until the
/// count is met.
fn decode_destroy_item(
    action: &ConvoFunctor,
    state: &mut dyn SaveState,
) -> Result<Vec<(VariableKey, Change)>, &'static str> {
    let tag = item_tag_param(action)?;
    let quantity = int_param(action, 1).unwrap_or(0);
    if quantity == 0 {
        return Ok(Vec::new());
    }
    let mut owners = vec![VariableKey {
        module: None,
        name: tag.clone(),
        value_type: ITEM_TAKE_TYPE.into(),
    }];
    if int_param(action, 2).unwrap_or(0) != 0 {
        owners.extend(
            state
                .party()
                .iter()
                .map(|ros_name| companion_key(ros_name, &tag, COMPANION_ITEM_TAKE_TYPE)),
        );
    }
    if quantity < 0 {
        return Ok(owners
            .into_iter()
            .map(|key| (key, Change::Set(Value::from(0))))
            .collect());
    }
    let mut writes = Vec::new();
    let mut remaining = quantity;
    for key in owners {
        if remaining == 0 {
            break;
        }
        let units = state.current(&key).unwrap_or(0).min(remaining);
        remaining -= units;
        writes.push((key, take_units_change(units)));
    }
    Ok(writes)
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
    if local && !is_module_target(action) {
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
            // StringToInt: "3x" is 3.
            string_to_int(delta).ok_or("invalid_assignment")?
        };
        Change::Add(sign * magnitude)
    } else {
        let number = raw.strip_prefix('=').unwrap_or(raw);
        // ginc_var_ops.GetChangeComponents treats a signed operand after '=' as 1.
        // Leave those surprising expressions for the engine instead of guessing intent;
        // a decoded wrapper's `"=-1"` is its `SetGlobalInt(x, -1)`.
        let decoded_negative = action.derived_from.is_some() && number.starts_with('-');
        if kind == "int" {
            if (number.starts_with(['+', '-']) && !decoded_negative) || number.is_empty() {
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

fn owned_root(bytes: &[u8]) -> Result<indexmap::IndexMap<String, GffValue<'static>>, String> {
    let parser = GffParser::from_bytes(bytes.to_vec()).map_err(|e| e.to_string())?;
    parser.read_owned_struct(0).map_err(|e| e.to_string())
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
        None if name == GOLD || name == EXPERIENCE => {
            GffValue::Dword(u32::try_from(after).unwrap_or(u32::MAX))
        }
        None if is_alignment_field(name) => GffValue::Byte(u8::try_from(after).unwrap_or(100)),
        None => GffValue::Int(i32::try_from(after).unwrap_or(i32::MAX)),
        Some(_) => return Err(format!("{name} has an unexpected storage type")),
    };
    fields.insert(name.into(), value.clone());
    Ok(value)
}

/// Apply `change` to an integer field the way `add_to_field` stores it; returns the
/// value as written and how far it moved.
fn change_field(
    fields: &mut GffStruct,
    name: &str,
    change: &Change,
) -> Result<(GffValue<'static>, i64), String> {
    let current = fields.get(name).and_then(gff_integer).unwrap_or(0);
    let after = change
        .apply(Some(&Value::from(current)))
        .as_i64()
        .ok_or_else(|| format!("{name} change is not an integer"))?;
    let value = add_to_field(fields, name, after - current)?;
    let moved = gff_integer(&value).unwrap_or(current) - current;
    Ok((value, moved))
}

/// Player fields as written by a reward flush: `(field name, value)`.
pub type WrittenFields = Vec<(String, GffValue<'static>)>;

/// Everything a quest flush wrote onto creatures, for mirroring into loaded characters.
#[derive(Debug, Default)]
pub struct FlushedWrites {
    pub player: WrittenFields,
    /// Per companion (lowercase roster name) fields written to its `.ros`.
    pub companions: Vec<(String, WrittenFields)>,
    /// Non-fatal problems to show with the save result.
    pub warnings: Vec<String>,
}

/// Apply reward changes in order to the player's struct in `playerlist.ifo`; returns the
/// new bytes and the fields as written.
pub fn add_player_rewards(
    bytes: &[u8],
    player: usize,
    rewards: &[(String, Change)],
) -> Result<(Vec<u8>, WrittenFields), String> {
    if rewards.is_empty() {
        return Ok((bytes.to_vec(), Vec::new()));
    }
    let mut root = owned_root(bytes)?;
    let Some(GffValue::ListOwned(players)) = root.get_mut("Mod_PlayerList") else {
        return Err("Mod_PlayerList missing".into());
    };
    let fields = players.get_mut(player).ok_or("Primary player missing")?;
    let mut written: WrittenFields = Vec::with_capacity(rewards.len());
    for (name, change) in rewards {
        let (value, _) = change_field(fields, name, change)?;
        written.retain(|(field, _)| field != name);
        written.push((name.clone(), value));
    }
    let bytes = GffWriter::new("IFO ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())?;
    Ok((bytes, written))
}

/// Same changes on the root struct of `player.bic`.
pub fn add_bic_rewards(bytes: &[u8], rewards: &[(String, Change)]) -> Result<Vec<u8>, String> {
    let mut root = owned_root(bytes)?;
    for (name, change) in rewards {
        change_field(&mut root, name, change)?;
    }
    GffWriter::new("BIC ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())
}

/// An item change, applied in staging order: a blueprint handed out, or a take whose
/// change applies to the units of the tag as they are by then.
pub enum ItemOp {
    Give(Arc<ItemBlueprint>, i64),
    Take(String, Change),
}

/// Give the reward items and take the turn-ins in the player's `ItemList` in
/// `playerlist.ifo`; returns the new bytes and the list as written.
pub fn apply_player_items(
    bytes: &[u8],
    player: usize,
    ops: &[ItemOp],
    game_data: &GameData,
) -> Result<(Vec<u8>, WrittenFields), String> {
    if ops.is_empty() {
        return Ok((bytes.to_vec(), Vec::new()));
    }
    let mut root = owned_root(bytes)?;
    let Some(GffValue::ListOwned(players)) = root.get_mut("Mod_PlayerList") else {
        return Err("Mod_PlayerList missing".into());
    };
    let slot = players.get_mut(player).ok_or("Primary player missing")?;
    let fields = apply_item_ops(std::mem::take(slot), ops, game_data)?;
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
    ops: &[ItemOp],
    game_data: &GameData,
) -> Result<Vec<u8>, String> {
    if ops.is_empty() {
        return Ok(bytes.to_vec());
    }
    let root = apply_item_ops(owned_root(bytes)?, ops, game_data)?;
    GffWriter::new("BIC ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())
}

/// Apply item changes in order through the Inventory tab's add path, so identification
/// and stacking match.
fn apply_item_ops(
    fields: GffStruct,
    ops: &[ItemOp],
    game_data: &GameData,
) -> Result<GffStruct, String> {
    let mut character = Character::from_gff(fields).map_err(|e| e.to_string())?;
    for op in ops {
        match op {
            ItemOp::Give(blueprint, quantity) => {
                give_items_to(&mut character, blueprint, *quantity, game_data)?;
            }
            ItemOp::Take(tag, change) => {
                let units = taken_units(character.gff(), tag, change);
                take_units(character.gff_mut(), tag, units);
            }
        }
    }
    Ok(character.into_gff())
}

/// `CreateItemOnObject` one at a time, the way `ga_give_item` loops: stackable items
/// fill stacks up to `baseitems.Stacking`, everything else is one entry per copy.
fn give_items_to(
    character: &mut Character,
    blueprint: &ItemBlueprint,
    quantity: i64,
    game_data: &GameData,
) -> Result<(), String> {
    let base_item = blueprint
        .fields
        .get("BaseItem")
        .and_then(gff_integer)
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(0);
    let max_stack = character.max_stack(base_item, game_data);
    let mut remaining = quantity;
    while remaining > 0 {
        let count = remaining.min(i64::from(max_stack));
        let mut item = blueprint.fields.clone();
        if max_stack > 1 {
            set_stack_size(&mut item, count);
        }
        character
            .add_item_from_struct(item, game_data)
            .map_err(|e| e.to_string())?;
        remaining -= count;
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

/// `GetTag(oItem) == sItem`: an exact, case-sensitive match.
fn item_tag_matches(item: &GffStruct, tag: &str) -> bool {
    item_tag(item) == Some(tag)
}

/// Units of `tag` in the backpack.
fn backpack_units(fields: &indexmap::IndexMap<String, GffValue<'static>>, tag: &str) -> i64 {
    backpack_items(fields)
        .filter(|item| item_tag_matches(item, tag))
        .map(item_stack_size)
        .sum()
}

fn item_stack_size(item: &indexmap::IndexMap<String, GffValue<'static>>) -> i64 {
    item.get("StackSize")
        .and_then(gff_integer)
        .unwrap_or(1)
        .max(1)
}

/// How many units of `tag` a take's change removes from the backpack as it is now.
fn taken_units(fields: &GffStruct, tag: &str, change: &Change) -> i64 {
    let current = backpack_units(fields, tag);
    let after = change
        .apply(Some(&Value::from(current)))
        .as_i64()
        .unwrap_or(current);
    (current - after).max(0)
}

/// `ga_destroy_item`'s loop: walk the backpack in order, drop whole stacks of the tag
/// while they fit into the remaining count, shrink the stack that does not. Returns
/// the entries touched, for putting them back in place.
fn take_units(fields: &mut GffStruct, tag: &str, units: i64) -> Vec<TakenItem> {
    let mut taken = Vec::new();
    let Some(GffValue::ListOwned(list)) = fields.get_mut("ItemList") else {
        return taken;
    };
    let mut remaining = units;
    let mut index = 0;
    while remaining > 0 && index < list.len() {
        if !item_tag_matches(&list[index], tag) {
            index += 1;
            continue;
        }
        let stack = item_stack_size(&list[index]);
        if stack <= remaining {
            remaining -= stack;
            taken.push(TakenItem {
                index,
                item: list.remove(index),
                units: stack,
                removed: true,
            });
            continue;
        }
        let item = &mut list[index];
        taken.push(TakenItem {
            index,
            item: item.clone(),
            units: remaining,
            removed: false,
        });
        set_stack_size(item, stack - remaining);
        remaining = 0;
    }
    taken
}

/// Apply the player's writes of a staged edit (`applies_to_player`) to the loaded
/// player the way `flush` would write them, each change on the value as it is now;
/// returns the changes for `revert_on_player`.
pub fn apply_to_player(
    edit: &PendingQuestEdit,
    character: &mut Character,
    game_data: &GameData,
) -> Result<Vec<MemoryChange>, String> {
    let mut changes = Vec::new();
    for write in &edit.writes {
        match write.key.value_type.as_str() {
            CHARACTER_TYPE => {
                let (_, delta) = change_field(character.gff_mut(), &write.key.name, &write.change)?;
                changes.push(MemoryChange::Field {
                    name: write.key.name.clone(),
                    delta,
                });
            }
            FEAT_TYPE => {
                let id = write
                    .key
                    .name
                    .parse::<i32>()
                    .map_err(|_| "Invalid feat id")?;
                let owned = write.after.as_i64().unwrap_or(0) != 0;
                if owned == has_feat(character.gff(), i64::from(id)) {
                    continue;
                }
                if owned {
                    set_feat(character, id, true);
                    changes.push(MemoryChange::FeatAdded { id });
                } else {
                    changes.push(MemoryChange::FeatRemoved(remove_feat_entries(
                        character, id,
                    )));
                }
            }
            ITEM_TYPE => {
                let blueprint = edit
                    .blueprints
                    .get(&write.key.name)
                    .ok_or_else(|| format!("Item blueprint {} missing", write.key.name))?;
                let units = write.after.as_i64().unwrap_or(0);
                if units > 0 {
                    changes.push(MemoryChange::ItemsGiven(give_tracked(
                        character, blueprint, units, game_data,
                    )?));
                }
            }
            ITEM_TAKE_TYPE => {
                let units = taken_units(character.gff(), &write.key.name, &write.change);
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

/// Give `units` of a blueprint, recording the `ItemList` entries that added or grew.
fn give_tracked(
    character: &mut Character,
    blueprint: &Arc<ItemBlueprint>,
    units: i64,
    game_data: &GameData,
) -> Result<GivenItems, String> {
    let stacks = |character: &Character| -> Vec<i64> {
        backpack_items(character.gff())
            .map(item_stack_size)
            .collect()
    };
    let before = stacks(character);
    give_items_to(character, blueprint, units, game_data)?;
    let after = stacks(character);
    Ok(GivenItems {
        tag: item_tag(&blueprint.fields).unwrap_or_default().to_string(),
        units,
        added: (before.len()..after.len()).collect(),
        grown: before
            .iter()
            .zip(&after)
            .enumerate()
            .filter(|(_, (old, new))| new > old)
            .map(|(index, (old, _))| (index, *old))
            .collect(),
    })
}

fn item_tag(item: &GffStruct) -> Option<&str> {
    match item.get("Tag") {
        Some(GffValue::String(tag)) => Some(tag.as_ref()),
        _ => None,
    }
}

fn item_list_mut(character: &mut Character) -> Result<&mut Vec<GffStruct>, String> {
    match character
        .gff_mut()
        .entry("ItemList".to_string())
        .or_insert_with(|| GffValue::ListOwned(Vec::new()))
    {
        GffValue::ListOwned(list) => Ok(list),
        _ => Err("ItemList is not a list".into()),
    }
}

fn set_stack_size(item: &mut GffStruct, units: i64) {
    let value = crate::character::typed_stack_size(
        item.get("StackSize"),
        i32::try_from(units).unwrap_or(i32::MAX),
    );
    item.insert("StackSize".into(), value);
}

/// Take back what `apply_to_player` changed, newest first, putting entries back where
/// they were. Entries later edits moved are matched by tag instead.
pub fn revert_on_player(changes: &[MemoryChange], character: &mut Character) -> Result<(), String> {
    for change in changes.iter().rev() {
        match change {
            MemoryChange::Field { name, delta } => {
                add_to_field(character.gff_mut(), name, -delta)?;
            }
            MemoryChange::FeatAdded { id } => set_feat(character, *id, false),
            MemoryChange::FeatRemoved(removed) => restore_feat(character, removed),
            MemoryChange::ItemsGiven(given) => {
                let list = item_list_mut(character)?;
                let ours = |index: &usize| {
                    list.get(*index)
                        .is_some_and(|item| item_tag(item) == Some(given.tag.as_str()))
                };
                if given.added.iter().all(ours) && given.grown.iter().all(|(i, _)| ours(i)) {
                    for (index, stack) in &given.grown {
                        set_stack_size(&mut list[*index], *stack);
                    }
                    for index in given.added.iter().rev() {
                        list.remove(*index);
                    }
                } else {
                    take_units(character.gff_mut(), &given.tag, given.units);
                }
            }
            MemoryChange::ItemsTaken { items } => {
                let list = item_list_mut(character)?;
                for taken in items.iter().rev() {
                    let tag = item_tag(&taken.item);
                    if taken.removed {
                        list.insert(taken.index.min(list.len()), taken.item.clone());
                    } else if let Some(item) = list
                        .get_mut(taken.index)
                        .filter(|item| item_tag(item) == tag)
                    {
                        set_stack_size(item, item_stack_size(&taken.item));
                    } else {
                        let mut item = taken.item.clone();
                        set_stack_size(&mut item, taken.units);
                        list.push(item);
                    }
                }
            }
        }
    }
    Ok(())
}

/// `FeatRemove` that remembers the feat's entries and where they sat.
fn remove_feat_entries(character: &mut Character, id: i32) -> RemovedFeat {
    let is_feat =
        |entry: &GffStruct| entry.get("Feat").and_then(gff_integer) == Some(i64::from(id));
    let entries = character
        .get_list("FeatList")
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, entry)| is_feat(entry))
        .map(|(index, entry)| (index, entry.clone()))
        .collect();
    let levels = character
        .get_list("LvlStatList")
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(level, entry)| match entry.get("FeatList") {
            Some(GffValue::ListOwned(feats)) => feats
                .iter()
                .enumerate()
                .filter(|(_, entry)| is_feat(entry))
                .map(|(index, entry)| (level, index, entry.clone()))
                .collect(),
            _ => Vec::new(),
        })
        .collect();
    set_feat(character, id, false);
    RemovedFeat {
        id,
        entries,
        levels,
    }
}

/// Put a removed feat's entries back at their positions, unless it was added again.
fn restore_feat(character: &mut Character, removed: &RemovedFeat) {
    if has_feat(character.gff(), i64::from(removed.id)) {
        return;
    }
    let mut list = character.get_list_owned("FeatList").unwrap_or_default();
    for (index, entry) in &removed.entries {
        list.insert((*index).min(list.len()), entry.clone());
    }
    character.set_list("FeatList", list);
    if removed.levels.is_empty() {
        return;
    }
    let mut levels = character.get_list_owned("LvlStatList").unwrap_or_default();
    for (level, index, entry) in &removed.levels {
        if let Some(GffValue::ListOwned(feats)) = levels.get_mut(*level).map(|level| {
            level
                .entry("FeatList".to_string())
                .or_insert_with(|| GffValue::ListOwned(Vec::new()))
        }) {
            feats.insert((*index).min(feats.len()), entry.clone());
        }
    }
    character.set_list("LvlStatList", levels);
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
    let fields = set_feats(std::mem::take(slot), feats)?;
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

/// What a flush changes on one companion's `.ros`.
#[derive(Default)]
struct CompanionChanges {
    /// Alignment changes in staging order.
    rewards: Vec<(String, Change)>,
    feats: Vec<(i32, bool)>,
    items: Vec<ItemOp>,
}

/// Apply `changes` to a companion's `.ros` root, keeping the file's type, version
/// and root struct id; returns the new bytes and the fields as written.
fn update_companion(
    bytes: Vec<u8>,
    changes: &CompanionChanges,
    game_data: &GameData,
) -> Result<(Vec<u8>, WrittenFields), String> {
    let mut written: WrittenFields = Vec::new();
    let bytes = rewrite_gff(bytes, "ros", |root| {
        for (name, change) in &changes.rewards {
            let (value, _) = change_field(root, name, change)?;
            written.retain(|(field, _)| field != name);
            written.push((name.clone(), value));
        }
        if !changes.feats.is_empty() {
            *root = set_feats(std::mem::take(root), &changes.feats)?;
            let list = root
                .get("FeatList")
                .cloned()
                .ok_or("FeatList missing after changing feats")?;
            written.push(("FeatList".into(), list));
        }
        if !changes.items.is_empty() {
            *root = apply_item_ops(std::mem::take(root), &changes.items, game_data)?;
            let list = root
                .get("ItemList")
                .cloned()
                .ok_or("ItemList missing after changing items")?;
            written.push(("ItemList".into(), list));
        }
        Ok(())
    })?;
    Ok((bytes, written))
}

/// Same feat changes on the root struct of `player.bic`.
pub fn set_bic_feats(bytes: &[u8], feats: &[(i32, bool)]) -> Result<Vec<u8>, String> {
    if feats.is_empty() {
        return Ok(bytes.to_vec());
    }
    let root = set_feats(owned_root(bytes)?, feats)?;
    GffWriter::new("BIC ", "V3.2")
        .write(root)
        .map_err(|e| e.to_string())
}

/// Uses the Feats tab's add/remove path; a feat already in (or absent from) the list
/// is left alone, as `FeatAdd` / `FeatRemove` do.
fn set_feats(
    fields: indexmap::IndexMap<String, GffValue<'static>>,
    feats: &[(i32, bool)],
) -> Result<indexmap::IndexMap<String, GffValue<'static>>, String> {
    let mut character = Character::from_gff(fields).map_err(|e| e.to_string())?;
    for (feat, owned) in feats {
        set_feat(&mut character, *feat, *owned);
    }
    let mut fields = character.into_gff();
    fields
        .entry("FeatList".into())
        .or_insert_with(|| GffValue::ListOwned(Vec::new()));
    Ok(fields)
}

const JOURNAL_PREFIX: &str = "NW_JOURNAL_ENTRY";

/// The PC local the journal keeps a quest's state in.
fn journal_variable(tag: &str) -> String {
    format!("{JOURNAL_PREFIX}{tag}")
}

/// Value of the integer local `name` on the player's struct, `None` when unset.
pub(crate) fn pc_int_local(
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
    if edits.iter().any(|(tag, _)| tag.is_empty()) {
        return Err("Empty quest tag".into());
    }
    let locals: Vec<_> = edits
        .iter()
        .map(|(tag, state)| (journal_variable(tag), i64::from(*state)))
        .collect();
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
    let mut root = parser.read_owned_struct(0).map_err(|e| e.to_string())?;
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
        // `GetFirstChars`: two letters decide; "on" is One of Many, a lone "o" nobody.
        let (key, _) =
            decode_assignment(&action("ka_influence_magnitude", json!(["OK", 1])), "m").unwrap();
        assert_eq!(key.name, "00_nInfluenceokku");
        let (key, _) =
            decode_assignment(&action("ka_influence_magnitude", json!(["on", 1])), "m").unwrap();
        assert_eq!(key.name, "00_nInfluenceoneofmany");
        assert!(
            decode_assignment(&action("ka_influence_magnitude", json!(["o", 1])), "m").is_err()
        );
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
        // `RewardPartyQuestXP`: the XP, and the full share marked as paid so a later
        // `ga_give_partial_quest_xp` pays nothing more.
        let mut state = Values(|_: &VariableKey| None);
        let writes = decode_writes(
            &action("ga_give_quest_xp", json!(["paid"])),
            "m",
            &tables,
            &mut state,
        )
        .unwrap();
        assert_eq!(
            writes,
            vec![
                (
                    character_key("Experience"),
                    Change::AddClamped {
                        delta: 300,
                        min: 0,
                        max: i64::from(i32::MAX)
                    }
                ),
                (qpr_key("paid"), Change::Add(100)),
            ]
        );
        assert_eq!(
            decode_writes(
                &action("ga_give_quest_xp", json!(["other"])),
                "m",
                &tables,
                &mut state
            ),
            Err("unavailable_target")
        );
        graph.quests[0].category.xp = 0;
        let tables = RewardTables::from_graph(&graph, None);
        assert_eq!(
            decode_writes(
                &action("ga_give_quest_xp", json!(["paid"])),
                "m",
                &tables,
                &mut state
            ),
            Ok(vec![(qpr_key("paid"), Change::Add(100))])
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
        let handler = SaveGameHandler::new(dir.path()).unwrap();
        (dir, handler)
    }

    /// A stack of `count` `qe_head` items.
    fn head(count: u16) -> indexmap::IndexMap<String, GffValue<'static>> {
        indexmap! {
            "BaseItem".into() => GffValue::Int(12),
            "Tag".into() => GffValue::String("qe_head".into()),
            "StackSize".into() => GffValue::Word(count),
        }
    }

    /// The rich fixture plus a two-member `roster.rst` (khelgar in the party and
    /// selectable, neeshka benched) and a `.ros` per member carrying the faction
    /// the roster state implies. The PC's party faction is set to 9. The PC carries
    /// 2 `qe_head`, each companion 4.
    fn roster_fixture_handler() -> (tempfile::TempDir, SaveGameHandler) {
        use std::io::Write;
        let mut player = owned_players(&playerlist()).swap_remove(0);
        player.insert("FactionID".into(), GffValue::Word(9));
        player.insert("ItemList".into(), GffValue::ListOwned(vec![head(2)]));
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
                        "ItemList".into() => GffValue::ListOwned(vec![head(4)]),
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
        let handler = SaveGameHandler::new(dir.path()).unwrap();
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
                LiveCharacters::default(),
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
            (Some(json!(20)), json!(85))
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
        let written = edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
            .unwrap();
        handler.commit_batch().unwrap();
        assert!(written.player.is_empty());
        assert_eq!(written.companions.len(), 2);
        assert_eq!(written.companions[0].0, "khelgar");
        assert_eq!(written.companions[0].1[0].0, "LawfulChaotic");
        assert_eq!(gff_integer(&written.companions[0].1[0].1), Some(85));

        let ros = |name: &str| {
            let stored = handler.companion_stored_name(name).unwrap();
            let bytes = handler.extract_file(&stored).unwrap();
            let parser = GffParser::from_bytes(bytes.clone()).unwrap();
            assert_eq!(parser.file_type, "ROS ");
            assert_eq!(parser.get_struct_id(0).unwrap(), 0xFFFF_FFFF);
            owned_root(&bytes).unwrap()
        };
        let khelgar = ros("khelgar");
        assert_eq!(gff_integer(&khelgar["LawfulChaotic"]), Some(85));
        assert!(matches!(khelgar["LawfulChaotic"], GffValue::Byte(_)));
        assert_eq!(gff_integer(&khelgar["GoodEvil"]), Some(60));
        assert_eq!(gff_integer(&khelgar["FactionID"]), Some(9));
        let neeshka = ros("neeshka");
        assert!(!has_feat(&neeshka, 45) && has_feat(&neeshka, 1705));
        assert_eq!(gff_integer(&neeshka["LawfulChaotic"]), Some(20));
    }

    /// `ga_destroy_item(tag, n, 1)` takes from the PC first, then the party in roster
    /// order until `n` is met; SoZ's `ga_give_item` copy gives every party member the
    /// full quantity, the stock script only the PC. Benched companions are untouched.
    #[test]
    fn party_wide_item_scripts_write_the_companions_ros() {
        let (_dir, mut handler) = roster_fixture_handler();
        let campaign = tempfile::tempdir().unwrap();
        let sword = GffWriter::new("UTI ", "V3.2")
            .write(indexmap! {
                "BaseItem".into() => GffValue::Int(12),
                "Tag".into() => GffValue::String("qe_sword".into()),
            })
            .unwrap();
        std::fs::write(campaign.path().join("qe_sword.uti"), sword).unwrap();
        let mut graph = graph();
        graph.campaign.campaign_path = Some(campaign.path().display().to_string());
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_destroy_item", json!(["qe_head", 5, 1])),
                action("ga_give_item", json!(["qe_sword", 1, 1])),
            ],
        );
        let writes = |preview: &QuestPreview| {
            preview
                .writes
                .iter()
                .map(|w| {
                    (
                        w.key.value_type.clone(),
                        w.key.name.clone(),
                        w.before.clone(),
                        w.after.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let preview = |tables: &RewardTables| {
            QuestEdits::default()
                .preview(
                    &graph,
                    tables,
                    &handler,
                    &NWN2Paths::new(),
                    0,
                    LiveCharacters::default(),
                    "test",
                    20,
                    Some(0),
                )
                .unwrap()
        };
        let take = |owner: &str, before: i64, after: i64| {
            let (value_type, name) = if owner.is_empty() {
                (ITEM_TAKE_TYPE, "qe_head".to_string())
            } else {
                (COMPANION_ITEM_TAKE_TYPE, format!("{owner}/qe_head"))
            };
            (
                value_type.to_string(),
                name,
                Some(json!(before)),
                json!(after),
            )
        };
        let stock = preview(&RewardTables::from_graph(&graph, None));
        assert_eq!(
            writes(&stock),
            vec![
                take("", 2, 0),
                take("khelgar", 4, 1),
                (
                    ITEM_TYPE.to_string(),
                    "qe_sword".to_string(),
                    None,
                    json!(1)
                ),
            ]
        );

        // A module whose own `ga_give_item` walks the whole faction (SoZ).
        let mut soz_graph = graph.clone();
        soz_graph
            .modules
            .push(crate::services::save_graph::AggregatedModule {
                name: "not_visited".into(),
                display_name: String::new(),
                resolved_path: String::new(),
                resolution_kind: crate::services::toolset_bridge::ResolutionKind::Unresolved,
                is_current: false,
                journal_category_tags: Vec::new(),
                speaker_names: Vec::new(),
                party_item_gifts: true,
            });
        let soz = preview(&RewardTables::from_graph(&soz_graph, None));
        assert_eq!(
            writes(&soz)[3],
            (
                COMPANION_ITEM_TYPE.to_string(),
                "khelgar/qe_sword".to_string(),
                None,
                json!(1)
            )
        );
        let mut edits = QuestEdits::default();
        let all: Vec<usize> = (0..soz.writes.len()).collect();
        edits.stage(soz, &graph, &all, false).unwrap();
        handler.begin_batch();
        let written = edits
            .flush(&mut handler, &NWN2Paths::new(), 0, false, &game_data())
            .unwrap();
        handler.commit_batch().unwrap();

        let tags = |fields: &indexmap::IndexMap<String, GffValue<'static>>| {
            backpack_items(fields)
                .map(|item| {
                    (
                        crate::parsers::gff::gff_text(item, "Tag").unwrap_or_default(),
                        item.get("StackSize").and_then(gff_integer).unwrap_or(1),
                    )
                })
                .collect::<Vec<_>>()
        };
        let ros = |name: &str| {
            let stored = handler.companion_stored_name(name).unwrap();
            owned_root(&handler.extract_file(&stored).unwrap()).unwrap()
        };
        assert_eq!(
            tags(&ros("khelgar")),
            vec![("qe_head".to_string(), 1), ("qe_sword".to_string(), 1)]
        );
        assert_eq!(tags(&ros("neeshka")), vec![("qe_head".to_string(), 4)]);
        let player = player_struct(&handler.extract_player_data().unwrap(), 0).unwrap();
        assert_eq!(tags(&player), vec![("qe_sword".to_string(), 1)]);
        assert_eq!(written.companions.len(), 1);
        assert_eq!(written.companions[0].0, "khelgar");
        assert_eq!(written.companions[0].1[0].0, "ItemList");
    }

    /// Gold from two quests adds up on save, so staging the second never asks to
    /// replace the first.
    #[test]
    fn rewards_of_two_quests_do_not_conflict() {
        let (_dir, handler) = rich_fixture_handler(100, 0);
        let mut graph = graph();
        add_quest(
            &mut graph,
            "one",
            vec![action("ga_give_gold", json!([100]))],
        );
        add_quest(
            &mut graph,
            "two",
            vec![action("ga_give_gold", json!([100]))],
        );
        let tables = RewardTables::from_graph(&graph, None);
        let mut edits = QuestEdits::default();
        for tag in ["one", "two"] {
            let preview = edits
                .preview(
                    &graph,
                    &tables,
                    &handler,
                    &NWN2Paths::new(),
                    0,
                    LiveCharacters::default(),
                    tag,
                    20,
                    Some(0),
                )
                .unwrap();
            assert!(
                edits
                    .stage(preview, &graph, &[0], false)
                    .unwrap()
                    .is_empty()
            );
        }
        assert_eq!(edits.pending.len(), 2);
        assert_eq!(edits.pending[1].writes[0].after, json!(300));
    }

    /// `ginc_var_ops`: the operand goes through `StringToInt`; NWScript ints wrap.
    #[test]
    fn relative_assignments_read_like_string_to_int_and_wrap() {
        let (_, change) =
            decode_assignment(&action("ga_global_int", json!(["x", "+3x"])), "m").unwrap();
        assert_eq!(change, Change::Add(3));
        assert_eq!(
            Change::Add(1).apply(Some(&json!(i32::MAX))),
            json!(i32::MIN)
        );
    }

    /// `ginc_param_const.GetTarget` upper-cases its `$` targets; in single player the
    /// PC, the party leader and the nearest PC are all the player.
    #[test]
    fn player_and_module_targets_match_get_target() {
        for target in ["$pc", "$PC_LEADER", "$PC_Nearest", " $PC_SPEAKER "] {
            let (key, _) =
                decode_assignment(&action("ga_give_feat", json!([target, 1705, 0])), "m").unwrap();
            assert_eq!(key.value_type, FEAT_TYPE, "{target}");
        }
        let (key, _) =
            decode_assignment(&action("ga_local_int", json!(["x", "1", "$module"])), "m").unwrap();
        assert_eq!(key.module.as_deref(), Some("m"));
    }

    #[test]
    fn huge_item_quantities_are_refused() {
        assert_eq!(
            decode_action(
                &action("ga_give_item", json!(["qe_sword", 2_000_000_000, 0])),
                "m",
                &RewardTables::empty()
            ),
            Err("invalid_assignment")
        );
    }

    /// The engine writes `Experience` as a DWORD.
    #[test]
    fn a_missing_experience_field_is_created_as_dword() {
        let mut fields = indexmap::IndexMap::new();
        assert!(matches!(
            add_to_field(&mut fields, EXPERIENCE, 250).unwrap(),
            GffValue::Dword(250)
        ));
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
                LiveCharacters::default(),
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
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
            .unwrap();
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
    fn rewards_apply_to_the_player_as_saved_in_the_same_batch() {
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
                LiveCharacters::default(),
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
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
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
                LiveCharacters::default(),
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.writes[0].after, json!(0));
        edits.stage(preview, &graph, &[0], false).unwrap();
        handler.begin_batch();
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
            .unwrap();
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
                LiveCharacters::default(),
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
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
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
        let item_list = &written
            .iter()
            .find(|(name, _)| name == "ItemList")
            .unwrap()
            .1;
        assert_eq!(swords(item_list), 3);
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
            assert_eq!(change, Change::AdjustAlignment(delta));
        }
        let (_, change) = decode_assignment(&action("ga_alignment", json!([4, 0])), "m").unwrap();
        assert_eq!(change.apply(Some(&json!(50))), json!(85));
    }

    /// Shifts within a band are exact; one that lands in another band snaps to its middle.
    #[test]
    fn alignment_shifts_snap_to_the_middle_of_a_new_band() {
        for (before, delta, after) in [
            (69, 1, 85),
            (71, -1, 70),
            (71, -2, 50),
            (71, -41, 15),
            (30, 1, 50),
            (31, -1, 15),
            (98, 5, 100),
            (10, -12, 0),
        ] {
            assert_eq!(
                Change::AdjustAlignment(delta).apply(Some(&json!(before))),
                json!(after),
                "{before} {delta:+}"
            );
        }
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

    /// A `SaveState` of fixed values, without backpacks or party.
    struct Values<F>(F);

    impl<F: FnMut(&VariableKey) -> Option<i64>> SaveState for Values<F> {
        fn current(&mut self, key: &VariableKey) -> Option<i64> {
            (self.0)(key)
        }

        fn stacks(&mut self, _: Option<&str>, _: &str) -> Vec<i64> {
            Vec::new()
        }

        fn party(&mut self) -> Vec<String> {
            Vec::new()
        }
    }

    /// nwscript `AdjustAlignment` out of a wrapper script: axis constants, both axes,
    /// and the neutral shift that stops at 50.
    #[test]
    fn adjust_alignment_points_shift_the_axes() {
        let tables = RewardTables::empty();
        let mut current = Values(|key: &VariableKey| match key.name.as_str() {
            "LawfulChaotic" => Some(10),
            "GoodEvil" => Some(85),
            _ => None,
        });
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
        assert_eq!(writes[1].1.apply(Some(&json!(85))), json!(70));
        let mut nearly_neutral = Values(|key: &VariableKey| match key.name.as_str() {
            "LawfulChaotic" => Some(40),
            "GoodEvil" => Some(50),
            _ => None,
        });
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
        let write = |value_type: &str, name: &str, change: Change| VariableWrite {
            key: VariableKey {
                module: None,
                name: name.into(),
                value_type: value_type.into(),
            },
            saved: None,
            before: None,
            after: change.apply(None),
            change,
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
            "FeatList".into() => GffValue::ListOwned(vec![
                indexmap! { "Feat".into() => GffValue::Word(10), "Uses".into() => GffValue::Byte(2) },
                indexmap! { "Feat".into() => GffValue::Word(11) },
            ]),
            "LvlStatList".into() => GffValue::ListOwned(vec![
                indexmap! { "FeatList".into() => GffValue::ListOwned(vec![
                    indexmap! { "Feat".into() => GffValue::Word(10) },
                ]) },
                indexmap! { "FeatList".into() => GffValue::ListOwned(Vec::new()) },
            ]),
            "ItemList".into() => GffValue::ListOwned(vec![item("qe_head", 4), item("other", 1)]),
        })
        .unwrap();
        let original = character.clone_gff();
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
                write(
                    CHARACTER_TYPE,
                    "Gold",
                    Change::AddClamped {
                        delta: 50,
                        min: 0,
                        max: i64::from(u32::MAX),
                    },
                ),
                // Previewed against 50, the tab moved it to 80: +45 lands on 100.
                write(CHARACTER_TYPE, "GoodEvil", Change::AdjustAlignment(45)),
                write(FEAT_TYPE, "20", Change::Set(Value::from(1))),
                write(FEAT_TYPE, "10", Change::Set(Value::from(0))),
                write(ITEM_TAKE_TYPE, "qe_head", take_units_change(3)),
                write(ITEM_TYPE, "reward", Change::Set(Value::from(2))),
                // Not the player's: left to `flush`.
                write("int", "SomeGlobal", Change::Add(1)),
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
        // Everything back in place: the split stack, the feat's entry with its uses and
        // its level, nothing given left behind.
        assert_eq!(format!("{:?}", character.gff()), format!("{original:?}"));
    }

    /// `ga_take_item` takes whole items (`TakeNumItems`), `ga_destroy_item` units; both
    /// match the tag exactly. A later take starts where the pending ones left off.
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
                    item("qe_head", Some(5)),
                ]),
            );
        });
        let mut graph = graph();
        add_quest(
            &mut graph,
            "take",
            vec![
                action("ga_take_item", json!(["qe_head", 1, 0])),
                action("ga_take_item", json!(["QE_Head", 0, 0])),
                action("ga_destroy_item", json!(["nothing_here", -1, 0])),
                action("ga_destroy_item", json!(["other", 0, 0])),
            ],
        );
        add_quest(
            &mut graph,
            "take_more",
            vec![action("ga_take_item", json!(["qe_head", 1, 0]))],
        );
        add_quest(
            &mut graph,
            "destroy_all",
            vec![action("ga_destroy_item", json!(["other", -1, 1]))],
        );
        let mut edits = QuestEdits::default();
        let tables = RewardTables::from_graph(&graph, None);
        let preview = |edits: &QuestEdits, tag: &str| {
            edits
                .preview(
                    &graph,
                    &tables,
                    &handler,
                    &NWN2Paths::new(),
                    0,
                    LiveCharacters::default(),
                    tag,
                    20,
                    Some(0),
                )
                .unwrap()
        };
        let take = preview(&edits, "take");
        assert!(take.effects.is_empty(), "{:?}", take.effects);
        let units = |p: &QuestPreview| {
            p.writes
                .iter()
                .map(|w| (w.key.name.clone(), w.before.clone(), w.after.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            units(&take),
            vec![
                ("qe_head".to_string(), Some(json!(11)), json!(7)),
                ("QE_Head".to_string(), Some(json!(1)), json!(0)),
            ]
        );
        assert_eq!(take.writes[0].key.value_type, ITEM_TAKE_TYPE);
        assert_eq!(take.writes[0].label.as_deref(), Some("Bandit Head"));
        assert_eq!(take.writes[0].label_strref, Some(4242));
        edits.stage(take, &graph, &[0, 1], false).unwrap();
        let take_more = preview(&edits, "take_more");
        assert_eq!(
            units(&take_more),
            vec![("qe_head".to_string(), Some(json!(7)), json!(5))]
        );
        edits.stage(take_more, &graph, &[0], false).unwrap();
        let destroy_all = preview(&edits, "destroy_all");
        assert_eq!(
            units(&destroy_all),
            vec![("other".to_string(), Some(json!(3)), json!(0))]
        );
        edits.stage(destroy_all, &graph, &[0], false).unwrap();
        edits
            .validate_saved(&handler, &NWN2Paths::new(), 0)
            .unwrap();

        handler.begin_batch();
        let written = edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
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
        let expected = vec![(
            "qe_head".to_string(),
            format!("{:?}", Some(GffValue::Word(5))),
        )];
        let item_list = &written
            .iter()
            .find(|(name, _)| name == "ItemList")
            .unwrap()
            .1;
        assert_eq!(stacks(item_list), expected);
        let player = owned_players(&handler.extract_player_data().unwrap()).swap_remove(0);
        assert_eq!(stacks(&player["ItemList"]), expected);
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        assert_eq!(stacks(&bic["ItemList"]), expected);
        let journal = read_live_journal(&handler, 0).unwrap();
        assert!(
            ["take", "take_more", "destroy_all"]
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
                LiveCharacters::default(),
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
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
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
                LiveCharacters::default(),
                "test",
                20,
                Some(0),
            )
            .unwrap();
        assert_eq!(preview.writes[0].before, Some(json!(1)));
        assert_eq!(preview.writes[0].after, json!(0));
        edits.stage(preview, &removal, &[0], false).unwrap();
        handler.begin_batch();
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
            .unwrap();
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
                LiveCharacters::default(),
                "test",
                20,
                Some(0),
            )
            .unwrap();
        let write = &preview.writes[0];
        let saved = write.saved.as_ref().and_then(Value::as_i64).unwrap_or(0);
        let after = Change::AdjustAlignment(10).apply(Some(&json!(saved)));
        assert_eq!(write.after, after);
        let after = after.as_i64().unwrap();
        let bic_before = owned_root(&handler.extract_player_bic().unwrap().unwrap())
            .unwrap()
            .get("LawfulChaotic")
            .and_then(gff_integer)
            .unwrap_or(0);
        edits.stage(preview, &graph, &[0], false).unwrap();
        handler.begin_batch();
        let written = edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
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
        assert_eq!(gff_integer(&player["LawfulChaotic"]).unwrap(), after);
        let bic = owned_root(&handler.extract_player_bic().unwrap().unwrap()).unwrap();
        assert_eq!(
            gff_integer(&bic["LawfulChaotic"]).unwrap(),
            (bic_before + after - saved).clamp(0, 100)
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
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
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
            .unwrap();
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
                LiveCharacters::default(),
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
        let GffValue::ListOwned(players) = root
            .swap_remove("Mod_PlayerList")
            .unwrap()
            .force_owned()
            .unwrap()
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
        let handler = SaveGameHandler::new(dir.path()).unwrap();
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
                lead_in: Vec::new(),
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
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
        assert_eq!(compare_check(3, "<4"), Some(true));
        assert_eq!(compare_check(3, "!3"), Some(false));
        // `CompareInts` reads one operator character and `StringToInt`s the rest, so
        // the two-character forms shipped dialogues use compare against 0.
        assert_eq!(compare_check(3, ">=4"), Some(true));
        assert_eq!(compare_check(0, ">=4"), Some(false));
        assert_eq!(compare_check(3, "<=2"), Some(false));
        assert_eq!(compare_check(3, "!=3"), Some(true));
        assert_eq!(compare_check(0, "=="), Some(true));
        assert_eq!(compare_check(3, "abc"), Some(false));
        assert_eq!(compare_check(0, "abc"), Some(true));

        let (dir, handler) = fixture_handler();
        std::fs::write(dir.path().join("globals.xml"), "<Globals><Integers><Integer><Name>a</Name><Value>1</Value></Integer></Integers></Globals>").unwrap();
        let paths = NWN2Paths::new();
        let mut reader = VariableReader::new(&handler, &paths, 0, LiveCharacters::default());
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
                LiveCharacters::default(),
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
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, false, &game_data())
            .unwrap();
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
                    LiveCharacters::default(),
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
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
                    LiveCharacters::default(),
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
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
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, false, &game_data())
            .unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("resgff.zip")).unwrap(),
            original
        );
        handler.abort_batch();
        assert_eq!(read_live_journal(&handler, 0).unwrap()[0].state, 10);
        handler.begin_batch();
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, false, &game_data())
            .unwrap();
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
                change: Change::Set(json!(target)),
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
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, false, &game_data())
            .unwrap();
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
        let handler = SaveGameHandler::new(path).unwrap();
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
            lead_in: Vec::new(),
            follow_up: Vec::new(),
            children: Vec::new(),
            ..base.clone()
        });
        quest.transitions.push(TransitionNode {
            new_state: 40,
            actions: vec![action("__connector_quest_tuple__", json!(["test", 40]))],
            gating_conditions: vec![condition(
                "gc_journal_entry",
                json!(["test", ">25"]),
                false,
                "",
            )],
            lead_in: Vec::new(),
            follow_up: Vec::new(),
            children: Vec::new(),
            ..base.clone()
        });
        quest.transitions.push(TransitionNode {
            new_state: 50,
            debug: true,
            actions: vec![action("__connector_quest_tuple__", json!(["test", 50]))],
            lead_in: Vec::new(),
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
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
                LiveCharacters::default(),
                "test",
                20,
            )
            .unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].effects.len(), 1);
        assert_eq!(routes[0].effects[0].script, "ga_destroy");
        assert!(routes[0].signature.contains("ga_destroy([\"door_a\",0])"));
    }

    fn preview_with_live(
        edits: &QuestEdits,
        graph: &SaveGraph,
        handler: &SaveGameHandler,
        live: LiveCharacters<'_>,
        tag: &str,
        route: Option<usize>,
    ) -> QuestPreview {
        let paths = NWN2Paths::new();
        edits
            .preview(
                graph,
                &RewardTables::from_graph(graph, None),
                handler,
                &paths,
                0,
                live,
                tag,
                20,
                route,
            )
            .unwrap()
    }

    fn write_of<'w>(preview: &'w QuestPreview, name: &str) -> &'w VariableWrite {
        preview
            .writes
            .iter()
            .find(|w| w.key.name == name)
            .unwrap_or_else(|| panic!("no write of {name}: {:?}", preview.writes))
    }

    /// The loaded player's unsaved values are what the scripts change: GoodEvil 57 on
    /// disk but 69 in the editor, +1 lands in the good band's middle like the engine.
    #[test]
    fn preview_and_staging_build_on_the_loaded_player() {
        let (_dir, handler) = fixture_handler_with(|player| {
            player.insert("GoodEvil".into(), GffValue::Byte(57));
            player.insert("Gold".into(), GffValue::Dword(300));
        });
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_alignment", json!([1, 0])),
                action("ga_take_gold", json!([500, 0])),
            ],
        );
        let mut loaded = owned_players(&handler.extract_player_data().unwrap()).swap_remove(0);
        loaded.insert("GoodEvil".into(), GffValue::Byte(69));
        loaded.insert("Gold".into(), GffValue::Dword(1000));
        let live = || LiveCharacters {
            player: Some(std::borrow::Cow::Borrowed(&loaded)),
            ..LiveCharacters::default()
        };
        let mut edits = QuestEdits::default();
        let preview = preview_with_live(&edits, &graph, &handler, live(), "test", Some(0));
        let alignment = write_of(&preview, "GoodEvil");
        assert_eq!(
            (
                alignment.saved.clone(),
                alignment.before.clone(),
                alignment.after.clone()
            ),
            (Some(json!(57)), Some(json!(69)), json!(85))
        );
        let gold = write_of(&preview, "Gold");
        assert_eq!(
            (gold.before.clone(), gold.after.clone()),
            (Some(json!(1000)), json!(500))
        );
        let revision = preview.revision.clone();
        edits.stage(preview, &graph, &[0, 1], false).unwrap();

        // A tab edit after the preview: staging applies the change to the value then.
        let mut player = Character::from_gff(loaded.clone()).unwrap();
        player
            .gff_mut()
            .insert("GoodEvil".into(), GffValue::Byte(30));
        apply_to_player(&edits.pending[0], &mut player, &game_data()).unwrap();
        let int = |name: &str| player.gff().get(name).and_then(gff_integer);
        assert_eq!((int("GoodEvil"), int("Gold")), (Some(50), Some(500)));

        // The revision follows the loaded characters, not just the save.
        let moved = LiveCharacters {
            revision: 1,
            ..live()
        };
        edits.discard(None);
        let again = preview_with_live(&edits, &graph, &handler, moved, "test", Some(0));
        assert_ne!(again.revision, revision);
    }

    /// Previewing a quest again replaces its pending edit, so it starts from the values
    /// before that edit; relative writes of different quests stack instead of conflicting.
    #[test]
    fn restaging_builds_on_the_values_before_the_replaced_edit() {
        let (dir, mut handler) = fixture_handler();
        std::fs::write(
            dir.path().join("globals.xml"),
            "<Globals><Integers><Integer><Name>counter</Name><Value>10</Value></Integer></Integers></Globals>",
        )
        .unwrap();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![action("ga_global_int", json!(["counter", "+3"]))],
        );
        add_quest(
            &mut graph,
            "other",
            vec![action("ga_global_int", json!(["counter", "+2"]))],
        );
        let mut edits = QuestEdits::default();
        let first = preview_with_live(
            &edits,
            &graph,
            &handler,
            LiveCharacters::default(),
            "test",
            Some(0),
        );
        assert_eq!(first.writes[0].after, json!(13));
        edits.stage(first, &graph, &[0], false).unwrap();
        let again = preview_with_live(
            &edits,
            &graph,
            &handler,
            LiveCharacters::default(),
            "test",
            Some(0),
        );
        assert_eq!(
            (
                again.writes[0].before.clone(),
                again.writes[0].after.clone()
            ),
            (Some(json!(10)), json!(13))
        );
        edits.stage(again, &graph, &[0], false).unwrap();
        let other = preview_with_live(
            &edits,
            &graph,
            &handler,
            LiveCharacters::default(),
            "other",
            Some(0),
        );
        assert_eq!(other.writes[0].after, json!(15));
        edits.stage(other, &graph, &[0], false).unwrap();
        assert_eq!(edits.pending.len(), 2);

        let mut globals = crate::parsers::xml::XmlData::default();
        globals.integers.insert("counter".into(), 10);
        edits.overlay_globals(&mut globals);
        assert_eq!(globals.integers["counter"], 15);

        // A tab edit of the global before saving: both changes land on top of it.
        std::fs::write(
            dir.path().join("globals.xml"),
            "<Globals><Integers><Integer><Name>counter</Name><Value>100</Value></Integer></Integers></Globals>",
        )
        .unwrap();
        handler.begin_batch();
        edits
            .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
            .unwrap();
        handler.commit_batch().unwrap();
        let saved = CampaignManager::get_campaign_variables(&handler).unwrap();
        assert_eq!(saved.integers["counter"], 105);
    }

    /// Journal entries apply in staging order: a quest's own entry is set, another
    /// quest's line only moves an entry up.
    #[test]
    fn journal_entries_of_several_edits_apply_in_staging_order() {
        let journal = |edits: &QuestEdits| {
            let (_dir, mut handler) = fixture_handler();
            handler.begin_batch();
            edits
                .flush(&mut handler, &NWN2Paths::new(), 0, true, &game_data())
                .unwrap();
            handler.commit_batch().unwrap();
            read_live_journal(&handler, 0)
                .unwrap()
                .into_iter()
                .map(|q| (q.tag, q.state))
                .collect::<BTreeMap<_, _>>()
        };
        let (_dir, handler) = fixture_handler();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "a",
            vec![
                action("__connector_quest_tuple__", json!(["a", 20])),
                action("__connector_quest_tuple__", json!(["b", 5])),
            ],
        );
        add_quest(&mut graph, "b", Vec::new());
        let stage = |edits: &mut QuestEdits, tag: &str, target: u32, route: Option<usize>| {
            let preview = edits
                .preview(
                    &graph,
                    &RewardTables::from_graph(&graph, None),
                    &handler,
                    &NWN2Paths::new(),
                    0,
                    LiveCharacters::default(),
                    tag,
                    target,
                    route,
                )
                .unwrap();
            let all: Vec<usize> = (0..preview.writes.len()).collect();
            edits.stage(preview, &graph, &all, false).unwrap();
        };

        // a advances b to 5, then b is set to 10 and to 3: the explicit entry wins.
        let mut edits = QuestEdits::default();
        stage(&mut edits, "a", 20, Some(0));
        stage(&mut edits, "b", 10, None);
        assert_eq!(journal(&edits)["b"], 10);
        stage(&mut edits, "b", 3, None);
        assert_eq!(journal(&edits)["b"], 3);

        // b set to 3 first, then a's line raises it to 5; set to 10 first, a leaves it.
        let mut edits = QuestEdits::default();
        stage(&mut edits, "b", 3, None);
        stage(&mut edits, "a", 20, Some(0));
        assert_eq!(journal(&edits)["b"], 5);
        let mut edits = QuestEdits::default();
        stage(&mut edits, "b", 10, None);
        stage(&mut edits, "a", 20, Some(0));
        assert_eq!(journal(&edits)["b"], 10);
    }

    /// Conditions further down the conversation see what the lines before them wrote,
    /// the lead-in lines included; lead-in writes are staged with their line.
    #[test]
    fn played_lines_see_earlier_writes_and_lead_in_lines_are_staged() {
        let (dir, handler) = fixture_handler();
        std::fs::write(dir.path().join("globals.xml"), "<Globals />").unwrap();
        let mut graph = graph();
        cormick_like(&mut graph);
        let tr = &mut graph.quests.last_mut().unwrap().transitions[0];
        tr.lead_in = vec![follow(
            5,
            false,
            true,
            Vec::new(),
            vec![
                // Cormick's opening line closes the intro: `10a_faulk_state(1)`.
                action("ga_global_int", json!(["FaulkState", "1"])),
                // An earlier entry of the same quest, not reached yet, on the way is moot.
                action("__connector_quest_tuple__", json!(["test", 15])),
            ],
            vec![1],
        )];
        tr.actions
            .push(action("ga_global_int", json!(["friendly", "1"])));
        let edits = QuestEdits::default();
        let preview = preview_with_live(
            &edits,
            &graph,
            &handler,
            LiveCharacters::default(),
            "test",
            Some(0),
        );
        assert_eq!(
            preview.lead_in.iter().map(|l| l.node).collect::<Vec<_>>(),
            vec![5]
        );
        assert_eq!(write_of(&preview, "FaulkState").line, Some(5));
        assert_eq!(write_of(&preview, "friendly").line, None);
        // `friendly` is set by the chosen line, so the walk takes the gated line 10.
        assert_eq!(preview.followed[0].node, 10);
        assert!(
            preview.effects.iter().all(|e| e.line != Some(5)),
            "{:?}",
            preview.effects
        );
    }

    /// MotB `d05_coven`: the opening line sets `00_nStoryStep` 5 and later lead-in lines
    /// add journal entries; a save that already has those entries played them.
    #[test]
    fn lead_in_lines_the_save_already_played_are_skipped() {
        let line = |node, actions, next| follow(node, false, true, Vec::new(), actions, vec![next]);
        let entry = |quest: &str, state: u32| {
            vec![action("__connector_quest_tuple__", json!([quest, state]))]
        };
        let lead_in = vec![
            line(
                8625,
                vec![action("ga_global_int", json!(["00_nStoryStep", "5"]))],
                8405,
            ),
            line(8405, entry("q90_passage_to_thay", 100), 8621),
            line(8621, entry("q00_dark_hunger", 400), 8551),
            line(8551, Vec::new(), 8201),
        ];
        let unplayed = |entries: &[(&str, u32)]| {
            unplayed_lead_in(&lead_in, |quest, state| {
                entries
                    .iter()
                    .any(|(tag, saved)| *tag == quest && *saved >= state)
            })
        };
        assert_eq!(unplayed(&[]), 0);
        assert_eq!(unplayed(&[("q90_passage_to_thay", 100)]), 2);
        assert_eq!(
            unplayed(&[("q90_passage_to_thay", 900), ("q00_dark_hunger", 600)]),
            3
        );
        assert_eq!(unplayed(&[("q00_dark_hunger", 300)]), 0);
    }

    /// A reply the graph kept without its children ends the walk as `depth`, not `end`.
    #[test]
    fn walk_stops_at_lines_the_graph_did_not_collect() {
        let (_dir, handler) = fixture_handler();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![action("__connector_quest_tuple__", json!(["test", 20]))],
        );
        let tr = &mut graph.quests.last_mut().unwrap().transitions[0];
        tr.is_player = true;
        tr.children = vec![10];
        // Replies of a line offering several are collected without their own lines.
        tr.follow_up = vec![
            follow(10, false, true, Vec::new(), Vec::new(), vec![20, 21]),
            follow(
                20,
                true,
                true,
                vec![condition(
                    "gc_journal_entry",
                    json!(["test", "5"]),
                    false,
                    "",
                )],
                Vec::new(),
                vec![30],
            ),
            follow(21, true, true, Vec::new(), Vec::new(), vec![31]),
        ];
        let edits = QuestEdits::default();
        let preview = preview_with_live(
            &edits,
            &graph,
            &handler,
            LiveCharacters::default(),
            "test",
            Some(0),
        );
        assert_eq!(
            preview.followed.iter().map(|l| l.node).collect::<Vec<_>>(),
            vec![10, 21]
        );
        let stop = preview.follow_stop.unwrap();
        assert_eq!((stop.reason.as_str(), stop.node), ("depth", Some(31)));
    }

    /// Scripts that do nothing in-game are left out, not listed as "not applied".
    #[test]
    fn no_op_lines_are_not_listed() {
        let (_dir, handler) = fixture_handler();
        let mut graph = graph();
        add_quest(
            &mut graph,
            "test",
            vec![
                action("ga_influence", json!([1, 0])),
                action("ka_influence_magnitude", json!(["safiya", 7])),
                action("ga_door_close", json!(["door", 0])),
                action("ga_give_gold", json!([0, 0])),
                // The player has no such feat.
                action("ga_remove_feat", json!(["$PC_SPEAKER", "9999"])),
            ],
        );
        let edits = QuestEdits::default();
        let preview = preview_with_live(
            &edits,
            &graph,
            &handler,
            LiveCharacters::default(),
            "test",
            Some(0),
        );
        assert!(preview.effects.is_empty(), "{:?}", preview.effects);
        assert!(preview.writes.is_empty(), "{:?}", preview.writes);
    }

    /// A decoded wrapper's `SetGlobalInt(x, -1)` sets -1; a dialogue's own `"=-1"` stays
    /// `ginc_var_ops`'s surprise.
    #[test]
    fn decoded_negative_values_are_set() {
        let mut decoded = action("ga_global_int", json!(["x", "=-1"]));
        assert_eq!(
            decode_assignment(&decoded, "m"),
            Err("unsupported_expression")
        );
        decoded.derived_from = Some("wrapper(1)".into());
        assert_eq!(
            decode_assignment(&decoded, "m").map(|(_, change)| change),
            Ok(Change::Set(json!(-1)))
        );
        let relative = ConvoFunctor {
            derived_from: Some("wrapper(1)".into()),
            ..action("ga_global_int", json!(["x", "-1"]))
        };
        assert_eq!(
            decode_assignment(&relative, "m").map(|(_, change)| change),
            Ok(Change::Add(-1))
        );
    }

    /// Gives and takes apply in staging order, like staging does in memory: a take-all
    /// before a hand-out of the same tag leaves the new items.
    #[test]
    fn item_changes_apply_in_staging_order() {
        let item = |stack: u16| -> GffStruct {
            indexmap! {
                "BaseItem".into() => GffValue::Int(12),
                "Tag".into() => GffValue::String("reward".into()),
                "StackSize".into() => GffValue::Word(stack),
            }
        };
        let blueprint = Arc::new(ItemBlueprint {
            fields: item(1),
            name: None,
            name_strref: None,
        });
        let fields = || indexmap! { "ItemList".into() => GffValue::ListOwned(vec![item(1)]) };
        let units = |ops: &[ItemOp]| {
            backpack_units(
                &apply_item_ops(fields(), ops, &game_data()).unwrap(),
                "reward",
            )
        };
        let take_all = || ItemOp::Take("reward".into(), Change::Set(Value::from(0)));
        let give = || ItemOp::Give(blueprint.clone(), 2);
        assert_eq!(units(&[take_all(), give()]), 2);
        assert_eq!(units(&[give(), take_all()]), 0);
    }
}
