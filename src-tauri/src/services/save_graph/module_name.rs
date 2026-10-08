//! Read the human-readable `Mod_Name` from a `.mod` file's `module.ifo`.
//!
//! `ErfParser::read` mmaps the file and only pages in what gets touched, so the
//! cost is the header + key/resource list + one small resource (`module.ifo`).
//! TLK strref resolution is intentionally omitted; the first LocString
//! substring is sufficient for OC/MotB/SoZ and standard custom campaigns.

use crate::parsers::ErfParser;
use crate::parsers::gff::{GffParser, GffValue};

/// `Mod_Name` of an opened module archive (`script_source::open_module_archive`).
pub fn read_module_display_name(erf: &mut ErfParser) -> Option<String> {
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
