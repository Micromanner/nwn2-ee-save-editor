pub mod icon_cache;

pub use icon_cache::RustIconCache;

#[cfg(feature = "python-bindings")]
pub use icon_cache::PyRustIconCache;
