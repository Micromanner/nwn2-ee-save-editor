//! Read the human-readable `Mod_Name` from a `.mod` file's `module.ifo`.
//!
//! `ErfParser::read` mmaps the file and only pages in what gets touched, so the
//! cost is the header + key/resource list + one small resource (`module.ifo`).
//! TLK strref resolution is intentionally omitted; the first LocString
//! substring is sufficient for OC/MotB/SoZ and standard custom campaigns.

use std::path::Path;

use crate::parsers::ErfParser;
use crate::parsers::gff::{GffParser, GffValue};

pub fn read_module_display_name(mod_path: &Path) -> Option<String> {
    let mut erf = ErfParser::new();
    erf.read(mod_path).ok()?;
    let ifo_bytes = erf.get_module_info().ok().flatten()?;

    let gff = GffParser::from_bytes(ifo_bytes).ok()?;
    let root = gff.read_struct_fields(0).ok()?;

    let raw = match root.get("Mod_Name")? {
        GffValue::LocString(ls) => ls.substrings.first().map(|s| s.string.to_string()),
        GffValue::String(s) | GffValue::ResRef(s) => Some(s.to_string()),
        _ => None,
    }?;

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
