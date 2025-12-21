use crate::define_parser_error;


define_parser_error! {
    GffError {
        InvalidHeader(String) => "Invalid header: {0}",
        InvalidVersion(String) => "Invalid version: {0}",
        InvalidStructIndex(u32) => "Invalid struct index: {0}",
        InvalidFieldIndex(u32) => "Invalid field index: {0}",
        InvalidLabelIndex(u32) => "Invalid label index: {0}",
        FieldNotFound(String) => "Field not found: {0}",
        UnsupportedFieldType(u32) => "Unsupported field type: {0}",
        BufferOverflow(String) => "Buffer overflow: {0}",
    }
}
