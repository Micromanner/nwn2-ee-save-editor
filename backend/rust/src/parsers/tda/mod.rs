pub mod error;
pub mod parser;
pub mod tokenizer;
pub mod types;

#[cfg(feature = "python-bindings")]
pub mod python;

pub use error::{SecurityLimits, TDAError, TDAResult};
pub use parser::{load_multiple_files, ParserStatistics};
pub use tokenizer::TDATokenizer;
pub use types::{CellValue, SerializableCellValue, SerializableTDAParser, TDAParser};

#[cfg(feature = "python-bindings")]
pub use python::PyTDAParser;
