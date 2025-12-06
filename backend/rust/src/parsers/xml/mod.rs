pub mod parser;
pub mod types;

#[cfg(feature = "python-bindings")]
pub mod python;

pub use parser::{CompanionDefinition, CompanionStatus, FullSummary, QuestGroup, QuestOverview, RustXmlParser};
pub use types::{Vector3, XmlData};

#[cfg(feature = "python-bindings")]
pub use python::PyXmlParser;
