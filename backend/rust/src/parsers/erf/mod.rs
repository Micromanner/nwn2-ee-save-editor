pub mod error;
pub mod parser;
pub mod types;

#[cfg(feature = "python-bindings")]
pub mod python;

pub use error::{ErfError, ErfResult};
pub use types::SecurityLimits;
pub use parser::ErfParser;
pub use types::{
    resource_type_to_extension, ErfHeader, ErfResource, ErfStatistics, ErfType, ErfVersion,
    FileMetadata, KeyEntry, ResourceEntry,
};

#[cfg(feature = "python-bindings")]
pub use python::PyErfParser;
