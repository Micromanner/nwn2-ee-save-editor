#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]

pub mod error;
pub mod prelude;
pub mod types;

pub mod parsers;
pub mod services;
pub mod utils;

pub use error::CommonError;
pub use types::{FileMetadata, ParserStats, SecurityLimits};

pub use parsers::{ErfParser, RustXmlParser, TDAParser, TLKParser};
pub use services::RustIconCache;
pub use utils::{
    CacheBuilder, CacheManager, DirectoryWalker, DiscoveryResult, PathTiming, PrerequisiteGraph,
    ResourceLocation, ResourceScanner, ScanResults, ZipContentReader, ZipIndexer, ZipReadRequest,
    ZipReadResult,
};

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

#[cfg(feature = "python-bindings")]
use parsers::{PyErfParser, PyTDAParser, PyTLKParser, PyXmlParser};

#[cfg(feature = "python-bindings")]
use services::PyRustIconCache;

#[cfg(feature = "python-bindings")]
use utils::{discover_nwn2_paths_rust, profile_path_discovery_rust};

#[cfg(feature = "python-bindings")]
#[pymodule]
fn nwn2_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = env_logger::try_init();

    m.add_class::<PyTDAParser>()?;
    m.add_class::<PyTLKParser>()?;
    m.add_class::<PyErfParser>()?;
    m.add_class::<PyXmlParser>()?;

    m.add_class::<PyRustIconCache>()?;

    m.add_class::<ResourceScanner>()?;
    m.add_class::<ResourceLocation>()?;
    m.add_class::<ScanResults>()?;
    m.add_function(wrap_pyfunction!(discover_nwn2_paths_rust, m)?)?;
    m.add_function(wrap_pyfunction!(profile_path_discovery_rust, m)?)?;
    m.add_class::<DiscoveryResult>()?;
    m.add_class::<PathTiming>()?;
    m.add_class::<PrerequisiteGraph>()?;
    m.add_class::<CacheBuilder>()?;
    m.add_class::<CacheManager>()?;
    m.add_class::<ZipContentReader>()?;
    m.add_class::<ZipReadRequest>()?;
    m.add_class::<ZipReadResult>()?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("FORMAT_WEBP", "image/webp")?;

    Ok(())
}
