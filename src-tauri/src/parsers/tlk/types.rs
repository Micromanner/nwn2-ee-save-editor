#[derive(Debug, Clone)]
pub struct TLKStringEntry {
    /// Bit 0 set when the entry has text.
    pub flags: u32,
    /// Offset within the string data section
    pub data_offset: u32,
    pub string_size: u32,
}

impl TLKStringEntry {
    pub fn is_present(&self) -> bool {
        (self.flags & 0x01) != 0
    }
}

#[derive(Debug, Default)]
pub struct TLKParser {
    /// NWN language id from the header, selects the legacy code page.
    pub(super) language_id: u32,
    pub(super) entries: Vec<TLKStringEntry>,
    pub(super) string_data: Vec<u8>,
}

impl TLKParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn string_count(&self) -> usize {
        self.entries.len()
    }
}
