use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use app_lib::parsers::gff::{GffParser, GffValue};
use app_lib::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;
use serde_json::Value as JsonValue;

fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gff_dump")
}

fn saves_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/Classic_Campaign")
}

/// BIC field groups - maps each top-level field to a category file
fn bic_field_groups() -> BTreeMap<&'static str, &'static str> {
    let groups: &[(&str, &[&str])] = &[
        (
            "identity",
            &[
                "FirstName",
                "LastName",
                "Description",
                "Tag",
                "TemplateResRef",
                "Subrace",
                "Deity",
                "Gender",
                "Race",
                "Age",
                "IsPC",
                "IsPCDisguised",
                "BodyBag",
                "BodyBagId",
                "PortraitId",
                "Portrait",
                "CustomPortrait",
                "Conversation",
                "DecayTime",
                "CompanionName",
                "CompanionType",
                "RosterMember",
                "RosterTag",
                "CharBackground",
                "Mod_CommntyId",
                "Mod_CommntyName",
                "Mod_CommntyPlatf",
                "Mod_IsPrimaryPlr",
                "Mod_LastModId",
            ],
        ),
        (
            "abilities",
            &[
                "Str",
                "Dex",
                "Con",
                "Int",
                "Wis",
                "Cha",
                "HitPoints",
                "MaxHitPoints",
                "CurrentHitPoints",
                "TempHitPoints",
                "PregameCurrent",
                "Experience",
                "Gold",
                "SkillPoints",
            ],
        ),
        (
            "combat",
            &[
                "BAB",
                "NaturalAC",
                "ArmorClass",
                "DR",
                "SR",
                "MovementRate",
                "BaseAttackBonus",
                "ChallengeRating",
                "OverrideBAB",
                "OverrideBABMin",
                "OnHandAttacks",
                "OffHandAttacks",
                "DamageMin",
                "DamageMax",
                "CombatMode",
                "CombatInfo",
                "CombatRoundData",
                "DmgReduction",
                "AttackResult",
                "oidTarget",
                "CreatureSize",
            ],
        ),
        (
            "alignment",
            &["GoodEvil", "LawfulChaotic", "LawfulChaotic2"],
        ),
        (
            "saves",
            &[
                "fortbonus",
                "refbonus",
                "willbonus",
                "FortSaveThrow",
                "RefSaveThrow",
                "WillSaveThrow",
            ],
        ),
        (
            "appearance",
            &[
                "Appearance_Type",
                "Appearance_Head",
                "Appearance_Hair",
                "Appearance_FHair",
                "AppearanceSEF",
                "Color_Skin",
                "Color_Hair",
                "Color_Tattoo1",
                "Color_Tattoo2",
                "SoundSetFile",
                "Phenotype",
                "Wings",
                "Wings_NewID",
                "Tail",
                "Tail_NewID",
                "Variation",
                "ArmorVisualType",
                "NeverDrawHelmet",
                "NeverShowArmor",
                "CrtrCastsShadow",
                "CrtrRcvShadow",
            ],
        ),
        (
            "body_parts",
            &[
                "ACBkHip",
                "ACFtHip",
                "ACLtAnkle",
                "ACLtArm",
                "ACLtBracer",
                "ACLtElbow",
                "ACLtFoot",
                "ACLtHip",
                "ACLtKnee",
                "ACLtLeg",
                "ACLtShin",
                "ACLtShoulder",
                "ACRtAnkle",
                "ACRtArm",
                "ACRtBracer",
                "ACRtElbow",
                "ACRtFoot",
                "ACRtHip",
                "ACRtKnee",
                "ACRtLeg",
                "ACRtShin",
                "ACRtShoulder",
            ],
        ),
        (
            "tints",
            &["Tint_Hair", "Tint_Head", "Tintable", "ArmorTint"],
        ),
        ("model", &["ModelScale", "UVScroll"]),
        (
            "position",
            &[
                "XPosition",
                "YPosition",
                "ZPosition",
                "XOrientation",
                "YOrientation",
                "ZOrientation",
                "AreaId",
            ],
        ),
        ("classes", &["ClassList"]),
        ("feats", &["FeatList"]),
        ("skills", &["SkillList"]),
        (
            "spells",
            &[
                "SpellMemorizedList0",
                "SpellMemorizedList1",
                "SpellMemorizedList2",
                "SpellMemorizedList3",
                "SpellMemorizedList4",
                "SpellMemorizedList5",
                "SpellMemorizedList6",
                "SpellMemorizedList7",
                "SpellMemorizedList8",
                "SpellKnownList0",
                "SpellKnownList1",
                "SpellKnownList2",
                "SpellKnownList3",
                "SpellKnownList4",
                "SpellKnownList5",
                "SpellKnownList6",
                "SpellKnownList7",
                "SpellKnownList8",
            ],
        ),
        ("level_history", &["LvlStatList"]),
        ("inventory", &["Equip_ItemList", "ItemList"]),
        (
            "scripts",
            &[
                "ScriptAttacked",
                "ScriptDamaged",
                "ScriptDeath",
                "ScriptDialogue",
                "ScriptDisturbed",
                "ScriptEndRound",
                "ScriptHeartbeat",
                "ScriptOnBlocked",
                "ScriptOnNotice",
                "ScriptRested",
                "ScriptSpawn",
                "ScriptSpellAt",
                "ScriptUserDefine",
                "ScriptHidden",
                "ScriptsBckdUp",
                "OriginAttacked",
                "OriginDamaged",
                "OriginDeath",
                "OriginDialogue",
                "OriginDisturbed",
                "OriginEndRound",
                "OriginHeartbeat",
                "OriginOnBlocked",
                "OriginOnNotice",
                "OriginRested",
                "OriginSpawn",
                "OriginSpellAt",
                "OriginUserDefine",
            ],
        ),
        ("hotbar", &["HotbarList"]),
        ("perception", &["PerceptionList", "PerceptionRange"]),
        ("expressions", &["ExpressionList"]),
        (
            "state_flags",
            &[
                "IsCommandable",
                "IsDestroyable",
                "IsImmortal",
                "IsRaiseable",
                "IsDM",
                "Plot",
                "Lootable",
                "Disarmable",
                "DeadSelectable",
                "Interruptable",
                "Listening",
                "AlwysPrcvbl",
                "NoPermDeath",
                "UnrestrictLU",
                "MClassLevUpIn",
                "StartingPackage",
                "AmbientAnimState",
                "AnimationDay",
                "AnimationTime",
                "BumpState",
                "CreatnScrptFird",
                "CreatureVersion",
                "CustomHeartbeat",
                "DefCastMode",
                "DetectMode",
                "DisableAIHidden",
                "EnhVisionMode",
                "FactionID",
                "HlfrBlstMode",
                "HlfrShldMode",
                "IgnoreTarget",
                "BlockBroadcast",
                "BlockCombat",
                "BlockRespond",
                "MasterID",
                "ObjectId",
                "OrientOnDialog",
                "PM_IsPolymorphed",
                "PossBlocked",
                "SitObject",
                "SpiritOverride",
                "StealthMode",
                "TalkPlayerOwn",
                "TrackingMode",
                "XpMod",
                "ConjureSoundTag",
            ],
        ),
        (
            "runtime_state",
            &[
                "ActionList",
                "EffectList",
                "PersonalRepList",
                "ReputationList",
                "VarTable",
            ],
        ),
        (
            "module_tracking",
            &["Mod_FirstName", "Mod_LastName", "Mod_ModuleList"],
        ),
    ];

    let mut map = BTreeMap::new();
    for (group, fields) in groups {
        for field in *fields {
            map.insert(*field, *group);
        }
    }
    map
}

fn write_json(path: &std::path::Path, value: &JsonValue) {
    let json = serde_json::to_string_pretty(value).expect("Failed to pretty-print JSON");
    fs::write(path, &json).expect("Failed to write JSON file");
    println!("  Wrote: {} ({} bytes)", path.display(), json.len());
}

fn gff_type_name(v: &GffValue<'_>) -> &'static str {
    match v {
        GffValue::Byte(_) => "Byte",
        GffValue::Char(_) => "Char",
        GffValue::Word(_) => "Word",
        GffValue::Short(_) => "Short",
        GffValue::Dword(_) => "Dword",
        GffValue::Int(_) => "Int",
        GffValue::Dword64(_) => "Dword64",
        GffValue::Int64(_) => "Int64",
        GffValue::Float(_) => "Float",
        GffValue::Double(_) => "Double",
        GffValue::String(_) => "String",
        GffValue::ResRef(_) => "ResRef",
        GffValue::LocString(_) => "LocString",
        GffValue::Void(_) => "Void",
        GffValue::Struct(_) | GffValue::StructOwned(_) | GffValue::StructRef(_) => "Struct",
        GffValue::List(_) | GffValue::ListOwned(_) | GffValue::ListRef(_) => "List",
    }
}

fn dump_character_fields(fields: &IndexMap<String, GffValue<'static>>, dir: &std::path::Path) {
    fs::create_dir_all(dir).expect("Failed to create dir");

    let field_groups = bic_field_groups();

    let mut groups: BTreeMap<String, IndexMap<String, &GffValue<'static>>> = BTreeMap::new();
    let mut ungrouped: IndexMap<String, &GffValue<'static>> = IndexMap::new();

    for (key, value) in fields {
        if key == "__struct_id__" {
            continue;
        }
        if let Some(group) = field_groups.get(key.as_str()) {
            groups
                .entry(group.to_string())
                .or_default()
                .insert(key.clone(), value);
        } else {
            ungrouped.insert(key.clone(), value);
        }
    }

    for (group_name, group_fields) in &groups {
        let json = serde_json::to_value(group_fields).expect("serialize");
        write_json(&dir.join(format!("{group_name}.json")), &json);
    }

    if !ungrouped.is_empty() {
        let json = serde_json::to_value(&ungrouped).expect("serialize");
        write_json(&dir.join("other.json"), &json);
    }

    let index: BTreeMap<&str, &str> = fields
        .iter()
        .filter(|(k, _)| k.as_str() != "__struct_id__")
        .map(|(k, v)| (k.as_str(), gff_type_name(v)))
        .collect();

    let index_json = serde_json::to_value(&index).expect("serialize");
    write_json(&dir.join("_field_index.json"), &index_json);

    let total = fields.len();
    let grouped: usize = groups.values().map(|g| g.len()).sum();
    println!(
        "  => {total} fields in {} groups + {} ungrouped",
        groups.len(),
        total - grouped
    );
}

#[test]
fn dump_bic_structured() {
    let handler = SaveGameHandler::new(saves_dir(), false, false).expect("Failed to open save");

    let bic_bytes = handler
        .extract_file("player.bic")
        .expect("Failed to extract player.bic");

    let parser = GffParser::from_bytes(bic_bytes).expect("Failed to parse BIC");
    let root_fields = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");

    let owned: IndexMap<String, GffValue<'static>> = root_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    println!("Dumping player.bic:");
    dump_character_fields(&owned, &output_dir().join("bic"));
}

#[test]
fn dump_ifo_structured() {
    let handler = SaveGameHandler::new(saves_dir(), false, false).expect("Failed to open save");

    let ifo_bytes = handler
        .extract_file("playerlist.ifo")
        .expect("Failed to extract playerlist.ifo");

    let parser = GffParser::from_bytes(ifo_bytes).expect("Failed to parse IFO");
    let root_fields = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");

    let owned: IndexMap<String, GffValue<'static>> = root_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let ifo_dir = output_dir().join("ifo");
    fs::create_dir_all(&ifo_dir).expect("Failed to create ifo dir");

    // Dump IFO-level field index
    let index: BTreeMap<&str, &str> = owned
        .iter()
        .map(|(k, v)| (k.as_str(), gff_type_name(v)))
        .collect();
    let index_json = serde_json::to_value(&index).expect("serialize");
    write_json(&ifo_dir.join("_field_index.json"), &index_json);

    // Mod_PlayerList is the main content - a list of character structs
    if let Some(GffValue::ListOwned(players)) = owned.get("Mod_PlayerList") {
        println!("IFO has {} player(s) in Mod_PlayerList", players.len());
        for (i, player_fields) in players.iter().enumerate() {
            let name = player_fields
                .get("FirstName")
                .and_then(|v| match v {
                    GffValue::LocString(ls) => ls.substrings.first().map(|s| s.string.to_string()),
                    _ => None,
                })
                .unwrap_or_else(|| format!("player_{i}"));

            let slug = name.to_lowercase().replace(' ', "_");
            let player_dir = ifo_dir.join(&slug);

            println!("Dumping IFO player {i}: {name}");
            dump_character_fields(player_fields, &player_dir);
        }
    }

    // Dump any other IFO fields (non Mod_PlayerList)
    let other_scalars: IndexMap<String, &GffValue<'static>> = owned
        .iter()
        .filter(|(k, _)| k.as_str() != "Mod_PlayerList")
        .map(|(k, v)| (k.clone(), v))
        .collect();

    if !other_scalars.is_empty() {
        let json = serde_json::to_value(&other_scalars).expect("serialize");
        write_json(&ifo_dir.join("scalars.json"), &json);
    }

    println!("\nIFO dump complete: {} top-level fields", owned.len());
}

fn cheatdebug_dir(save_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/saves/cheatdebug")
        .join(save_name)
}

fn dump_ifo_to(save_name: &str, out_subdir: &str) {
    let handler =
        SaveGameHandler::new(cheatdebug_dir(save_name), false, false).expect("Failed to open save");

    let ifo_bytes = handler
        .extract_file("playerlist.ifo")
        .expect("Failed to extract playerlist.ifo");

    let parser = GffParser::from_bytes(ifo_bytes).expect("Failed to parse IFO");
    let root_fields = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");

    let owned: IndexMap<String, GffValue<'static>> = root_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let ifo_dir = output_dir().join("cheatdebug").join(out_subdir);
    fs::create_dir_all(&ifo_dir).expect("Failed to create ifo dir");

    let index: BTreeMap<&str, &str> = owned
        .iter()
        .map(|(k, v)| (k.as_str(), gff_type_name(v)))
        .collect();
    let index_json = serde_json::to_value(&index).expect("serialize");
    write_json(&ifo_dir.join("_field_index.json"), &index_json);

    if let Some(GffValue::ListOwned(players)) = owned.get("Mod_PlayerList") {
        println!("[{out_subdir}] IFO has {} player(s)", players.len());
        for (i, player_fields) in players.iter().enumerate() {
            let name = player_fields
                .get("FirstName")
                .and_then(|v| match v {
                    GffValue::LocString(ls) => ls.substrings.first().map(|s| s.string.to_string()),
                    _ => None,
                })
                .unwrap_or_else(|| format!("player_{i}"));

            let slug = name.to_lowercase().replace(' ', "_");
            let player_dir = ifo_dir.join(&slug);

            println!("[{out_subdir}] Dumping player {i}: {name}");
            dump_character_fields(player_fields, &player_dir);
        }
    }

    let other_scalars: IndexMap<String, &GffValue<'static>> = owned
        .iter()
        .filter(|(k, _)| k.as_str() != "Mod_PlayerList")
        .map(|(k, v)| (k.clone(), v))
        .collect();

    if !other_scalars.is_empty() {
        let json = serde_json::to_value(&other_scalars).expect("serialize");
        write_json(&ifo_dir.join("scalars.json"), &json);
    }
}

#[test]
fn dump_cheatdebug_noncheat_ifo() {
    dump_ifo_to("000061 - 16-04-2026-23-01", "noncheat");
}

#[test]
fn dump_cheatdebug_cheat_ifo() {
    dump_ifo_to("000062 - 16-04-2026-23-04", "cheat");
}

#[test]
fn copy_globals_xml() {
    let xml_dir = output_dir().join("xml");
    fs::create_dir_all(&xml_dir).expect("Failed to create xml dir");

    let src = saves_dir().join("globals.xml");
    let dst = xml_dir.join("globals.xml");

    fs::copy(&src, &dst).expect("Failed to copy globals.xml");
    println!(
        "Copied globals.xml ({} bytes)",
        fs::metadata(&dst).unwrap().len()
    );
}

// ---------------------------------------------------------------------------
// Profile a bloated playerlist.ifo / player.bic to find the heaviest field
// paths without materializing the whole tree into memory.
// ---------------------------------------------------------------------------

use std::sync::Arc;

// GFF LocString on-disk framing: 4-byte total_size + 4-byte string_ref +
// 4-byte count header, then 4-byte id + 4-byte len per substring.
const LOCSTRING_HEADER_BYTES: u64 = 4;
const LOCSTRING_SUBSTRING_HEADER_BYTES: u64 = 8;

enum ProfileEntry {
    Bytes { path: String, bytes: u64 },
    ListLen { path: String, count: u64 },
}

struct Profile {
    total_bytes: u64,
    entries: Vec<ProfileEntry>,
}

impl Profile {
    fn new() -> Self {
        Self {
            total_bytes: 0,
            entries: Vec::new(),
        }
    }
}

fn scalar_bytes(v: &GffValue<'_>) -> u64 {
    match v {
        GffValue::Byte(_) | GffValue::Char(_) => 1,
        GffValue::Word(_) | GffValue::Short(_) => 2,
        GffValue::Dword(_) | GffValue::Int(_) | GffValue::Float(_) => 4,
        GffValue::Dword64(_) | GffValue::Int64(_) | GffValue::Double(_) => 8,
        GffValue::String(s) | GffValue::ResRef(s) => s.len() as u64,
        GffValue::LocString(ls) => ls
            .substrings
            .iter()
            .map(|s| s.string.len() as u64 + LOCSTRING_SUBSTRING_HEADER_BYTES)
            .sum::<u64>()
            .saturating_add(LOCSTRING_HEADER_BYTES),
        GffValue::Void(b) => b.len() as u64,
        _ => 0,
    }
}

fn profile_struct(
    parser: &Arc<GffParser>,
    fields: &IndexMap<String, GffValue<'_>>,
    path: &str,
    profile: &mut Profile,
    depth: usize,
    max_depth: usize,
) -> u64 {
    let mut subtotal: u64 = 0;
    for (key, value) in fields {
        if key == "__struct_id__" {
            continue;
        }
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        let bytes = match value {
            GffValue::Struct(lazy) if depth < max_depth => parser
                .read_struct_fields(lazy.struct_index)
                .map(|inner| {
                    profile_struct(parser, &inner, &child_path, profile, depth + 1, max_depth)
                })
                .unwrap_or(0),
            GffValue::List(items) => {
                profile.entries.push(ProfileEntry::ListLen {
                    path: format!("{child_path} (list len)"),
                    count: items.len() as u64,
                });
                if depth >= max_depth {
                    0
                } else {
                    items
                        .iter()
                        .filter_map(|lazy| parser.read_struct_fields(lazy.struct_index).ok())
                        .map(|inner| {
                            profile_struct(
                                parser,
                                &inner,
                                &format!("{child_path}[]"),
                                profile,
                                depth + 1,
                                max_depth,
                            )
                        })
                        .sum()
                }
            }
            GffValue::Struct(_) => 0,
            _ => scalar_bytes(value),
        };
        if bytes > 0 {
            profile.entries.push(ProfileEntry::Bytes {
                path: child_path,
                bytes,
            });
        }
        subtotal += bytes;
    }
    subtotal
}

fn profile_gff_file(path: &std::path::Path, label: &str, max_depth: usize) {
    println!("\n=== Profiling {label}: {} ===", path.display());
    let file_size = fs::metadata(path).expect("metadata").len();
    println!(
        "  File size: {file_size} bytes ({:.1} MB)",
        file_size as f64 / 1_048_576.0
    );

    let parser = GffParser::new(path).expect("open GFF");
    println!(
        "  Header: type={}, version={}",
        parser.file_type, parser.file_version
    );

    let root = parser.read_struct_fields(0).expect("root struct");
    let mut profile = Profile::new();
    profile.total_bytes = profile_struct(&parser, &root, "", &mut profile, 0, max_depth);

    let mut byte_entries: Vec<(&str, u64)> = profile
        .entries
        .iter()
        .filter_map(|e| match e {
            ProfileEntry::Bytes { path, bytes } => Some((path.as_str(), *bytes)),
            _ => None,
        })
        .collect();
    byte_entries.sort_by_key(|(_, bytes)| std::cmp::Reverse(*bytes));

    let mut list_entries: Vec<(&str, u64)> = profile
        .entries
        .iter()
        .filter_map(|e| match e {
            ProfileEntry::ListLen { path, count } => Some((path.as_str(), *count)),
            _ => None,
        })
        .collect();
    list_entries.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    println!(
        "\n  Total content bytes walked: {} ({:.1} MB)",
        profile.total_bytes,
        profile.total_bytes as f64 / 1_048_576.0
    );

    println!("\n  TOP 30 HEAVIEST PATHS (bytes):");
    for (path, bytes) in byte_entries.iter().take(30) {
        println!("    {bytes:>12} bytes  {path}");
    }

    println!("\n  TOP 30 LARGEST LISTS (element count):");
    for (path, count) in list_entries.iter().take(30) {
        println!("    {count:>12} items  {path}");
    }
}

/// Bloated save fixture - 908 MB BIC/IFO extracted from a heavily-re-saved
/// character. Not checked in (fixtures dir is gitignored); copy the raw
/// `player.bic` and `playerlist.ifo` from an affected save zip here to run.
fn bloated_save_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/bloat_debug/000071")
}

#[test]
#[ignore = "depends on local fixture tests/fixtures/saves/bloat_debug/000071"]
fn profile_bloated_playerlist_ifo() {
    let path = bloated_save_dir().join("playerlist.ifo");
    profile_gff_file(&path, "playerlist.ifo (bloated)", 8);
}

#[test]
#[ignore = "depends on local fixture tests/fixtures/saves/bloat_debug/000071"]
fn profile_bloated_player_bic() {
    let path = bloated_save_dir().join("player.bic");
    profile_gff_file(&path, "player.bic (bloated)", 8);
}

fn find_repeating_period(bytes: &[u8]) -> Option<usize> {
    for &period in &[1usize, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024] {
        if bytes.len() <= period * 4 {
            continue;
        }
        let head = &bytes[..period];
        let repeats = bytes
            .chunks(period)
            .take(4)
            .all(|c| c == &head[..period.min(c.len())]);
        if repeats {
            return Some(period);
        }
    }
    None
}

fn dump_substring_patterns(bytes: &[u8]) {
    let len = bytes.len();
    let head = &bytes[..200.min(len)];
    println!("    HEAD: {:?}", String::from_utf8_lossy(head));
    if len > 400 {
        let tail = &bytes[len.saturating_sub(200)..];
        println!("    TAIL: {:?}", String::from_utf8_lossy(tail));
    }
    if len <= 4096 {
        return;
    }

    let chunk = 256;
    let first = &bytes[..chunk];
    let mid_start = len / 2;
    let middle = &bytes[mid_start..(mid_start + chunk).min(len)];
    let last = &bytes[len.saturating_sub(chunk)..];

    let all_same = bytes.iter().all(|&b| b == bytes[0]);
    println!(
        "    PATTERN CHECK: all_same_byte={all_same} first==middle={} first==last={}",
        first == middle,
        first == last,
    );
    let first_byte = bytes[0];
    let glyph = if first_byte.is_ascii_graphic() {
        first_byte as char
    } else {
        '.'
    };
    println!("    First byte value: 0x{first_byte:02X} ('{glyph}')");

    if !all_same && let Some(period) = find_repeating_period(bytes) {
        println!("    Repeating period of {period} bytes detected");
    }
}

#[test]
#[ignore = "depends on local fixture tests/fixtures/saves/bloat_debug/000071"]
fn inspect_bloated_description() {
    let path = bloated_save_dir().join("player.bic");
    let parser = GffParser::new(&path).expect("open GFF");
    let root = parser.read_struct_fields(0).expect("root struct");

    let desc = root.get("Description").expect("Description field");
    match desc {
        GffValue::LocString(ls) => {
            println!(
                "Description is LocString, string_ref={}, {} substrings",
                ls.string_ref,
                ls.substrings.len()
            );
            for (i, sub) in ls.substrings.iter().enumerate() {
                let bytes = sub.string.as_bytes();
                let len = bytes.len();
                println!(
                    "  substring[{i}]: lang={} gender={} len={} ({:.1} MB)",
                    sub.language,
                    sub.gender,
                    len,
                    len as f64 / 1_048_576.0
                );
                dump_substring_patterns(bytes);
            }
        }
        GffValue::String(s) => {
            println!("Description is String, len={}", s.len());
        }
        other => {
            println!("Description is unexpected type: {}", gff_type_name(other));
        }
    }
}
