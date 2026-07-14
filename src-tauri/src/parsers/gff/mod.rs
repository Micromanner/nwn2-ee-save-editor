pub mod error;
pub mod helpers;
mod merge;
pub mod parser;
pub mod types;
pub mod writer;

pub use error::GffError;
pub use helpers::{
    insert_bool_preserving_type, insert_i32_preserving_type, insert_u32_preserving_type,
    variant_name,
};
pub use merge::merge_fields_into_gff;
pub use parser::GffParser;
pub use types::{GffFieldType, GffValue, LazyStruct, LocalizedString, LocalizedSubstring};
pub use writer::GffWriter;
