use super::types::{XmlData, GlobalsXml};
use quick_xml::de::from_str;
use regex::Regex;
use std::collections::{HashMap, HashSet, BTreeMap};
use chrono::{Utc, TimeZone};
use std::sync::OnceLock;
use serde::Serialize;
use quick_xml::Writer;
use std::io::Cursor;
use quick_xml::events::{BytesDecl, Event};

// Constants
const BLACKLIST: &[&str] = &["of", "the", "level", "count", "quest", "plot", "state"];

static INFLUENCE_PATTERN: OnceLock<Regex> = OnceLock::new();
static PREFIX_PATTERN: OnceLock<Regex> = OnceLock::new();

fn get_influence_pattern() -> &'static Regex {
    INFLUENCE_PATTERN.get_or_init(|| Regex::new(r"(?i)^(?:[a-zA-Z0-9_]*_)?(?:inf|influence)([a-z]+)$").unwrap())
}

fn get_prefix_pattern() -> &'static Regex {
    PREFIX_PATTERN.get_or_init(|| Regex::new(r"^([a-zA-Z0-9]+_)+|^([a-zA-Z]+)").unwrap())
}

// Quest patterns: (pattern, category, priority)
static QUEST_PATTERNS: OnceLock<Vec<(Regex, &'static str, i32)>> = OnceLock::new();

fn get_quest_patterns() -> &'static Vec<(Regex, &'static str, i32)> {
    QUEST_PATTERNS.get_or_init(|| vec![
        // 1. Exclusions
        (Regex::new(r"(?i)^_OG.*").unwrap(), "exclude", 1),
        (Regex::new(r"(?i)^__conv.*").unwrap(), "exclude", 1),
        (Regex::new(r"(?i)^WM_.*").unwrap(), "exclude", 1),
        (Regex::new(r"(?i).*(Num|Indx|Count|Fmn|Spc|Col|StN|FcN|LcN)$").unwrap(), "exclude", 1),
        (Regex::new(r"(?i).*NumKilled$").unwrap(), "exclude", 1),
        (Regex::new(r"(?i).*Influence.*").unwrap(), "exclude", 1),
        (Regex::new(r"(?i).*(rep|reputation)$").unwrap(), "exclude", 1),
        (Regex::new(r"(?i)^(MinimalDifficultyLevel|LastWriteTime|CAMPAIGN_SETUP_FLAG|N2_.*)$").unwrap(), "exclude", 1),
        
        // 2. Completion
        (Regex::new(r"(?i).*(Done|Over|Dead)$").unwrap(), "completed", 5),
        (Regex::new(r"(?i).*Complete(d)?$").unwrap(), "completed", 5),
        
        // 3. Active/State
        (Regex::new(r"(?i).*(State|Plot)$").unwrap(), "state", 10),
        (Regex::new(r"(?i).*(Quest|Mission|Intro|Go|Enabled|Visited|Met)$").unwrap(), "active", 11),
    ])
}

#[derive(Clone, Serialize)]
pub struct CompanionDefinition {
    pub name: &'static str,
    pub influence_var: &'static str,
    pub joined_var: &'static str,
    pub met_var: Option<&'static str>,
}

pub fn get_companion_definitions() -> HashMap<&'static str, CompanionDefinition> {
    let mut map = HashMap::new();
    map.insert("neeshka", CompanionDefinition { name: "Neeshka", influence_var: "00_nInfluenceneeshka", joined_var: "00_bNeeshka_Joined", met_var: None });
    map.insert("khelgar", CompanionDefinition { name: "Khelgar", influence_var: "00_nInfluencekhelgar", joined_var: "00_bKhelgar_Joined", met_var: None });
    map.insert("elanee", CompanionDefinition { name: "Elanee", influence_var: "00_nInfluenceelanee", joined_var: "00_bElanee_Joined", met_var: None });
    map.insert("qara", CompanionDefinition { name: "Qara", influence_var: "00_nInfluenceqara", joined_var: "00_bQaraJoined", met_var: None });
    map.insert("casavir", CompanionDefinition { name: "Casavir", influence_var: "00_nInfluencecasavir", joined_var: "00_bCasavir_Joined", met_var: None });
    map.insert("grobnar", CompanionDefinition { name: "Grobnar", influence_var: "00_nInfluencegrobnar", joined_var: "00_bGrobnar_Joined", met_var: None });
    map.insert("sand", CompanionDefinition { name: "Sand", influence_var: "00_nInfluencesand", joined_var: "00_bSand_Joined", met_var: Some("SandIntroDone") });
    map.insert("bishop", CompanionDefinition { name: "Bishop", influence_var: "00_nInfluencebishop", joined_var: "00_bBishop_Joined", met_var: None });
    map.insert("shandra", CompanionDefinition { name: "Shandra", influence_var: "00_nInfluenceshandra", joined_var: "00_bShandra_Joined", met_var: Some("bShandraMet") });
    map.insert("ammon_jerro", CompanionDefinition { name: "Ammon Jerro", influence_var: "00_nInfluenceammon", joined_var: "00_bAmmon_Joined", met_var: Some("bAmmonMet") });
    map.insert("zhjaeve", CompanionDefinition { name: "Zhjaeve", influence_var: "00_nInfluencezhjaeve", joined_var: "00_bZhjaeve_Joined", met_var: Some("bZhjaeveMet") });
    map.insert("construct", CompanionDefinition { name: "Construct", influence_var: "00_nInfluenceconstruct", joined_var: "00_bConstruct_Joined", met_var: Some("bConstructMet") });
    map.insert("safiya", CompanionDefinition { name: "Safiya", influence_var: "00_nInfluencesafiya", joined_var: "00_bSafiya_Joined", met_var: Some("bSafiyaMet") });
    map.insert("gann", CompanionDefinition { name: "Gann", influence_var: "00_nInfluencegann", joined_var: "00_bGann_Joined", met_var: Some("bGannMet") });
    map.insert("kaelyn", CompanionDefinition { name: "Kaelyn the Dove", influence_var: "00_nInfluencekaelyn", joined_var: "00_bKaelyn_Joined", met_var: Some("bKaelynMet") });
    map.insert("okku", CompanionDefinition { name: "Okku", influence_var: "00_nInfluenceokku", joined_var: "00_bOkku_Joined", met_var: Some("bOkkuMet") });
    map.insert("one_of_many", CompanionDefinition { name: "One of Many", influence_var: "00_nInfluenceoneofmany", joined_var: "00_bOneOfMany_Joined", met_var: Some("bOneOfManyMet") });
    map
}

pub struct RustXmlParser {
    pub data: XmlData,
}

impl RustXmlParser {
    pub fn new() -> Self {
        Self {
            data: XmlData::default(),
        }
    }

    pub fn from_string(content: &str) -> Result<Self, String> {
        let globals: GlobalsXml = from_str(content).map_err(|e| format!("Failed to parse XML: {}", e))?;
        Ok(Self {
            data: XmlData::from_xml_struct(globals),
        })
    }

    pub fn to_xml_string(&self) -> Result<String, String> {
        let xml_struct = self.data.to_xml_struct();
        
        // Manually write the declaration using custom approach or standard way to ensure it's clean
        // quick-xml's Writer can write the declaration.
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        writer.write_event(Event::Decl(BytesDecl::new("1.0", None, None))).map_err(|e| e.to_string())?;
        
        let mut result = String::from_utf8(writer.into_inner().into_inner()).map_err(|e| e.to_string())?;
        result.push('\n'); // Add newline after declaration
        
        let mut ser = quick_xml::se::Serializer::new(&mut result);
        ser.indent(' ', 2);
        xml_struct.serialize(ser).map_err(|e| e.to_string())?;
        
        Ok(result)
    }

    pub fn discover_potential_companions(&self) -> HashMap<String, BTreeMap<String, String>> {
        let mut discovered = HashMap::new();
        let blacklist_set: HashSet<&str> = BLACKLIST.iter().cloned().collect();
        let pattern = get_influence_pattern();

        for (var_name, value) in &self.data.integers {
            if let Some(caps) = pattern.captures(var_name) {
                if let Some(matched) = caps.get(1) {
                    let companion_name = matched.as_str();
                    if companion_name.len() > 2 && !blacklist_set.contains(companion_name) {
                        let lower_name = companion_name.to_lowercase();
                        if !discovered.contains_key(&lower_name) {
                            let mut info = BTreeMap::new();
                            info.insert("name".to_string(), capitalize(companion_name));
                            info.insert("influence".to_string(), value.to_string());
                            info.insert("recruitment".to_string(), "unknown".to_string());
                            info.insert("source".to_string(), "discovered".to_string());
                            discovered.insert(lower_name, info);
                        }
                    }
                }
            }
        }
        discovered
    }

    pub fn get_companion_status(&self) -> HashMap<String, CompanionStatus> {
        let mut companion_status = HashMap::new();
        let defs = get_companion_definitions();

        // 1. Explicit definitions
        for (comp_id, def) in defs {
            let mut influence = None;
            let mut recruitment = "not_recruited".to_string();

            if let Some(val) = self.data.integers.get(def.influence_var) {
                influence = Some(*val);
            }

            let joined = self.data.integers.get(def.joined_var).copied().unwrap_or(0);
            if joined > 0 {
                recruitment = "recruited".to_string();
            } else if let Some(met_var) = def.met_var {
                if self.data.integers.get(met_var).copied().unwrap_or(0) > 0 {
                    recruitment = "met".to_string();
                }
            }

            if influence.is_some() || recruitment != "not_recruited" {
                companion_status.insert(comp_id.to_string(), CompanionStatus {
                    name: def.name.to_string(),
                    influence,
                    recruitment,
                    source: "explicit".to_string(),
                });
            }
        }

        // 2. Discovered
        let discovered = self.discover_potential_companions();
        for (comp_id, status) in discovered {
            if !companion_status.contains_key(&comp_id) {
                // explicit definitions take precedence
                // discovered returns HashMap<String, BTreeMap<String, String>>
                // we need to convert BTreeMap to CompanionStatus
                let influence_val = status.get("influence").and_then(|s| s.parse::<i32>().ok());
                
                companion_status.insert(comp_id, CompanionStatus {
                    name: status.get("name").cloned().unwrap_or_default(),
                    influence: influence_val,
                    recruitment: status.get("recruitment").cloned().unwrap_or("unknown".to_string()),
                    source: "discovered".to_string(),
                });
            }
        }

        companion_status
    }

    fn identify_quest_vars(&self) -> (HashSet<String>, HashSet<String>) {
        let mut completed = HashSet::new();
        let mut active = HashSet::new();
        
        let sorted_patterns = get_quest_patterns();
        // Since sorted_patterns is now static and pre-sorted in declaration (or we can assume index order priority if careful),
        // but wait, we declared them in a specific order in `get_quest_patterns` but `Regex` doesn't implement Ord.
        // We used `sort_by_key(|k| k.2)` previously. We can sort them once or just iterate.
        // For OnceLock, it's immutable reference. We can't sort it in place unless we use Mutex or similar.
        // But we can just iterate. The priorities are small integers.
        // Better: sort them when initializing the OnceLock vec!
        // But `Regex` is not `Clone` easily? No, `Regex` is `Clone`.
        // Let's modify `get_quest_patterns` to return sorted vector or sort it there.
        // Or just `iterate` and filter.
        
        // Actually, sorting the static vec inside `get_or_init` is best.
        
        // Let's assume `get_quest_patterns` returns them pre-sorted by us manually in the code order above? 
        // We wrote them in priority groups, but the priority integer is key.
        // 1 (Exclusions) -> 5 (Completion) -> 10 (State) -> 11 (Active).
        // This is ALREADY sorted by priority (1, 1, ..., 5, 5, 10, 11).
        // So we can just iterate directly.

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
            quest_groups,
        }
    }
    
    pub fn get_general_info(&self) -> HashMap<String, Option<String>> {
        let mut info = HashMap::new();
        info.insert("player_name".to_string(), None);
        info.insert("game_act".to_string(), None);
        info.insert("last_saved".to_string(), None);

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

        if let Some(timestamp) = self.data.integers.get("LastWriteTime") {
            if let Some(dt) = Utc.timestamp_opt(*timestamp as i64, 0).single() {
                info.insert("last_saved".to_string(), Some(dt.to_rfc3339()));
            } else {
                info.insert("last_saved".to_string(), Some(format!("Invalid timestamp: {}", timestamp)));
            }
        }

        info
    }

    pub fn get_full_summary_struct(&self) -> FullSummary {
        FullSummary {
            general_info: self.get_general_info(),
            companion_status: self.get_companion_status(),
            quest_overview: self.get_quest_overview_struct(),
            raw_data_counts: HashMap::from([
                ("integers".to_string(), self.data.integers.len()),
                ("strings".to_string(), self.data.strings.len()),
                ("floats".to_string(), self.data.floats.len()),
                ("vectors".to_string(), self.data.vectors.len()),
            ]),
        }
    }
}

// Helper types for quest overview

#[derive(Serialize)]
pub struct CompanionStatus {
    pub name: String,
    pub influence: Option<i32>,
    pub recruitment: String,
    pub source: String,
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
    pub quest_groups: BTreeMap<String, QuestGroup>,
}

#[derive(Serialize)]
pub struct FullSummary {
    pub general_info: HashMap<String, Option<String>>,
    pub companion_status: HashMap<String, CompanionStatus>,
    pub quest_overview: QuestOverview,
    pub raw_data_counts: HashMap<String, usize>,
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
