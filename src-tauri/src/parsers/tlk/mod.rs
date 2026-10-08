pub mod error;
pub mod parser;
pub mod types;

pub use types::TLKParser;

use std::sync::{PoisonError, RwLock};

/// Strrefs at or above this address the module's custom TLK
/// (`Mod_CustomTlk`), row `strref - CUSTOM_TLK_BASE`.
pub const CUSTOM_TLK_BASE: i32 = 0x0100_0000;

/// Resolves a strref the way the engine does: below `CUSTOM_TLK_BASE` from
/// `dialog.tlk`, above it from the custom TLK. `None` for negative or
/// unresolvable strrefs.
pub fn resolve_strref(
    base: Option<&RwLock<TLKParser>>,
    custom: Option<&RwLock<TLKParser>>,
    str_ref: i32,
) -> Option<String> {
    if str_ref < 0 {
        return None;
    }
    let (table, index) = if str_ref >= CUSTOM_TLK_BASE {
        (custom?, str_ref - CUSTOM_TLK_BASE)
    } else {
        (base?, str_ref)
    };
    let tlk = table.read().unwrap_or_else(PoisonError::into_inner);
    tlk.read_string(index as usize)
}
