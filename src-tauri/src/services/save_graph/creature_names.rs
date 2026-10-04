//! Display names for dialogue speaker tags, read from the creature blueprints
//! (`.utc`) and area instances (`.git`) shipped inside a module. Blueprints win;
//! area instances only fill tags no blueprint declares. Names are shipped with
//! their strrefs because the aggregator has no TLK; the UI resolves them.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use tracing::debug;

use crate::parsers::erf::{ErfParser, SecurityLimits};
use crate::parsers::gff::{GffParser, GffValue};

const RES_TYPE_UTC: u16 = 2027;
const RES_TYPE_GIT: u16 = 2023;

#[derive(Debug, Clone, Serialize)]
pub struct SpeakerName {
    pub tag: String,
    pub first: String,
    pub last: String,
    pub first_strref: Option<i32>,
    pub last_strref: Option<i32>,
    /// Lowercase `.dlg` resref this creature owns, empty when it has none. Lets
    /// the UI name the conversation owner, whose lines carry an empty speaker tag.
    pub conversation: String,
}

pub fn read_speaker_names(module_path: &Path) -> Vec<SpeakerName> {
    let mut by_tag: HashMap<String, SpeakerName> = HashMap::new();
    if module_path.is_dir() {
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(module_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let lower = name.to_ascii_lowercase();
                if (lower.ends_with(".utc") || lower.ends_with(".git"))
                    && let Ok(bytes) = std::fs::read(&path)
                {
                    files.push((lower, bytes));
                }
            }
        }
        for pass in [".utc", ".git"] {
            for (name, bytes) in files.iter().filter(|(n, _)| n.ends_with(pass)) {
                index_resource(&mut by_tag, name, bytes.clone());
            }
        }
    } else if module_path.is_file() {
        let mut parser = ErfParser::new().with_limits(SecurityLimits {
            max_file_size: 4 * 1024 * 1024 * 1024,
            ..SecurityLimits::default()
        });
        if let Err(e) = parser.read(module_path) {
            debug!(
                "speaker names unavailable for {}: {e}",
                module_path.display()
            );
            return Vec::new();
        }
        for res_type in [RES_TYPE_UTC, RES_TYPE_GIT] {
            for (name, _, _) in parser.list_resources(Some(res_type)) {
                if let Ok(bytes) = parser.extract_resource(&name) {
                    index_resource(&mut by_tag, &name, bytes);
                }
            }
        }
    }
    let mut names: Vec<SpeakerName> = by_tag.into_values().collect();
    names.sort_by(|a, b| a.tag.cmp(&b.tag));
    names
}

fn index_resource(by_tag: &mut HashMap<String, SpeakerName>, name: &str, bytes: Vec<u8>) {
    let Ok(gff) = GffParser::from_bytes(bytes) else {
        return;
    };
    let Ok(root) = gff.read_struct_fields(0) else {
        return;
    };
    if let Some(stem) = name.strip_suffix(".utc") {
        if let Some(entry) = speaker_from_fields(&root, Some(stem)) {
            by_tag.insert(entry.tag.clone(), entry);
        }
        return;
    }
    let Some(GffValue::List(creatures)) = root.get("Creature List") else {
        return;
    };
    for creature in creatures {
        let fields = match creature.force_load() {
            Ok(fields) => fields,
            Err(e) => {
                tracing::warn!("Skipping unreadable creature entry: {e}");
                continue;
            }
        };
        if let Some(entry) = speaker_from_fields(&fields, None)
            && !by_tag.contains_key(&entry.tag)
        {
            by_tag.insert(entry.tag.clone(), entry);
        }
    }
}

fn speaker_from_fields(
    fields: &indexmap::IndexMap<String, GffValue<'_>>,
    resref_fallback: Option<&str>,
) -> Option<SpeakerName> {
    let tag = match fields.get("Tag") {
        Some(GffValue::String(s)) if !s.trim().is_empty() => s.trim().to_ascii_lowercase(),
        _ => resref_fallback?.to_ascii_lowercase(),
    };
    let (first, first_strref) = loc_string(fields, "FirstName");
    let (last, last_strref) = loc_string(fields, "LastName");
    if first.is_empty() && last.is_empty() && first_strref.is_none() && last_strref.is_none() {
        return None;
    }
    let conversation = match fields.get("Conversation") {
        Some(GffValue::ResRef(r) | GffValue::String(r)) => r.trim().to_ascii_lowercase(),
        _ => String::new(),
    };
    Some(SpeakerName {
        tag,
        first,
        last,
        first_strref,
        last_strref,
        conversation,
    })
}

fn loc_string(
    fields: &indexmap::IndexMap<String, GffValue<'_>>,
    key: &str,
) -> (String, Option<i32>) {
    match fields.get(key) {
        Some(GffValue::LocString(ls)) => {
            let text = ls
                .substrings
                .iter()
                .map(|s| s.string.trim())
                .find(|s| !s.is_empty())
                .unwrap_or_default()
                .to_string();
            let strref = (ls.string_ref >= 0).then_some(ls.string_ref);
            (text, strref)
        }
        Some(GffValue::String(s)) => (s.trim().to_string(), None),
        _ => (String::new(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OC_A1: &str = "C:/Program Files (x86)/Steam/steamapps/common/NWN2 Enhanced Edition/modules/1000_neverwinter_a1.mod";

    #[test]
    fn oc_blacklake_speakers_resolve_to_creature_names() {
        if !Path::new(OC_A1).is_file() {
            return;
        }
        let names = read_speaker_names(Path::new(OC_A1));
        let sand = names.iter().find(|n| n.tag == "10_intro_sand").unwrap();
        assert_eq!(sand.first_strref, Some(141851));
        let caleb = names.iter().find(|n| n.tag == "caleb").unwrap();
        assert_eq!(caleb.conversation, "10_caleb");
        assert!(names.iter().any(|n| n.tag == "cormick"));
    }
}
