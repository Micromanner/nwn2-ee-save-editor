//! Parse and update the party roster file (`roster.rst` / `ROSTER.rst`).

use indexmap::IndexMap;

use crate::parsers::gff::{GffParser, GffValue, GffWriter};

pub struct RosterMember {
    pub ros_name: String,
    pub char_name: String,
    /// `(class_id, level)` pairs from `RosClass0-3`/`RosLevel0-3` where level > 0.
    pub classes: Vec<(i32, i32)>,
    /// `RosAvailable` is the engine's party-membership record: 0 = in the
    /// active party, 1 = benched/available for recruitment (in-game verified
    /// 2026-08-31; flipping it swaps party members on the next load).
    pub available: bool,
    pub campaign_npc: bool,
}

/// `FactionID` the engine writes on a benched companion's `.ros` ("Defender").
/// Party members carry the PC's dynamic party faction instead (read it from
/// `playerlist.ifo` `Mod_PlayerList[0].FactionID`; never hardcode it).
pub const BENCHED_FACTION: u16 = 4;

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
/// Fields the roster caches from a member's `.ros` and shows in the party-selection UI.
#[derive(Debug, Clone, Copy, Default)]
pub struct RosterIdentity {
    pub gender: i32,
    /// `RosRace` mirrors the creature's `Subrace` byte (a `racialsubtypes.2da` row), not `Race`.
    pub subrace: i32,
}

pub fn sync_member(
    bytes: Vec<u8>,
    ros_name: &str,
    classes: &[(i32, i32)],
    identity: RosterIdentity,
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
        set_int_preserving_type(entry, "RosGender", i64::from(identity.gender));
        set_int_preserving_type(entry, "RosRace", i64::from(identity.subrace));
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

/// `RosPartyLimit` from the roster root: how many companions the party screen
/// allows at once. `None` when the roster lacks the field.
pub fn parse_party_limit(bytes: Vec<u8>) -> Result<Option<i32>, String> {
    let parser = GffParser::from_bytes(bytes).map_err(|e| format!("roster parse error: {e}"))?;
    let root = parser
        .read_struct_fields(0)
        .map_err(|e| format!("roster root error: {e}"))?;
    Ok(root
        .get("RosPartyLimit")
        .and_then(gff_int)
        .map(|v| v as i32))
}

/// Set `RosAvailable` for each `(ros_name, in_party)` change (in party = 0,
/// benched = 1), preserving the file's numeric GFF variant and creating the
/// field as Int (the engine's type) on entries that lack it. Errors if any
/// named member is missing from the roster.
pub fn set_party_membership(bytes: Vec<u8>, changes: &[(String, bool)]) -> Result<Vec<u8>, String> {
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
        return Err("roster has no RosMembers list".into());
    };

    let mut missing: Vec<&str> = changes.iter().map(|(n, _)| n.as_str()).collect();
    for entry in members.iter_mut() {
        let Some(name) = entry.get("RosName").and_then(gff_str) else {
            continue;
        };
        let Some((_, in_party)) = changes.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name))
        else {
            continue;
        };
        missing.retain(|n| !n.eq_ignore_ascii_case(&name));
        if entry.contains_key("RosAvailable") {
            set_int_preserving_type(entry, "RosAvailable", i64::from(!in_party));
        } else {
            entry.insert("RosAvailable".into(), GffValue::Int(i32::from(!in_party)));
        }
    }
    if !missing.is_empty() {
        return Err(format!("roster members not found: {}", missing.join(", ")));
    }

    GffWriter::new(&file_type, &file_version)
        .write_with_struct_id(root, root_struct_id)
        .map_err(|e| format!("roster serialization error: {e}"))
}

/// Rewrite the root `FactionID` of a creature GFF (a `.ros`). Membership is
/// read from `roster.rst`; the engine mirrors it here on save, so we keep the
/// two consistent. Preserves the existing numeric variant, creates as Word.
pub fn set_faction_id(bytes: Vec<u8>, faction: u16) -> Result<Vec<u8>, String> {
    let parser = GffParser::from_bytes(bytes).map_err(|e| format!("ros parse error: {e}"))?;
    let file_type = parser.file_type.clone();
    let file_version = parser.file_version.clone();
    let root_struct_id = parser
        .get_struct_id(0)
        .map_err(|e| format!("ros root id error: {e}"))?;
    let mut root: IndexMap<String, GffValue<'static>> = parser
        .read_struct_fields(0)
        .map_err(|e| format!("ros root error: {e}"))?
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    set_faction_in_fields(&mut root, faction);

    GffWriter::new(&file_type, &file_version)
        .write_with_struct_id(root, root_struct_id)
        .map_err(|e| format!("ros serialization error: {e}"))
}

/// In-memory variant of [`set_faction_id`] for callers that already hold the
/// creature's field map (e.g. the companion save path).
pub fn set_faction_in_fields(fields: &mut IndexMap<String, GffValue<'static>>, faction: u16) {
    if fields.contains_key("FactionID") {
        set_int_preserving_type(fields, "FactionID", i64::from(faction));
    } else {
        fields.insert("FactionID".into(), GffValue::Word(faction));
    }
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
        m.insert("RosGender".into(), GffValue::Byte(0));
        m.insert("RosRace".into(), GffValue::Byte(2));
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
        let identity = RosterIdentity {
            gender: 1,
            subrace: 14,
        };
        let synced = sync_member(bytes, "khelgar", &[(4, 9), (23, 1)], identity)
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
        assert!(matches!(entry.get("RosGender"), Some(GffValue::Byte(1))));
        assert!(matches!(entry.get("RosRace"), Some(GffValue::Byte(14))));
    }

    #[test]
    fn sync_unknown_member_returns_none() {
        let bytes = build_roster(vec![member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1)]);
        let result =
            sync_member(bytes, "nonexistent", &[(1, 1)], RosterIdentity::default()).expect("sync");
        assert!(result.is_none());
    }

    #[test]
    fn parse_party_limit_reads_root_field() {
        let bytes = build_roster(vec![member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1)]);
        assert_eq!(parse_party_limit(bytes).expect("parse"), Some(3));
    }

    #[test]
    fn set_party_membership_flips_available_preserving_type() {
        let bytes = build_roster(vec![
            member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1),
            member("neeshka", "Neeshka", (9, 2), 0, 1),
        ]);
        let updated = set_party_membership(
            bytes,
            &[("Khelgar".into(), true), ("neeshka".into(), false)],
        )
        .expect("set membership");

        let members = parse_roster_members(updated.clone()).expect("reparse");
        assert!(!members[0].available, "khelgar now in party");
        assert!(members[1].available, "neeshka now benched");

        // The builder writes RosAvailable as Byte; the variant must survive.
        let parser = crate::parsers::gff::GffParser::from_bytes(updated).expect("parse");
        let root = parser.read_struct_fields(0).expect("root");
        let GffValue::List(entries) = root.get("RosMembers").expect("RosMembers") else {
            panic!("RosMembers not a list");
        };
        let entry = entries.first().expect("entry").force_load();
        assert!(matches!(entry.get("RosAvailable"), Some(GffValue::Byte(0))));
    }

    #[test]
    fn set_party_membership_creates_missing_field_as_int() {
        let mut stripped = member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1);
        stripped.shift_remove("RosAvailable");
        let bytes = build_roster(vec![stripped]);
        let updated =
            set_party_membership(bytes, &[("khelgar".into(), false)]).expect("set membership");

        let parser = crate::parsers::gff::GffParser::from_bytes(updated).expect("parse");
        let root = parser.read_struct_fields(0).expect("root");
        let GffValue::List(entries) = root.get("RosMembers").expect("RosMembers") else {
            panic!("RosMembers not a list");
        };
        let entry = entries.first().expect("entry").force_load();
        assert!(matches!(entry.get("RosAvailable"), Some(GffValue::Int(1))));
    }

    #[test]
    fn set_party_membership_unknown_member_errors() {
        let bytes = build_roster(vec![member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1)]);
        let err = set_party_membership(bytes, &[("bishop".into(), true)]).unwrap_err();
        assert!(err.contains("bishop"), "error names the member: {err}");
    }

    #[test]
    fn set_faction_id_preserves_word_and_creates_word() {
        let mut fields: IndexMap<String, GffValue<'static>> = IndexMap::new();
        fields.insert("FactionID".into(), GffValue::Word(9));
        let bytes = GffWriter::new("ROS ", "V3.2").write(fields).expect("write");
        let updated = set_faction_id(bytes, BENCHED_FACTION).expect("set");
        let parser = crate::parsers::gff::GffParser::from_bytes(updated).expect("parse");
        let root = parser.read_struct_fields(0).expect("root");
        assert!(matches!(root.get("FactionID"), Some(GffValue::Word(4))));

        let no_field: IndexMap<String, GffValue<'static>> = IndexMap::new();
        let bytes = GffWriter::new("ROS ", "V3.2")
            .write(no_field)
            .expect("write");
        let updated = set_faction_id(bytes, 9).expect("set");
        let parser = crate::parsers::gff::GffParser::from_bytes(updated).expect("parse");
        let root = parser.read_struct_fields(0).expect("root");
        assert!(matches!(root.get("FactionID"), Some(GffValue::Word(9))));
    }
}
