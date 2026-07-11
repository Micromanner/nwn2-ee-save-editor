use indexmap::IndexMap;

use super::{GffParser, GffValue, GffWriter};

/// Merge edited root fields into an existing GFF file's bytes: overwrite keys
/// the file already has, preserve file-only keys, keep the original file
/// type/version/root struct id. `original: None` writes the fields fresh.
///
/// `insert_new_keys` controls how `fields` entries absent from the original
/// are handled: `false` drops them (playerlist.ifo stays authoritative for
/// the player .bic); `true` appends them (the .ros is the sole artifact for
/// companions, so editor-created keys like `LvlStatList` must survive).
/// Either way, `__`-prefixed keys are always skipped.
pub fn merge_fields_into_gff(
    original: Option<&[u8]>,
    fields: &IndexMap<String, GffValue<'static>>,
    default_file_type: &str,
    insert_new_keys: bool,
) -> Result<Vec<u8>, String> {
    let Some(original) = original else {
        return GffWriter::new(default_file_type, "V3.2")
            .write(fields.clone())
            .map_err(|e| format!("{default_file_type}serialization error: {e}"));
    };

    let gff =
        GffParser::from_bytes(original.to_vec()).map_err(|e| format!("GFF parse error: {e}"))?;
    let file_type = gff.file_type.clone();
    let file_version = gff.file_version.clone();
    let root_struct_id = gff
        .get_struct_id(0)
        .map_err(|e| format!("Failed to read root struct_id: {e}"))?;
    let existing = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read root fields: {e}"))?;

    let mut merged: IndexMap<String, GffValue<'static>> = existing
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    for (key, value) in fields {
        if key.starts_with("__") {
            continue;
        }
        if merged.contains_key(key) || insert_new_keys {
            merged.insert(key.clone(), value.clone());
        }
    }

    GffWriter::new(&file_type, &file_version)
        .write_with_struct_id(merged, root_struct_id)
        .map_err(|e| format!("GFF serialization error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::{GffParser, GffValue, GffWriter};
    use indexmap::IndexMap;

    fn base_file() -> Vec<u8> {
        let mut root: IndexMap<String, GffValue<'static>> = IndexMap::new();
        root.insert("Str".into(), GffValue::Byte(14));
        root.insert("OnlyInFile".into(), GffValue::Int(7));
        GffWriter::new("ROS ", "V3.2").write(root).expect("write")
    }

    #[test]
    fn merge_overwrites_shared_and_preserves_file_only_keys() {
        let original = base_file();
        let mut edited: IndexMap<String, GffValue<'static>> = IndexMap::new();
        edited.insert("Str".into(), GffValue::Byte(18));
        edited.insert("__struct_id__".into(), GffValue::Dword(0));
        edited.insert("NotInFile".into(), GffValue::Int(99));

        let merged = merge_fields_into_gff(Some(&original), &edited, "ROS ", false).expect("merge");
        let parser = GffParser::from_bytes(merged).expect("parse");
        assert_eq!(parser.file_type, "ROS ");
        let root = parser.read_struct_fields(0).expect("root");
        assert!(matches!(root.get("Str"), Some(GffValue::Byte(18))));
        assert!(matches!(root.get("OnlyInFile"), Some(GffValue::Int(7))));
        assert!(root.get("NotInFile").is_none());
    }

    #[test]
    fn merge_with_insert_new_keys_adds_missing_fields() {
        let original = base_file();
        let mut edited: IndexMap<String, GffValue<'static>> = IndexMap::new();
        edited.insert("Str".into(), GffValue::Byte(18));
        edited.insert("__struct_id__".into(), GffValue::Dword(0));
        edited.insert("NotInFile".into(), GffValue::Int(99));

        let merged = merge_fields_into_gff(Some(&original), &edited, "ROS ", true).expect("merge");
        let parser = GffParser::from_bytes(merged).expect("parse");
        let root = parser.read_struct_fields(0).expect("root");
        assert!(matches!(root.get("Str"), Some(GffValue::Byte(18))));
        assert!(matches!(root.get("OnlyInFile"), Some(GffValue::Int(7))));
        assert!(matches!(root.get("NotInFile"), Some(GffValue::Int(99))));
        assert!(root.get("__struct_id__").is_none());
    }

    #[test]
    fn merge_without_original_writes_fresh_file() {
        let mut fields: IndexMap<String, GffValue<'static>> = IndexMap::new();
        fields.insert("Str".into(), GffValue::Byte(12));
        let bytes = merge_fields_into_gff(None, &fields, "BIC ", false).expect("merge");
        let parser = GffParser::from_bytes(bytes).expect("parse");
        assert_eq!(parser.file_type, "BIC ");
    }
}
