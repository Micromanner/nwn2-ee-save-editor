//! Parse and update the party roster file (`roster.rst` / `ROSTER.rst`).

use indexmap::IndexMap;
use tracing::{info, warn};

use crate::parsers::erf::RES_TYPE_GIT;
use crate::parsers::gff::{GffParser, GffValue, gff_integer, gff_text, rewrite_gff};
use crate::services::campaign::content;
use crate::services::savegame_handler::SaveGameHandler;

pub struct RosterMember {
    pub ros_name: String,
    pub char_name: String,
    /// `(class_id, level)` pairs from `RosClass0-3`/`RosLevel0-3` where level > 0.
    pub classes: Vec<(i32, i32)>,
    /// `RosAvailable` is the engine's party-membership record: 0 = in the
    /// active party, 1 = benched/available for recruitment (in-game verified
    /// 2026-08-31; flipping it swaps party members on the next load).
    pub available: bool,
    /// `RosSelectable`: 0 = locked (forced member or unrecruitable on the
    /// party screen), 1 = the player may pick them. Absent counts as selectable.
    pub selectable: bool,
    /// `RosLoadedBefore`: the companion has spawned from the roster at least once.
    pub loaded_before: bool,
    pub campaign_npc: bool,
    pub gender: Option<i32>,
    /// `RosRace` mirrors the creature's `Subrace` byte (a `racialsubtypes.2da` row).
    pub subrace: Option<i32>,
}

impl RosterMember {
    /// Benched, selectable and spawned from the roster before: a recruited companion
    /// the game may have parked as a creature in an area (OC Act 1: the Sunken Flagon).
    /// Locked members are not: the area creature of an unrecruited SoZ cohort or a junk
    /// NPC is the NPC the player hires or meets.
    pub fn is_parked(&self) -> bool {
        self.available && self.selectable && self.loaded_before
    }
}

/// `FactionID` the engine writes on a benched companion's `.ros` ("Defender").
/// Party members carry the PC's dynamic party faction instead (read it from
/// `playerlist.ifo` `Mod_PlayerList[0].FactionID`; never hardcode it).
pub const BENCHED_FACTION: u16 = 4;

fn int_field(fields: &IndexMap<String, GffValue<'_>>, name: &str) -> Option<i64> {
    fields.get(name).and_then(gff_integer)
}

pub fn parse_roster_members(bytes: Vec<u8>) -> Result<Vec<RosterMember>, String> {
    let parser = GffParser::from_bytes(bytes).map_err(|e| format!("roster parse error: {e}"))?;
    let root = parser
        .read_struct_fields(0)
        .map_err(|e| format!("roster root error: {e}"))?;

    let Some(members_value) = root.get("RosMembers") else {
        return Ok(Vec::new());
    };
    let GffValue::ListOwned(entries) = members_value
        .clone()
        .force_owned()
        .map_err(|e| format!("roster members error: {e}"))?
    else {
        return Err("RosMembers is not a list".into());
    };

    let mut out = Vec::new();
    for entry in &entries {
        let Some(ros_name) = gff_text(entry, "RosName") else {
            continue;
        };
        let char_name = gff_text(entry, "RosCharName").unwrap_or_else(|| ros_name.clone());
        let mut classes = Vec::new();
        for i in 0..4 {
            let class_id = int_field(entry, &format!("RosClass{i}")).unwrap_or(0);
            let level = int_field(entry, &format!("RosLevel{i}")).unwrap_or(0);
            if level > 0 {
                classes.push((class_id as i32, level as i32));
            }
        }
        out.push(RosterMember {
            ros_name,
            char_name,
            classes,
            available: int_field(entry, "RosAvailable").unwrap_or(0) != 0,
            selectable: int_field(entry, "RosSelectable").unwrap_or(1) != 0,
            loaded_before: int_field(entry, "RosLoadedBefore").unwrap_or(0) != 0,
            campaign_npc: int_field(entry, "RosCampaignNPC").unwrap_or(0) != 0,
            gender: int_field(entry, "RosGender").map(|v| v as i32),
            subrace: int_field(entry, "RosRace").map(|v| v as i32),
        });
    }
    Ok(out)
}

/// Fields the roster caches from a member's `.ros` and shows in the party-selection UI.
#[derive(Debug, Clone, Copy, Default)]
pub struct RosterIdentity {
    pub gender: i32,
    /// `RosRace` mirrors the creature's `Subrace` byte (a `racialsubtypes.2da` row), not `Race`.
    pub subrace: i32,
}

/// Roster members that have a `.ros` file in the save; empty without a roster file.
/// Neither globals.xml `*_Joined` vars nor the RosAvailable/RosSelectable/
/// RosCampaignNPC flags identify a companion: the game skips the vars for
/// some companions and clears all three flags on an active party member.
pub fn companion_members(handler: &SaveGameHandler) -> Result<Vec<RosterMember>, String> {
    let Some((_, bytes)) = handler
        .extract_roster()
        .map_err(|e| format!("Failed to read roster: {e}"))?
    else {
        warn!("No roster file found in save; companion list empty");
        return Ok(Vec::new());
    };
    let members =
        parse_roster_members(bytes).map_err(|e| format!("Failed to parse roster: {e}"))?;
    let companion_files = handler
        .list_companions()
        .map_err(|e| format!("Failed to list companions: {e}"))?;
    Ok(members
        .into_iter()
        .filter(|m| {
            companion_files
                .iter()
                .any(|c| c.eq_ignore_ascii_case(&m.ros_name))
        })
        .collect())
}

/// Rewrite the cached class/level pairs and identity of every member named
/// `ros_name`. `Ok(None)` means the roster has no matching member; callers surface
/// that as a warning, not an error.
pub fn sync_member(
    bytes: Vec<u8>,
    ros_name: &str,
    classes: &[(i32, i32)],
    identity: RosterIdentity,
) -> Result<Option<Vec<u8>>, String> {
    let mut found = false;
    let bytes = rewrite_gff(bytes, "roster", |root| {
        let Some(GffValue::ListOwned(members)) = root.get_mut("RosMembers") else {
            return Ok(());
        };
        for entry in members.iter_mut() {
            if !gff_text(entry, "RosName").is_some_and(|n| n.eq_ignore_ascii_case(ros_name)) {
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
        }
        Ok(())
    })?;
    Ok(found.then_some(bytes))
}

/// `RosPartyLimit` from the roster root: how many companions the party screen
/// allows at once. `None` when the roster lacks the field.
pub fn parse_party_limit(bytes: Vec<u8>) -> Result<Option<i32>, String> {
    let parser = GffParser::from_bytes(bytes).map_err(|e| format!("roster parse error: {e}"))?;
    let root = parser
        .read_struct_fields(0)
        .map_err(|e| format!("roster root error: {e}"))?;
    Ok(int_field(&root, "RosPartyLimit").map(|v| v as i32))
}

/// Set `RosAvailable` for each `(ros_name, in_party)` change (in party = 0,
/// benched = 1). See [`set_member_int`].
pub fn set_party_membership(bytes: Vec<u8>, changes: &[(String, bool)]) -> Result<Vec<u8>, String> {
    let values: Vec<(String, i64)> = changes
        .iter()
        .map(|(name, in_party)| (name.clone(), i64::from(!in_party)))
        .collect();
    set_member_int(bytes, "RosAvailable", &values)
}

/// Set `RosSelectable` for each `(ros_name, selectable)` change. See [`set_member_int`].
pub fn set_selectable(bytes: Vec<u8>, changes: &[(String, bool)]) -> Result<Vec<u8>, String> {
    let values: Vec<(String, i64)> = changes
        .iter()
        .map(|(name, selectable)| (name.clone(), i64::from(*selectable)))
        .collect();
    set_member_int(bytes, "RosSelectable", &values)
}

/// Write an integer `field` on each named `RosMembers` entry, preserving the
/// file's numeric GFF variant and creating the field as Int (the engine's type)
/// on entries that lack it. Errors if any named member is missing from the roster.
fn set_member_int(
    bytes: Vec<u8>,
    field: &str,
    changes: &[(String, i64)],
) -> Result<Vec<u8>, String> {
    rewrite_gff(bytes, "roster", |root| {
        let Some(GffValue::ListOwned(members)) = root.get_mut("RosMembers") else {
            return Err("roster has no RosMembers list".into());
        };
        let mut missing: Vec<&str> = changes.iter().map(|(n, _)| n.as_str()).collect();
        for entry in members.iter_mut() {
            let Some(name) = gff_text(entry, "RosName") else {
                continue;
            };
            let Some((_, value)) = changes.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name))
            else {
                continue;
            };
            missing.retain(|n| !n.eq_ignore_ascii_case(&name));
            if entry.contains_key(field) {
                set_int_preserving_type(entry, field, *value);
            } else {
                entry.insert(
                    field.into(),
                    GffValue::Int((*value).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32),
                );
            }
        }
        if !missing.is_empty() {
            return Err(format!("roster members not found: {}", missing.join(", ")));
        }
        Ok(())
    })
}

/// The PC's dynamic party faction id (`Mod_PlayerList[0].FactionID`, one past
/// the module's saved faction table). Party members' `.ros` carry this value;
/// benched companions carry [`BENCHED_FACTION`].
pub fn pc_party_faction(handler: &SaveGameHandler) -> Option<u16> {
    let data = handler.extract_player_data().ok()?;
    let gff = GffParser::from_bytes(data).ok()?;
    let root = gff.read_struct_fields(0).ok()?;
    let GffValue::List(players) = root.get("Mod_PlayerList")? else {
        return None;
    };
    let first = players.first()?.force_load().ok()?;
    u16::try_from(int_field(&first, "FactionID")?).ok()
}

/// Change which companions are in the active party, in the save or the open
/// batch. Writes `RosAvailable` in roster.rst (the record the engine reads on
/// load) and mirrors the matching `FactionID` into each changed `.ros`. Only
/// members whose desired state differs from the file are touched. Returns the
/// warnings when the roster changed but the faction mirror failed.
pub fn apply_party_membership(
    handler: &mut SaveGameHandler,
    desired: &[(String, bool)],
) -> Result<Vec<String>, String> {
    let Some((stored_name, bytes)) = handler
        .extract_roster()
        .map_err(|e| format!("Failed to read roster: {e}"))?
    else {
        return Err("Save has no roster file".into());
    };

    let members = parse_roster_members(bytes.clone())?;
    let changes: Vec<(String, bool)> = desired
        .iter()
        .filter(|(name, in_party)| {
            members
                .iter()
                .any(|m| m.ros_name.eq_ignore_ascii_case(name) && m.available == *in_party)
        })
        .cloned()
        .collect();
    if changes.is_empty() {
        return Ok(Vec::new());
    }

    let updated = set_party_membership(bytes, &changes)?;
    handler
        .update_file(&stored_name, &updated)
        .map_err(|e| format!("Failed to write roster: {e}"))?;

    // Mirror the membership into each changed .ros FactionID. The engine
    // does not read it for membership, but writes it itself on save; a
    // failure here leaves a cosmetic inconsistency, not a broken party.
    let Some(party_faction) = pc_party_faction(handler) else {
        warn!("Could not read PC party faction from playerlist.ifo");
        return Ok(vec!["faction sync skipped".into()]);
    };
    let mut failed = Vec::new();
    for (ros_name, in_party) in &changes {
        let faction = if *in_party {
            party_faction
        } else {
            BENCHED_FACTION
        };
        let result = handler
            .companion_stored_name(ros_name)
            .map_err(|e| e.to_string())
            .and_then(|stored| {
                let ros = handler.extract_file(&stored).map_err(|e| e.to_string())?;
                let bytes = set_faction_id(ros, faction)?;
                handler
                    .update_file(&stored, &bytes)
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = result {
            warn!("Faction sync failed for {ros_name}: {e}");
            failed.push(ros_name.as_str());
        }
    }
    if failed.is_empty() {
        Ok(Vec::new())
    } else {
        Ok(vec![format!("faction sync failed: {}", failed.join(", "))])
    }
}

/// Delete the area copies of the parked companions (see [`RosterMember::is_parked`])
/// whose `.ros` the open batch rewrites, in all module states of the save. A parked
/// companion is also a full creature in that area's `.git`, and the game loads that
/// object instead of the `.ros`; joining the party in-game deletes it. Without the copy
/// the game spawns the companion from the `.ros` (`SpawnRosterMember`, as the Flagon's
/// enter script does). A module state that cannot be read is skipped; returns a
/// warning for each.
pub fn remove_staged_companion_area_copies(
    handler: &mut SaveGameHandler,
) -> Result<Vec<String>, String> {
    let staged = handler.staged_files();
    if !staged
        .iter()
        .any(|name| name.to_ascii_lowercase().ends_with(".ros"))
    {
        return Ok(Vec::new());
    }
    let Some((_, roster)) = handler
        .extract_roster()
        .map_err(|e| format!("Failed to read roster: {e}"))?
    else {
        return Ok(Vec::new());
    };

    let mut tags = Vec::new();
    for member in parse_roster_members(roster)?
        .iter()
        .filter(|m| m.is_parked())
    {
        let Ok(name) = handler.companion_stored_name(&member.ros_name) else {
            continue;
        };
        if !staged.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
            continue;
        }
        let bytes = handler.extract_file(&name).map_err(|e| e.to_string())?;
        let parser = GffParser::from_bytes(bytes).map_err(|e| format!("{name}: {e}"))?;
        let root = parser
            .read_struct_fields(0)
            .map_err(|e| format!("{name}: {e}"))?;
        if let Some(tag) = gff_text(&root, "Tag").filter(|t| !t.is_empty()) {
            tags.push(tag);
        }
    }
    if tags.is_empty() {
        return Ok(Vec::new());
    }

    let mut warnings = Vec::new();
    for module in content::module_state_ids(handler.save_dir()) {
        match module_without_creatures(handler, &module, &tags) {
            Ok(Some((file_name, bytes))) => {
                handler
                    .update_file(&file_name, &bytes)
                    .map_err(|e| format!("Failed to write {file_name}: {e}"))?;
                info!("Removed parked companion copies from {module}");
            }
            Ok(None) => {}
            Err(e) => {
                let warning = format!("Could not check {module} for parked companion copies: {e}");
                warn!("{warning}");
                warnings.push(warning);
            }
        }
    }
    Ok(warnings)
}

/// The state of `module` without the creatures tagged with one of `tags`, as its `.z`
/// file name and new bytes; `None` when no area holds one.
fn module_without_creatures(
    handler: &SaveGameHandler,
    module: &str,
    tags: &[String],
) -> Result<Option<(String, Vec<u8>)>, String> {
    let (z_path, decompressed) = content::read_module_state(handler, module)?;
    // Most modules hold none of the tags: skip parsing hundreds of MB of resources.
    if !tags.iter().any(|tag| holds_gff_string(&decompressed, tag)) {
        return Ok(None);
    }
    let mut erf = content::parse_module_state(&decompressed)?;
    let mut edited = false;
    for (area, _, _) in erf.list_resources(Some(RES_TYPE_GIT)) {
        let mut edit_area = || -> Result<bool, String> {
            let bytes = erf.extract_resource(&area).map_err(|e| e.to_string())?;
            let Some(bytes) = without_creatures(bytes, tags)? else {
                return Ok(false);
            };
            erf.update_resource(&area, bytes)
                .map_err(|e| e.to_string())?;
            Ok(true)
        };
        edited |= edit_area().map_err(|e| format!("{area}: {e}"))?;
    }
    if !edited {
        return Ok(None);
    }
    let bytes = content::serialize_module_state(&erf, &z_path)?;
    let file_name = z_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid module filename")?;
    Ok(Some((file_name.to_string(), bytes)))
}

/// Whether `bytes` (a GFF, or an archive of them) store the string `text`, compared
/// case-insensitively: a GFF string is its u32 length followed by the text.
fn holds_gff_string(bytes: &[u8], text: &str) -> bool {
    let mut needle = (text.len() as u32).to_le_bytes().to_vec();
    needle.extend_from_slice(text.as_bytes());
    bytes
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(&needle))
}

/// An area `.git` without the `Creature List` entries tagged with one of `tags`, or
/// `None` when it holds none of them. Tags compare case-insensitively.
fn without_creatures(bytes: Vec<u8>, tags: &[String]) -> Result<Option<Vec<u8>>, String> {
    if !tags.iter().any(|tag| holds_gff_string(&bytes, tag)) {
        return Ok(None);
    }
    let mut removed = false;
    let bytes = rewrite_gff(bytes, "git", |root| {
        if let Some(GffValue::ListOwned(creatures)) = root.get_mut("Creature List") {
            let before = creatures.len();
            creatures.retain(|creature| {
                gff_text(creature, "Tag")
                    .is_none_or(|tag| !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)))
            });
            removed = creatures.len() != before;
        }
        Ok(())
    })?;
    Ok(removed.then_some(bytes))
}

/// Set `RosSelectable` on the named members, in the save or the open batch.
/// Members already in the desired state are left untouched.
pub fn apply_selectable(
    handler: &mut SaveGameHandler,
    desired: &[(String, bool)],
) -> Result<(), String> {
    let Some((stored_name, bytes)) = handler
        .extract_roster()
        .map_err(|e| format!("Failed to read roster: {e}"))?
    else {
        return Err("Save has no roster file".into());
    };
    let members = parse_roster_members(bytes.clone())?;
    let changes: Vec<(String, bool)> = desired
        .iter()
        .filter(|(name, selectable)| {
            members
                .iter()
                .any(|m| m.ros_name.eq_ignore_ascii_case(name) && m.selectable != *selectable)
        })
        .cloned()
        .collect();
    if changes.is_empty() {
        return Ok(());
    }
    let updated = set_selectable(bytes, &changes)?;
    handler
        .update_file(&stored_name, &updated)
        .map_err(|e| format!("Failed to write roster: {e}"))
}

/// Rewrite the root `FactionID` of a creature GFF (a `.ros`). Membership is
/// read from `roster.rst`; the engine mirrors it here on save, so we keep the
/// two consistent. Preserves the existing numeric variant, creates as Word.
pub fn set_faction_id(bytes: Vec<u8>, faction: u16) -> Result<Vec<u8>, String> {
    rewrite_gff(bytes, "ros", |root| {
        set_faction_in_fields(root, faction);
        Ok(())
    })
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
        assert_eq!(members[0].gender, Some(0));
        assert_eq!(members[0].subrace, Some(2));
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
        let entry = entries
            .first()
            .expect("entry")
            .force_load()
            .expect("entry fields");
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
        let entry = entries
            .first()
            .expect("entry")
            .force_load()
            .expect("entry fields");
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
        let entry = entries
            .first()
            .expect("entry")
            .force_load()
            .expect("entry fields");
        assert!(matches!(entry.get("RosAvailable"), Some(GffValue::Int(1))));
    }

    #[test]
    fn set_party_membership_unknown_member_errors() {
        let bytes = build_roster(vec![member("khelgar", "Khelgar Ironfist", (4, 2), 1, 1)]);
        let err = set_party_membership(bytes, &[("bishop".into(), true)]).unwrap_err();
        assert!(err.contains("bishop"), "error names the member: {err}");
    }

    fn creature(fields: Vec<(&str, GffValue<'static>)>) -> IndexMap<String, GffValue<'static>> {
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn tag(value: &str) -> GffValue<'static> {
        GffValue::String(value.to_string().into())
    }

    fn git(root: Vec<(&str, GffValue<'static>)>) -> Vec<u8> {
        GffWriter::new("GIT ", "V3.2")
            .write(creature(root))
            .expect("write git")
    }

    fn creature_tags(bytes: Vec<u8>) -> Vec<Option<String>> {
        let root = GffParser::from_bytes(bytes)
            .expect("parse")
            .read_owned_struct(0)
            .expect("root");
        let Some(GffValue::ListOwned(creatures)) = root.get("Creature List") else {
            panic!("no Creature List");
        };
        creatures.iter().map(|c| gff_text(c, "Tag")).collect()
    }

    #[test]
    fn without_creatures_removes_tagged_entries_and_keeps_diff_entries() {
        let bytes = git(vec![(
            "Creature List",
            GffValue::ListOwned(vec![
                creature(vec![("Tag", tag("Bishop"))]),
                creature(vec![("DIFF_INDEX", GffValue::Dword(3))]),
                creature(vec![("Tag", tag("duncan"))]),
            ]),
        )]);
        let updated = without_creatures(bytes, &["bishop".into()])
            .expect("filter")
            .expect("changed");
        assert_eq!(creature_tags(updated), vec![None, Some("duncan".into())]);
    }

    #[test]
    fn without_creatures_ignores_tags_inside_other_strings() {
        let bytes = git(vec![(
            "Creature List",
            GffValue::ListOwned(vec![creature(vec![("Tag", tag("thousand"))])]),
        )]);
        assert!(
            without_creatures(bytes, &["sand".into()])
                .expect("filter")
                .is_none()
        );
    }

    #[test]
    fn without_creatures_leaves_other_lists_alone() {
        let bytes = git(vec![(
            "WaypointList",
            GffValue::ListOwned(vec![creature(vec![("Tag", tag("bishop"))])]),
        )]);
        assert!(
            without_creatures(bytes, &["bishop".into()])
                .expect("filter")
                .is_none()
        );
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
