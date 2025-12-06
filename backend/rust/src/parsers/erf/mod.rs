pub mod error;
pub mod parser;
pub mod types;

#[cfg(feature = "python-bindings")]
pub mod python;

pub use error::{ErfError, ErfResult};
pub use types::SecurityLimits;
pub use parser::ErfParser;
pub use types::{
    extension_to_resource_type, resource_type_to_extension, ErfBuilder, ErfHeader, ErfResource,
    ErfStatistics, ErfType, ErfVersion, FileMetadata, KeyEntry, ResourceEntry,
};

#[cfg(feature = "python-bindings")]
pub use python::PyErfParser;
