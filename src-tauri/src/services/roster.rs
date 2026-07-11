//! Parse and update the party roster file (`roster.rst` / `ROSTER.rst`).

use indexmap::IndexMap;

use crate::parsers::gff::{GffParser, GffValue, GffWriter};

pub struct RosterMember {
    pub ros_name: String,
    pub char_name: String,
    /// `(class_id, level)` pairs from `RosClass0-3`/`RosLevel0-3` where level > 0.
    pub classes: Vec<(i32, i32)>,
    pub available: bool,
    pub campaign_npc: bool,
}

fn gff_int(value: &GffValue<'_>) -> Option<i64> {
    match value {
        GffValue::Byte(v) => Some(i64::from(*v)),
        GffValue::Word(v) => Some(i64::from(*v)),
        GffValue::Short(v) => Some(i64::from(*v)),
        GffValue::Dword(v) => Some(i64::from(*v)),
        GffValue::Int(v) => Some(i64::from(*v)),
        _ => None,
    }
}

fn gff_str(value: &GffValue<'_>) -> Option<String> {
    match value {
        GffValue::String(s) => Some(s.to_string()),
        GffValue::ResRef(s) => Some(s.to_string()),
        _ => None,
    }
}

pub fn parse_roster_members(bytes: Vec<u8>) -> Result<Vec<RosterMember>, String> {
    let parser = GffParser::from_bytes(bytes).map_err(|e| format!("roster parse error: {e}"))?;
    let root = parser
        .read_struct_fields(0)
        .map_err(|e| format!("roster root error: {e}"))?;

    let Some(members_value) = root.get("RosMembers") else {
        return Ok(Vec::new());
    };
    let GffValue::ListOwned(entries) = members_value.clone().force_owned() else {
        return Err("RosMembers is not a list".into());
    };

    let mut out = Vec::new();
    for entry in &entries {
        let Some(ros_name) = entry.get("RosName").and_then(gff_str) else {
            continue;
        };
        let char_name = entry
            .get("RosCharName")
            .and_then(gff_str)
            .unwrap_or_else(|| ros_name.clone());
        let mut classes = Vec::new();
        for i in 0..4 {
            let class_id = entry
                .get(format!("RosClass{i}").as_str())
                .and_then(gff_int)
                .unwrap_or(0);
            let level = entry
                .get(format!("RosLevel{i}").as_str())
                .and_then(gff_int)
                .unwrap_or(0);
            if level > 0 {
                classes.push((class_id as i32, level as i32));
            }
        }
        out.push(RosterMember {
            ros_name,
            char_name,
            classes,
            available: entry.get("RosAvailable").and_then(gff_int).unwrap_or(0) != 0,
            campaign_npc: entry.get("RosCampaignNPC").and_then(gff_int).unwrap_or(0) != 0,
        });
    }
    Ok(out)
}

/// Rewrite the cached class/level pairs for `ros_name`. `Ok(None)` means the
/// roster has no matching member; callers surface that as a warning, not an error.
pub fn sync_member_classes(
    bytes: Vec<u8>,
    ros_name: &str,
    classes: &[(i32, i32)],
) -> Result<Option<Vec<u8>>, String> {
    let parser = GffParser::from_bytes(bytes).map_err(|e| format!("roster parse error: {e}"))?;
    let file_type = parser.file_type.clone();
    let file_version = parser.file_version.clone();
    let root_struct_id = parser
        .get_struct_id(0)
        .map_err(|e| format!("roster root id error: {e}"))?;
    let mut root: IndexMap<String, GffValue<'static>> = parser
        .read_struct_fields(0)
        .map_err(|e| format!("roster root error: {e}"))?
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let Some(GffValue::ListOwned(members)) = root.get_mut("RosMembers") else {
        return Ok(None);
    };

    let mut found = false;
    for entry in members.iter_mut() {
        let matches = entry
            .get("RosName")
            .and_then(gff_str)
            .is_some_and(|n| n.eq_ignore_ascii_case(ros_name));
        if !matches {
            continue;
        }
        found = true;
        for i in 0..4 {
            let (class_id, level) = classes.get(i).copied().unwrap_or((0, 0));
            set_int_preserving_type(entry, &format!("RosClass{i}"), i64::from(class_id));
            set_int_preserving_type(entry, &format!("RosLevel{i}"), i64::from(level));
        }
        break;
    }
    if !found {
        return Ok(None);
    }

    GffWriter::new(&file_type, &file_version)
        .write_with_struct_id(root, root_struct_id)
        .map(Some)
        .map_err(|e| format!("roster serialization error: {e}"))
}

/// Overwrite `key` keeping the numeric GFF variant the file already uses so the
/// game reads it back with the type it wrote. Missing keys stay absent.
fn set_int_preserving_type(entry: &mut IndexMap<String, GffValue<'static>>, key: &str, value: i64) {
    let Some(existing) = entry.get_mut(key) else {
        return;
    };
    *existing = match &*existing {
        GffValue::Byte(_) => GffValue::Byte(value.clamp(0, i64::from(u8::MAX)) as u8),
        GffValue::Word(_) => GffValue::Word(value.clamp(0, i64::from(u16::MAX)) as u16),
        GffValue::Short(_) => {
            GffValue::Short(value.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16)
        }
        GffValue::Dword(_) => GffValue::Dword(value.clamp(0, i64::from(u32::MAX)) as u32),
        GffValue::Int(_) => {
            GffValue::Int(value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
        }
        other => other.clone(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::{GffValue, GffWriter};
    use indexmap::IndexMap;

    fn member(
        ros_name: &str,
        char_name: &str,
        class0: (u32, i32),
        available: u8,
        campaign_npc: u8,
    ) -> IndexMap<String, GffValue<'static>> {
        let mut m: IndexMap<String, GffValue<'static>> = IndexMap::new();
        m.insert(
            "RosName".into(),
            GffValue::String(ros_name.to_string().into()),
        );
        m.insert(
            "RosCharName".into(),
            GffValue::String(char_name.to_string().into()),
        );
        for i in 0..4u8 {
            let (class, level) = if i == 0 { class0 } else { (0, 0) };
            m.insert(format!("RosClass{i}"), GffValue::Dword(class));
            m.insert(format!("RosLevel{i}"), GffValue::Int(level));
        }
        m.insert("RosAvailable".into(), GffValue::Byte(available));
        m.insert("RosCampaignNPC".into(), GffValue::Byte(campaign_npc));
        m
    }

    fn build_roster(members: Vec<IndexMap<String, GffValue<'static>>>) -> Vec<u8> {
        let mut root: IndexMap<String, GffValue<'static>> = IndexMap::new();
        root.insert("RosPartyLimit".into(), GffValue::Byte(3));
        root.insert("RosMembers".into(), GffValue::ListOwned(members));
        GffWriter::new("RST ", "V3.2")
            .write(root)
            .expect("write roster")
    }

    #[test]
    fn parse_extracts_members() {
        let bytes = build_roster(vec![
            member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1),
            member("npc_bevil", "Bevil Starling", (4, 1), 0, 0),
        ]);
        let members = parse_roster_members(bytes).expect("parse");
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].ros_name, "khelgar");
        assert_eq!(members[0].char_name, "Khelgar Ironfist");
        assert_eq!(members[0].classes, vec![(4, 2)]);
        assert!(members[0].available);
        assert!(members[0].campaign_npc);
        assert!(!members[1].available);
        assert!(!members[1].campaign_npc);
    }

    #[test]
    fn sync_updates_classes_preserving_gff_types() {
        let bytes = build_roster(vec![member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1)]);
        let synced = sync_member_classes(bytes, "khelgar", &[(4, 9), (23, 1)])
            .expect("sync")
            .expect("member found");

        let members = parse_roster_members(synced.clone()).expect("reparse");
        assert_eq!(members[0].classes, vec![(4, 9), (23, 1)]);

        // Original numeric variants must survive the rewrite.
        let parser = crate::parsers::gff::GffParser::from_bytes(synced).expect("parse");
        let root = parser.read_struct_fields(0).expect("root");
        let GffValue::List(entries) = root.get("RosMembers").expect("RosMembers") else {
            panic!("RosMembers not a list");
        };
        let entry = entries.first().expect("entry").force_load();
        assert!(matches!(entry.get("RosClass0"), Some(GffValue::Dword(4))));
        assert!(matches!(entry.get("RosLevel0"), Some(GffValue::Int(9))));
        assert!(matches!(entry.get("RosClass1"), Some(GffValue::Dword(23))));
    }

    #[test]
    fn sync_unknown_member_returns_none() {
        let bytes = build_roster(vec![member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1)]);
        let result = sync_member_classes(bytes, "nonexistent", &[(1, 1)]).expect("sync");
        assert!(result.is_none());
    }
}
