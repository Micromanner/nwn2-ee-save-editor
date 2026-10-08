//! Door locks and map pins in a module's saved area state. `<module>.z` keeps one
//! `.git` per area as a diff against the module's own `.git`: an entry points at its
//! module counterpart through `DIFF_INDEX` and carries only the fields that differ,
//! so a door without `Locked` still has the module's value. Objects spawned at
//! runtime are full entries with their own `Tag`.

use std::collections::HashMap;
use std::path::Path;

use indexmap::IndexMap;

use super::script_source::open_module_archive;
use crate::parsers::erf::{ErfParser, RES_TYPE_GIT};
use crate::parsers::gff::{
    GffParser, GffValue, gff_integer, gff_text, insert_bool_preserving_type, rewrite_gff,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AreaField {
    DoorLocked,
    MapNoteEnabled,
}

impl AreaField {
    fn list(self) -> &'static str {
        match self {
            Self::DoorLocked => "Door List",
            Self::MapNoteEnabled => "WaypointList",
        }
    }

    fn field(self) -> &'static str {
        match self {
            Self::DoorLocked => "Locked",
            Self::MapNoteEnabled => "MapNoteEnabled",
        }
    }
}

/// One door or map-pin waypoint of the installed module.
#[derive(Debug, Clone)]
pub struct ModuleObject {
    /// Lowercase `.git` resource name, the same in the module and in `<module>.z`.
    area: String,
    /// Position in the module's list, what the save's `DIFF_INDEX` refers to.
    index: usize,
    value: i64,
    pub label_strref: Option<i32>,
}

/// Doors and map-pin waypoints of a module by `(field, lowercase tag)`.
#[derive(Debug, Default)]
pub struct ModuleObjects(HashMap<(AreaField, String), Vec<ModuleObject>>);

impl ModuleObjects {
    /// The one object carrying `tag`; several (or none) cannot be told apart.
    pub fn unique(&self, field: AreaField, tag: &str) -> Result<&ModuleObject, String> {
        match self
            .0
            .get(&(field, tag.to_ascii_lowercase()))
            .map(Vec::as_slice)
        {
            Some([object]) => Ok(object),
            Some(_) => Err(format!("Several objects tagged {tag} in the module")),
            None => Err(format!("No object tagged {tag} in the module")),
        }
    }
}

/// Doors and map-pin waypoints (`HasMapNote`) of every area of an installed module,
/// a `.mod` file or an expanded module folder.
pub fn read_module_objects(module_path: &Path) -> ModuleObjects {
    let mut objects = ModuleObjects::default();
    if module_path.is_dir() {
        let Ok(entries) = std::fs::read_dir(module_path) else {
            return objects;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.ends_with(".git")
                && let Ok(bytes) = std::fs::read(entry.path())
            {
                index_module_git(&mut objects, &name, bytes);
            }
        }
    } else if let Some(mut parser) = open_module_archive(module_path) {
        for (name, _, _) in parser.list_resources(Some(RES_TYPE_GIT)) {
            if let Ok(bytes) = parser.extract_resource(&name) {
                index_module_git(&mut objects, &name, bytes);
            }
        }
    }
    objects
}

fn index_module_git(objects: &mut ModuleObjects, area: &str, bytes: Vec<u8>) {
    let Ok(gff) = GffParser::from_bytes(bytes) else {
        return;
    };
    let Ok(root) = gff.read_struct_fields(0) else {
        return;
    };
    for field in [AreaField::DoorLocked, AreaField::MapNoteEnabled] {
        let Some(GffValue::List(entries)) = root.get(field.list()) else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let fields = match entry.force_load() {
                Ok(fields) => fields,
                Err(e) => {
                    tracing::warn!("Skipping unreadable {} entry in {area}: {e}", field.list());
                    continue;
                }
            };
            if field == AreaField::MapNoteEnabled
                && fields.get("HasMapNote").and_then(gff_integer) != Some(1)
            {
                continue;
            }
            let Some(tag) = gff_text(&fields, "Tag").filter(|t| !t.is_empty()) else {
                continue;
            };
            let label_strref = ["MapNote", "LocName", "LocalizedName"]
                .iter()
                .filter(|name| field == AreaField::MapNoteEnabled || **name != "MapNote")
                .find_map(|name| match fields.get(*name) {
                    Some(GffValue::LocString(ls)) if ls.string_ref >= 0 => Some(ls.string_ref),
                    _ => None,
                });
            objects
                .0
                .entry((field, tag.to_ascii_lowercase()))
                .or_default()
                .push(ModuleObject {
                    area: area.to_ascii_lowercase(),
                    index,
                    value: fields.get(field.field()).and_then(gff_integer).unwrap_or(0),
                    label_strref,
                });
        }
    }
}

/// The object's value in the save: the entry's own field, else the module's.
pub fn read_saved(
    save: &mut ErfParser,
    field: AreaField,
    tag: &str,
    object: &ModuleObject,
) -> Result<i64, String> {
    let bytes = save
        .extract_resource(&object.area)
        .map_err(|e| format!("{} missing from the save: {e}", object.area))?;
    saved_value(bytes, field, tag, object)
}

fn saved_value(
    bytes: Vec<u8>,
    field: AreaField,
    tag: &str,
    object: &ModuleObject,
) -> Result<i64, String> {
    let gff = GffParser::from_bytes(bytes).map_err(|e| e.to_string())?;
    let root = gff.read_struct_fields(0).map_err(|e| e.to_string())?;
    let Some(GffValue::List(entries)) = root.get(field.list()) else {
        return Err(format!("{} has no {}", object.area, field.list()));
    };
    let entries: Vec<_> = entries
        .iter()
        .map(|e| e.force_load().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    Ok(entries[saved_position(&entries, tag, object)?]
        .get(field.field())
        .and_then(gff_integer)
        .unwrap_or(object.value))
}

/// Set `field` on each tagged object in the save's area `.git`s, keeping the stored
/// variant of the field (BYTE on doors and waypoints) and the file's header.
pub fn write_saved(
    save: &mut ErfParser,
    objects: &ModuleObjects,
    edits: &[(AreaField, String, bool)],
) -> Result<(), String> {
    let mut by_area: IndexMap<&str, Vec<(AreaField, &str, &ModuleObject, bool)>> = IndexMap::new();
    for (field, tag, value) in edits {
        let object = objects.unique(*field, tag)?;
        by_area
            .entry(object.area.as_str())
            .or_default()
            .push((*field, tag, object, *value));
    }
    for (area, edits) in by_area {
        let bytes = save
            .extract_resource(area)
            .map_err(|e| format!("{area} missing from the save: {e}"))?;
        let bytes = edit_git(bytes, &edits)?;
        save.update_resource(area, bytes)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn edit_git(
    bytes: Vec<u8>,
    edits: &[(AreaField, &str, &ModuleObject, bool)],
) -> Result<Vec<u8>, String> {
    rewrite_gff(bytes, "area state", |root| {
        for &(field, tag, object, value) in edits {
            let Some(GffValue::ListOwned(entries)) = root.get_mut(field.list()) else {
                return Err(format!("{} has no {}", object.area, field.list()));
            };
            let position = saved_position(entries, tag, object)?;
            insert_bool_preserving_type(&mut entries[position], field.field(), value);
        }
        Ok(())
    })
}

/// The save entry of a module object: a diff entry pointing at its module index, or
/// (full-copy GITs) an entry carrying its tag. A runtime copy with the same tag
/// makes the object ambiguous; no entry means it was destroyed.
fn saved_position(
    entries: &[IndexMap<String, GffValue<'_>>],
    tag: &str,
    object: &ModuleObject,
) -> Result<usize, String> {
    let matches: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| match gff_text(entry, "Tag") {
            Some(own) => own.eq_ignore_ascii_case(tag),
            None => {
                entry.get("DIFF_INDEX").and_then(gff_integer) == i64::try_from(object.index).ok()
            }
        })
        .map(|(i, _)| i)
        .collect();
    match matches.as_slice() {
        [position] => Ok(*position),
        [] => Err(format!("{tag} is no longer in the saved area")),
        _ => Err(format!("Several saved objects match {tag}")),
    }
}

/// `read_module_objects` of the module a save's `<module>.z` belongs to.
pub fn installed_module_objects(
    paths: &crate::config::NWN2Paths,
    module: &str,
) -> Result<ModuleObjects, String> {
    let module_path =
        crate::services::resource_manager::module_loader::find_installed_module(paths, module)
            .ok_or_else(|| format!("Module {module} not installed"))?;
    Ok(read_module_objects(&module_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffWriter;

    fn entry(
        struct_id: u32,
        fields: Vec<(&str, GffValue<'static>)>,
    ) -> IndexMap<String, GffValue<'static>> {
        let mut entry = IndexMap::from([("__struct_id__".to_string(), GffValue::Dword(struct_id))]);
        entry.extend(fields.into_iter().map(|(k, v)| (k.to_string(), v)));
        entry
    }

    fn tag(value: &str) -> GffValue<'static> {
        GffValue::String(value.into())
    }

    fn git(fields: Vec<(&str, GffValue<'static>)>) -> Vec<u8> {
        let root: IndexMap<String, GffValue<'static>> = fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        GffWriter::new("GIT ", "V3.2")
            .write_with_struct_id(root, 0xFFFF_FFFF)
            .unwrap()
    }

    #[test]
    fn diff_entries_fall_back_to_the_module_and_take_edits_as_bytes() {
        let module = git(vec![
            (
                "Door List",
                GffValue::ListOwned(vec![
                    entry(
                        8,
                        vec![("Tag", tag("other")), ("Locked", GffValue::Byte(0))],
                    ),
                    entry(
                        8,
                        vec![
                            ("Tag", tag("10_Hagen_Door_Ext")),
                            ("Locked", GffValue::Byte(1)),
                        ],
                    ),
                ]),
            ),
            (
                "WaypointList",
                GffValue::ListOwned(vec![
                    entry(
                        5,
                        vec![("Tag", tag("plain")), ("HasMapNote", GffValue::Byte(0))],
                    ),
                    entry(
                        5,
                        vec![
                            ("Tag", tag("1000_mn_hagen")),
                            ("HasMapNote", GffValue::Byte(1)),
                            ("MapNoteEnabled", GffValue::Byte(0)),
                        ],
                    ),
                ]),
            ),
        ]);
        // Save diff: the door keeps the module's lock, a runtime waypoint spawned
        // with DIFF_INDEX 1 carries its own tag and must not be mistaken for the pin.
        let save = git(vec![
            ("DIFF", GffValue::Dword(0)),
            (
                "Door List",
                GffValue::ListOwned(vec![
                    entry(
                        8,
                        vec![
                            ("DIFF_INDEX", GffValue::Dword(0)),
                            ("Locked", GffValue::Byte(1)),
                        ],
                    ),
                    entry(
                        8,
                        vec![
                            ("DIFF_INDEX", GffValue::Dword(1)),
                            ("ObjectId", GffValue::Dword(85)),
                        ],
                    ),
                ]),
            ),
            (
                "WaypointList",
                GffValue::ListOwned(vec![
                    entry(5, vec![("DIFF_INDEX", GffValue::Dword(0))]),
                    entry(5, vec![("DIFF_INDEX", GffValue::Dword(1))]),
                    entry(
                        5,
                        vec![
                            ("DIFF_INDEX", GffValue::Dword(1)),
                            ("Tag", tag("wc_spawned")),
                        ],
                    ),
                ]),
            ),
        ]);
        let mut objects = ModuleObjects::default();
        index_module_git(&mut objects, "{1000}the docks.git", module);
        assert!(objects.unique(AreaField::MapNoteEnabled, "plain").is_err());
        let door = objects
            .unique(AreaField::DoorLocked, "10_hagen_door_ext")
            .unwrap();
        let pin = objects
            .unique(AreaField::MapNoteEnabled, "1000_mn_hagen")
            .unwrap();
        assert_eq!(
            saved_value(
                save.clone(),
                AreaField::DoorLocked,
                "10_hagen_door_ext",
                door
            ),
            Ok(1)
        );
        assert_eq!(
            saved_value(
                save.clone(),
                AreaField::MapNoteEnabled,
                "1000_mn_hagen",
                pin
            ),
            Ok(0)
        );

        let edited = edit_git(
            save,
            &[
                (AreaField::DoorLocked, "10_hagen_door_ext", door, false),
                (AreaField::MapNoteEnabled, "1000_mn_hagen", pin, true),
            ],
        )
        .unwrap();
        assert_eq!(
            saved_value(
                edited.clone(),
                AreaField::DoorLocked,
                "10_hagen_door_ext",
                door
            ),
            Ok(0)
        );
        assert_eq!(
            saved_value(
                edited.clone(),
                AreaField::MapNoteEnabled,
                "1000_mn_hagen",
                pin
            ),
            Ok(1)
        );

        let gff = GffParser::from_bytes(edited).unwrap();
        assert_eq!(gff.file_type, "GIT ");
        assert_eq!(gff.get_struct_id(0).unwrap(), 0xFFFF_FFFF);
        let root = gff.read_struct_fields(0).unwrap();
        let Some(GffValue::List(doors)) = root.get("Door List") else {
            panic!("Door List missing");
        };
        let hagen = doors[1].force_load().unwrap();
        assert!(matches!(hagen.get("Locked"), Some(GffValue::Byte(0))));
        assert!(matches!(hagen.get("ObjectId"), Some(GffValue::Dword(85))));
        assert!(matches!(
            hagen.get("__struct_id__"),
            Some(GffValue::Dword(8))
        ));
        assert!(matches!(root.get("DIFF"), Some(GffValue::Dword(0))));
    }
}
