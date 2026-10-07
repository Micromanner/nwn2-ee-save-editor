use super::types::{GlobalsXml, XmlData};
use chrono::{TimeZone, Utc};
use indexmap::IndexMap;
use quick_xml::de::from_str;
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

static PREFIX_PATTERN: OnceLock<Regex> = OnceLock::new();

fn get_prefix_pattern() -> &'static Regex {
    PREFIX_PATTERN.get_or_init(|| Regex::new(r"^([a-zA-Z0-9]+_)+|^([a-zA-Z]+)").unwrap())
}

// Quest patterns: (pattern, category, priority)
static QUEST_PATTERNS: OnceLock<Vec<(Regex, &'static str, i32)>> = OnceLock::new();

fn get_quest_patterns() -> &'static Vec<(Regex, &'static str, i32)> {
    QUEST_PATTERNS.get_or_init(|| {
        vec![
            // 1. Exclusions
            (Regex::new(r"(?i)^_OG.*").unwrap(), "exclude", 1),
            (Regex::new(r"(?i)^__conv.*").unwrap(), "exclude", 1),
            (Regex::new(r"(?i)^WM_.*").unwrap(), "exclude", 1),
            (
                Regex::new(r"(?i).*(Num|Indx|Count|Fmn|Spc|Col|StN|FcN|LcN)$").unwrap(),
                "exclude",
                1,
            ),
            (Regex::new(r"(?i).*NumKilled$").unwrap(), "exclude", 1),
            (Regex::new(r"(?i).*Influence.*").unwrap(), "exclude", 1),
            (
                Regex::new(r"(?i).*(rep|reputation)$").unwrap(),
                "exclude",
                1,
            ),
            (
                Regex::new(
                    r"(?i)^(MinimalDifficultyLevel|LastWriteTime|CAMPAIGN_SETUP_FLAG|N2_.*)$",
                )
                .unwrap(),
                "exclude",
                1,
            ),
            // 2. Completion
            (
                Regex::new(r"(?i).*(Done|Over|Dead)$").unwrap(),
                "completed",
                5,
            ),
            (Regex::new(r"(?i).*Complete(d)?$").unwrap(), "completed", 5),
            // 3. Active/State
            (Regex::new(r"(?i).*(State|Plot)$").unwrap(), "state", 10),
            (
                Regex::new(r"(?i).*(Quest|Mission|Intro|Go|Enabled|Visited|Met)$").unwrap(),
                "active",
                11,
            ),
        ]
    })
}

/// Official campaign a companion belongs to; used to list the whole cast of
/// the campaign a save is playing, not just the members the game has written
/// variables for yet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Campaign {
    Oc,
    MotB,
}

#[derive(Clone, Serialize)]
pub struct CompanionDefinition {
    pub campaign: Campaign,
    pub name: &'static str,
    pub influence_var: &'static str,
    /// Explicit roster `RosName` alias for companions whose comp_id doesn't
    /// normalize to a match (e.g. kaelyn's roster entry is "dove").
    pub ros_name: Option<&'static str>,
}

pub fn get_companion_definitions() -> IndexMap<&'static str, CompanionDefinition> {
    let mut map = IndexMap::new();
    map.insert(
        "neeshka",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Neeshka",
            influence_var: "00_nInfluenceneeshka",
            ros_name: None,
        },
    );
    map.insert(
        "khelgar",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Khelgar",
            influence_var: "00_nInfluencekhelgar",
            ros_name: None,
        },
    );
    map.insert(
        "elanee",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Elanee",
            influence_var: "00_nInfluenceelanee",
            ros_name: None,
        },
    );
    map.insert(
        "qara",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Qara",
            influence_var: "00_nInfluenceqara",
            ros_name: None,
        },
    );
    map.insert(
        "casavir",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Casavir",
            influence_var: "00_nInfluencecasavir",
            ros_name: None,
        },
    );
    map.insert(
        "grobnar",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Grobnar",
            influence_var: "00_nInfluencegrobnar",
            ros_name: None,
        },
    );
    map.insert(
        "sand",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Sand",
            influence_var: "00_nInfluencesand",
            ros_name: None,
        },
    );
    map.insert(
        "bishop",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Bishop",
            influence_var: "00_nInfluencebishop",
            ros_name: None,
        },
    );
    map.insert(
        "shandra",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Shandra",
            influence_var: "00_nInfluenceshandra",
            ros_name: None,
        },
    );
    map.insert(
        "ammon_jerro",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Ammon Jerro",
            influence_var: "00_nInfluenceammon_jerro",
            ros_name: None,
        },
    );
    map.insert(
        "zhjaeve",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Zhjaeve",
            influence_var: "00_nInfluencezhjaeve",
            ros_name: None,
        },
    );
    map.insert(
        "construct",
        CompanionDefinition {
            campaign: Campaign::Oc,
            name: "Construct",
            influence_var: "00_nInfluenceconstruct",
            ros_name: None,
        },
    );
    map.insert(
        "safiya",
        CompanionDefinition {
            campaign: Campaign::MotB,
            name: "Safiya",
            influence_var: "00_nInfluencesafiya",
            ros_name: None,
        },
    );
    map.insert(
        "gann",
        CompanionDefinition {
            campaign: Campaign::MotB,
            name: "Gann",
            influence_var: "00_nInfluencegann",
            ros_name: None,
        },
    );
    map.insert(
        "kaelyn",
        CompanionDefinition {
            campaign: Campaign::MotB,
            name: "Kaelyn the Dove",
            influence_var: "00_nInfluencedove",
            ros_name: Some("dove"),
        },
    );
    map.insert(
        "okku",
        CompanionDefinition {
            campaign: Campaign::MotB,
            name: "Okku",
            influence_var: "00_nInfluenceokku",
            ros_name: None,
        },
    );
    map.insert(
        "one_of_many",
        CompanionDefinition {
            campaign: Campaign::MotB,
            name: "One of Many",
            influence_var: "00_nInfluenceoneofmany",
            ros_name: Some("oneofmany"),
        },
    );
    map
}

/// `globals.xml` booleans the engine sets once a console cheat ran; they disable achievements.
const CHEAT_MARKERS: [&str; 2] = ["Cheater", "ShowCheatsWarning"];

pub struct RustXmlParser {
    pub data: XmlData,
}

impl Default for RustXmlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustXmlParser {
    pub fn new() -> Self {
        Self {
            data: XmlData::default(),
        }
    }

    pub fn from_string(content: &str) -> Result<Self, String> {
        let globals: GlobalsXml =
            from_str(content).map_err(|e| format!("Failed to parse XML: {e}"))?;
        Ok(Self {
            data: XmlData::from_xml_struct(globals),
        })
    }

    /// Whether the engine flagged this save as cheated (console commands under `DebugMode 1`).
    pub fn has_cheat_markers(&self) -> bool {
        CHEAT_MARKERS
            .iter()
            .any(|name| self.data.booleans.contains_key(*name))
    }

    pub fn to_xml_string(&self) -> Result<String, String> {
        let mut xml_struct = self.data.to_xml_struct();

        // Strip cheat markers on write so saves loaded in the editor come out clean.
        xml_struct
            .booleans
            .entries
            .retain(|entry| !CHEAT_MARKERS.contains(&entry.name.as_str()));

        let mut buffer = String::new();
        let mut ser = quick_xml::se::Serializer::new(&mut buffer);
        ser.indent(' ', 4);
        xml_struct.serialize(ser).map_err(|e| e.to_string())?;

        // NWN2 writes globals.xml with CRLF line endings and a trailing newline,
        // with no XML declaration. Match that byte-for-byte.
        let mut result = buffer.replace('\n', "\r\n");
        result.push_str("\r\n");

        Ok(result)
    }

    /// Status of every known companion of the campaign(s) this save is
    /// playing, plus the roster companions of other campaigns that store
    /// influence in globals.
    /// `roster` are the save's companions; a campaign counts as present
    /// when any of its companions has a `.ros` file or an influence variable.
    /// Recruitment is deliberately not reported: the OC writes `.ros` files for
    /// its whole cast at campaign start and skips the `*_Joined`/`*Met` globals
    /// for most companions.
    pub fn get_companion_status(
        &self,
        roster: &[RosterCompanion],
    ) -> IndexMap<String, CompanionStatus> {
        let mut companion_status = IndexMap::new();
        let defs = get_companion_definitions();
        let in_roster = |comp_id: &str, def: &CompanionDefinition| {
            let ros_name = def.ros_name.unwrap_or(comp_id);
            roster
                .iter()
                .any(|r| r.ros_name.eq_ignore_ascii_case(ros_name))
        };
        let campaigns: HashSet<Campaign> = defs
            .iter()
            .filter(|(comp_id, def)| {
                in_roster(comp_id, def) || self.data.integers.contains_key(def.influence_var)
            })
            .map(|(_, def)| def.campaign)
            .collect();

        // 1. Explicit definitions
        for (comp_id, def) in &defs {
            if !campaigns.contains(&def.campaign) {
                continue;
            }
            companion_status.insert(
                comp_id.to_string(),
                CompanionStatus {
                    name: def.name.to_string(),
                    influence: self.data.integers.get(def.influence_var).copied(),
                    influence_var: def.influence_var.to_string(),
                    source: "explicit".to_string(),
                },
            );
        }

        // 2. Other roster companions. Campaigns name the global prefix + companion
        // tag (`ginc_companion`: `00_nInfluence<tag>`, Westgate: `HF_COINF_<tag>`)
        // and the OC companion system makes the tag the roster name.
        let explicit_ros: HashSet<String> = defs
            .iter()
            .map(|(comp_id, def)| def.ros_name.unwrap_or(comp_id).to_ascii_lowercase())
            .collect();
        let mut prefixes = HashSet::new();
        for (var_name, value) in &self.data.integers {
            let Some(member) = roster_owner_of_influence_var(var_name, roster) else {
                continue;
            };
            let ros_key = member.ros_name.to_ascii_lowercase();
            if explicit_ros.contains(&ros_key) {
                continue;
            }
            prefixes.insert(&var_name[..var_name.len() - member.ros_name.len()]);
            companion_status
                .entry(ros_key)
                .or_insert_with(|| CompanionStatus {
                    name: member.name.clone(),
                    influence: Some(*value),
                    influence_var: var_name.clone(),
                    source: "roster".to_string(),
                });
        }

        // 3. Roster companions whose global is not written yet (the game reads it
        // as 0), named like the others when they all share one prefix. Defined
        // campaigns list their cast above, so their story NPCs stay out.
        if campaigns.is_empty()
            && prefixes.len() == 1
            && let Some(prefix) = prefixes.into_iter().next()
        {
            for member in roster {
                companion_status
                    .entry(member.ros_name.to_ascii_lowercase())
                    .or_insert_with(|| CompanionStatus {
                        name: member.name.clone(),
                        influence: None,
                        influence_var: format!("{prefix}{}", member.ros_name),
                        source: "roster".to_string(),
                    });
            }
        }

        companion_status
    }

    fn identify_quest_vars(&self) -> (HashSet<String>, HashSet<String>) {
        let mut completed = HashSet::new();
        let mut active = HashSet::new();

        let sorted_patterns = get_quest_patterns();

        for (var_name, &value) in &self.data.integers {
            if value <= 0 {
                continue;
            }

            for (pattern, category, _) in sorted_patterns {
                if pattern.is_match(var_name) {
                    match *category {
                        "exclude" => break,
                        "completed" => {
                            completed.insert(var_name.clone());
                            break;
                        }
                        "active" => {
                            active.insert(var_name.clone());
                            break;
                        }
                        "state" => {
                            if value >= 50 {
                                completed.insert(var_name.clone());
                            } else {
                                active.insert(var_name.clone());
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        let active: HashSet<String> = active.difference(&completed).cloned().collect();
        (completed, active)
    }

    pub fn get_quest_overview_struct(&self) -> QuestOverview {
        let (completed, active) = self.identify_quest_vars();
        let total = completed.len() + active.len();

        let mut quest_groups = BTreeMap::new();
        let all_quest_vars: HashSet<_> = completed.union(&active).collect();
        let prefix_pattern = get_prefix_pattern();

        for var in all_quest_vars {
            let group_key = if let Some(caps) = prefix_pattern.captures(var) {
                if let Some(g1) = caps.get(1) {
                    g1.as_str().trim_end_matches('_').to_string()
                } else if let Some(g2) = caps.get(2) {
                    g2.as_str().trim_end_matches('_').to_string()
                } else {
                    var.trim_end_matches('_').to_string()
                }
            } else {
                var.trim_end_matches('_').to_string()
            };

            let entry = quest_groups.entry(group_key).or_insert_with(|| QuestGroup {
                completed: Vec::new(),
                active: Vec::new(),
            });

            if completed.contains(var) {
                entry.completed.push(var.clone());
            } else {
                entry.active.push(var.clone());
            }
        }

        QuestOverview {
            completed_count: completed.len(),
            active_count: active.len(),
            total_quest_vars: total,
            completion_percentage: if total > 0 {
                (completed.len() as f32 / total as f32) * 100.0
            } else {
                0.0
            },
            quest_groups,
        }
    }

    pub fn get_general_info(&self) -> HashMap<String, Option<String>> {
        let mut info = HashMap::new();
        info.insert("player_name".to_string(), None);
        info.insert("game_act".to_string(), None);
        info.insert("last_saved".to_string(), None);
        info.insert("difficulty".to_string(), None);

        let name_vars = ["MainCharacter", "PlayerName"];
        for var in &name_vars {
            if let Some(val) = self.data.strings.get(*var) {
                info.insert("player_name".to_string(), Some(val.clone()));
                break;
            }
        }

        if let Some(val) = self.data.integers.get("00_nAct") {
            info.insert("game_act".to_string(), Some(val.to_string()));
        }

        if let Some(val) = self.data.integers.get("MinimalDifficultyLevel") {
            let label = match *val {
                0 => "Easy",
                1 => "Normal",
                2 => "Hard",
                3 => "Hardcore",
                _ => "Normal",
            };
            info.insert("difficulty".to_string(), Some(label.to_string()));
        }

        if let Some(timestamp) = self.data.integers.get("LastWriteTime") {
            if let Some(dt) = Utc.timestamp_opt(i64::from(*timestamp), 0).single() {
                info.insert("last_saved".to_string(), Some(dt.to_rfc3339()));
            } else {
                info.insert(
                    "last_saved".to_string(),
                    Some(format!("Invalid timestamp: {timestamp}")),
                );
            }
        }

        info
    }

    pub fn get_full_summary_struct(&self, roster: &[RosterCompanion]) -> FullSummary {
        FullSummary {
            general_info: self.get_general_info(),
            companion_status: self.get_companion_status(roster),
            quest_overview: self.get_quest_overview_struct(),
            raw_data_counts: HashMap::from([
                ("integers".to_string(), self.data.integers.len()),
                ("booleans".to_string(), self.data.booleans.len()),
                ("floats".to_string(), self.data.floats.len()),
                ("strings".to_string(), self.data.strings.len()),
            ]),
        }
    }
}

// Helper types for quest overview

#[derive(Serialize)]
pub struct CompanionStatus {
    pub name: String,
    pub influence: Option<i32>,
    /// The global the influence is stored in.
    pub influence_var: String,
    pub source: String,
}

/// A companion `.ros` file of the save.
pub struct RosterCompanion {
    pub ros_name: String,
    /// `RosCharName` from `roster.rst`.
    pub name: String,
}

/// The roster companion `var_name` holds the influence of: the global ends in
/// the companion's roster name and its prefix names influence ("inf"). The
/// longest roster name wins, so `kipp` never claims `0_kipp`'s global.
fn roster_owner_of_influence_var<'a>(
    var_name: &str,
    roster: &'a [RosterCompanion],
) -> Option<&'a RosterCompanion> {
    let var = var_name.as_bytes();
    roster
        .iter()
        .filter(|member| {
            let ros = member.ros_name.as_bytes();
            var.len() > ros.len() && {
                let (prefix, suffix) = var.split_at(var.len() - ros.len());
                suffix.eq_ignore_ascii_case(ros)
                    && prefix.windows(3).any(|w| w.eq_ignore_ascii_case(b"inf"))
            }
        })
        .max_by_key(|member| member.ros_name.len())
}

#[derive(Serialize)]
pub struct QuestGroup {
    pub completed: Vec<String>,
    pub active: Vec<String>,
}

#[derive(Serialize)]
pub struct QuestOverview {
    pub completed_count: usize,
    pub active_count: usize,
    pub total_quest_vars: usize,
    pub completion_percentage: f32,
    pub quest_groups: BTreeMap<String, QuestGroup>,
}

#[derive(Serialize)]
pub struct FullSummary {
    pub general_info: HashMap<String, Option<String>>,
    pub companion_status: IndexMap<String, CompanionStatus>,
    pub quest_overview: QuestOverview,
    pub raw_data_counts: HashMap<String, usize>,
}
