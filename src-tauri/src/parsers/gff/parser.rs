use std::borrow::Cow;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use byteorder::{ByteOrder, LittleEndian};
use indexmap::IndexMap;
use memmap2::Mmap;
use tracing::{debug, instrument, trace};

use crate::utils::text_encoding::decode_text;

use super::error::GffError;
use super::types::{
    GffText, GffValue, LazyStruct, LocalizedString, LocalizedSubstring, force_owned_map,
};

const HEADER_SIZE: usize = 56;
const LABEL_SIZE: usize = 16;
const FIELD_SIZE: usize = 12;
const STRUCT_SIZE: usize = 12;

enum DataSource {
    Mmap(Mmap),
    Bytes(Vec<u8>),
}

impl DataSource {
    fn as_slice(&self) -> &[u8] {
        match self {
            DataSource::Mmap(m) => &m[..],
            DataSource::Bytes(v) => &v[..],
        }
    }

    fn len(&self) -> usize {
        match self {
            DataSource::Mmap(m) => m.len(),
            DataSource::Bytes(v) => v.len(),
        }
    }
}

impl std::fmt::Debug for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataSource::Mmap(_) => write!(f, "DataSource::Mmap(...)"),
            DataSource::Bytes(v) => write!(f, "DataSource::Bytes({} bytes)", v.len()),
        }
    }
}

/// Every section range is checked against the file once in `parse_header`;
/// reads are then bounded by their section, so an index below its count is
/// always in bounds.
#[derive(Debug)]
pub struct GffParser {
    data: Arc<DataSource>,

    pub file_type: String,
    pub file_version: String,

    struct_offset: usize,
    struct_count: u32,
    field_offset: usize,
    field_count: u32,
    label_offset: usize,
    label_count: u32,
    field_data_offset: usize,
    field_data_len: usize,
    field_indices_offset: usize,
    field_indices_len: usize,
    list_indices_offset: usize,
    list_indices_len: usize,
}

impl GffParser {
    #[instrument(name = "GffParser::new", skip_all, fields(path = ?path.as_ref()))]
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Arc<Self>, GffError> {
        trace!("Opening GFF file");
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let file_size = mmap.len();
        trace!("Memory-mapped GFF file ({} bytes)", file_size);

        let data = Arc::new(DataSource::Mmap(mmap));

        let parser = Self::parse_header(data)?;
        debug!(
            "GFF file parsed: type={}, version={}, structs={}, fields={}",
            parser.file_type, parser.file_version, parser.struct_count, parser.field_count
        );
        Ok(Arc::new(parser))
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Arc<Self>, GffError> {
        let data = Arc::new(DataSource::Bytes(bytes));
        let parser = Self::parse_header(data)?;
        Ok(Arc::new(parser))
    }

    fn parse_header(data: Arc<DataSource>) -> Result<Self, GffError> {
        let slice = data.as_slice();
        if data.len() < HEADER_SIZE {
            return Err(GffError::InvalidHeader("File too small".to_string()));
        }

        let struct_offset = LittleEndian::read_u32(&slice[8..12]) as usize;
        let struct_count = LittleEndian::read_u32(&slice[12..16]);
        let field_offset = LittleEndian::read_u32(&slice[16..20]) as usize;
        let field_count = LittleEndian::read_u32(&slice[20..24]);
        let label_offset = LittleEndian::read_u32(&slice[24..28]) as usize;
        let label_count = LittleEndian::read_u32(&slice[28..32]);
        let field_data_offset = LittleEndian::read_u32(&slice[32..36]) as usize;
        let field_data_len = LittleEndian::read_u32(&slice[36..40]) as usize;
        let field_indices_offset = LittleEndian::read_u32(&slice[40..44]) as usize;
        let field_indices_len = LittleEndian::read_u32(&slice[44..48]) as usize;
        let list_indices_offset = LittleEndian::read_u32(&slice[48..52]) as usize;
        let list_indices_len = LittleEndian::read_u32(&slice[52..56]) as usize;

        let sections = [
            (
                "Struct array",
                struct_offset,
                struct_count as usize * STRUCT_SIZE,
            ),
            (
                "Field array",
                field_offset,
                field_count as usize * FIELD_SIZE,
            ),
            (
                "Label array",
                label_offset,
                label_count as usize * LABEL_SIZE,
            ),
            ("Field data", field_data_offset, field_data_len),
            ("Field indices", field_indices_offset, field_indices_len),
            ("List indices", list_indices_offset, list_indices_len),
        ];
        for (name, offset, len) in sections {
            if offset.checked_add(len).is_none_or(|end| end > data.len()) {
                return Err(GffError::BufferOverflow(format!(
                    "{name} {offset}+{len} exceeds file size {}",
                    data.len()
                )));
            }
        }

        let file_type_bytes = &slice[0..4];
        let file_ver_bytes = &slice[4..8];
        let file_type = String::from_utf8_lossy(file_type_bytes).to_string();
        let file_version = String::from_utf8_lossy(file_ver_bytes).to_string();

        Ok(GffParser {
            data,
            file_type,
            file_version,
            struct_offset,
            struct_count,
            field_offset,
            field_count,
            label_offset,
            label_count,
            field_data_offset,
            field_data_len,
            field_indices_offset,
            field_indices_len,
            list_indices_offset,
            list_indices_len,
        })
    }

    pub(crate) fn get_label<'a>(&self, index: u32) -> Result<Cow<'a, str>, GffError> {
        if index >= self.label_count {
            return Err(GffError::InvalidLabelIndex(index));
        }
        let offset = self.label_offset + (index as usize * LABEL_SIZE);
        let bytes = &self.data.as_slice()[offset..offset + LABEL_SIZE];
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(LABEL_SIZE);
        let label_bytes = &bytes[..len];

        Ok(Cow::Owned(decode_text(label_bytes)))
    }

    pub fn read_struct_fields<'a>(
        self: &Arc<Self>,
        struct_index: u32,
    ) -> Result<IndexMap<String, GffValue<'a>>, GffError> {
        let field_indices = self.struct_field_indices(struct_index)?;
        let mut map = IndexMap::with_capacity(field_indices.len());
        for field_idx in field_indices {
            let (label, value) = self.read_field(field_idx)?;
            map.insert(label, value);
        }
        Ok(map)
    }

    /// `read_struct_fields` with every nested struct and list loaded.
    pub fn read_owned_struct(
        self: &Arc<Self>,
        struct_index: u32,
    ) -> Result<IndexMap<String, GffValue<'static>>, GffError> {
        force_owned_map(self.read_struct_fields(struct_index)?)
    }

    fn struct_field_indices(&self, struct_index: u32) -> Result<Vec<u32>, GffError> {
        if struct_index >= self.struct_count {
            return Err(GffError::InvalidStructIndex(struct_index));
        }

        let slice = self.data.as_slice();
        let offset = self.struct_offset + (struct_index as usize * STRUCT_SIZE);
        let field_data_or_index = LittleEndian::read_u32(&slice[offset + 4..offset + 8]);
        let field_count = LittleEndian::read_u32(&slice[offset + 8..offset + 12]) as usize;

        match field_count {
            0 => Ok(Vec::new()),
            1 => Ok(vec![field_data_or_index]),
            _ => {
                let len = field_count
                    .checked_mul(4)
                    .ok_or_else(|| GffError::BufferOverflow("Field indices".into()))?;
                Ok(self
                    .section_slice(
                        self.field_indices_offset,
                        self.field_indices_len,
                        field_data_or_index as usize,
                        len,
                        "Field indices",
                    )?
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|chunk| u32::from_le_bytes(*chunk))
                    .collect())
            }
        }
    }

    /// (type, label index, data or offset) of a field array entry.
    fn field_entry(&self, field_index: u32) -> Result<(u32, u32, u32), GffError> {
        if field_index >= self.field_count {
            return Err(GffError::InvalidFieldIndex(field_index));
        }
        let offset = self.field_offset + (field_index as usize * FIELD_SIZE);
        let entry = &self.data.as_slice()[offset..offset + FIELD_SIZE];
        Ok((
            LittleEndian::read_u32(&entry[0..4]),
            LittleEndian::read_u32(&entry[4..8]),
            LittleEndian::read_u32(&entry[8..12]),
        ))
    }

    fn read_field<'a>(
        self: &Arc<Self>,
        field_index: u32,
    ) -> Result<(String, GffValue<'a>), GffError> {
        let (field_type_u32, label_index, data_or_offset) = self.field_entry(field_index)?;
        let label = self.get_label(label_index)?.into_owned();

        let value = match field_type_u32 {
            0 => GffValue::Byte(data_or_offset as u8),
            1 => GffValue::Char(data_or_offset as u8 as char),
            2 => GffValue::Word(data_or_offset as u16),
            3 => GffValue::Short(data_or_offset as i16),
            4 => GffValue::Dword(data_or_offset),
            5 => GffValue::Int(data_or_offset as i32),
            6 => GffValue::Dword64(self.read_u64_data(data_or_offset)?),
            7 => GffValue::Int64(self.read_i64_data(data_or_offset)?),
            8 => GffValue::Float(f32::from_bits(data_or_offset)),
            9 => GffValue::Double(self.read_f64_data(data_or_offset)?),
            10 => GffValue::String(self.read_string(data_or_offset)?),
            11 => GffValue::ResRef(self.read_resref(data_or_offset)?),
            12 => GffValue::LocString(self.read_locstring(data_or_offset)?),
            13 => GffValue::Void(self.read_void(data_or_offset)?),
            14 => GffValue::Struct(self.create_lazy_struct(data_or_offset)?),
            15 => GffValue::List(self.read_list(data_or_offset)?),
            _ => return Err(GffError::UnsupportedFieldType(field_type_u32)),
        };

        Ok((label, value))
    }

    /// `len` bytes at `offset` inside the section starting at `base`.
    fn section_slice(
        &self,
        base: usize,
        section_len: usize,
        offset: usize,
        len: usize,
        section: &str,
    ) -> Result<&[u8], GffError> {
        let end = offset
            .checked_add(len)
            .filter(|&end| end <= section_len)
            .ok_or_else(|| GffError::BufferOverflow(format!("{section} read at {offset}")))?;
        Ok(&self.data.as_slice()[base + offset..base + end])
    }

    fn get_data_slice(&self, offset: usize, len: usize) -> Result<&[u8], GffError> {
        self.section_slice(
            self.field_data_offset,
            self.field_data_len,
            offset,
            len,
            "Field data",
        )
    }

    fn read_u64_data(&self, offset: u32) -> Result<u64, GffError> {
        let slice = self.get_data_slice(offset as usize, 8)?;
        Ok(LittleEndian::read_u64(slice))
    }

    fn read_i64_data(&self, offset: u32) -> Result<i64, GffError> {
        let slice = self.get_data_slice(offset as usize, 8)?;
        Ok(LittleEndian::read_i64(slice))
    }

    fn read_f64_data(&self, offset: u32) -> Result<f64, GffError> {
        let slice = self.get_data_slice(offset as usize, 8)?;
        Ok(LittleEndian::read_f64(slice))
    }

    fn read_string(&self, offset: u32) -> Result<GffText, GffError> {
        let offset = offset as usize;
        let len = LittleEndian::read_u32(self.get_data_slice(offset, 4)?) as usize;
        let str_slice = self.get_data_slice(offset + 4, len)?;
        Ok(GffText::decode(str_slice, 0))
    }

    fn read_resref(&self, offset: u32) -> Result<GffText, GffError> {
        let offset = offset as usize;
        let len = self.get_data_slice(offset, 1)?[0] as usize;
        let str_slice = self.get_data_slice(offset + 1, len)?;
        Ok(GffText::decode(str_slice, 0))
    }

    fn read_void<'a>(&self, offset: u32) -> Result<Cow<'a, [u8]>, GffError> {
        let offset = offset as usize;
        let len = LittleEndian::read_u32(self.get_data_slice(offset, 4)?) as usize;
        let data = self.get_data_slice(offset + 4, len)?;
        Ok(Cow::Owned(data.to_vec()))
    }

    fn read_locstring(&self, offset: u32) -> Result<LocalizedString, GffError> {
        let offset = offset as usize;
        let slice = self.get_data_slice(offset, 12)?;
        let string_ref = LittleEndian::read_i32(&slice[4..8]);
        let count = LittleEndian::read_u32(&slice[8..12]) as usize;

        let mut current_offset = offset + 12;
        let max_count = self.field_data_len.saturating_sub(current_offset) / 8;
        let mut substrings = Vec::with_capacity(count.min(max_count));

        for _ in 0..count {
            let sub_header = self.get_data_slice(current_offset, 8)?;
            let id = LittleEndian::read_u32(&sub_header[0..4]);
            let len = LittleEndian::read_u32(&sub_header[4..8]) as usize;

            let str_slice = self.get_data_slice(current_offset + 8, len)?;
            substrings.push(LocalizedSubstring {
                string: GffText::decode(str_slice, id / 2),
                language: id / 2,
                gender: id % 2,
            });

            current_offset += 8 + len;
        }

        Ok(LocalizedString {
            string_ref,
            substrings,
        })
    }

    fn create_lazy_struct(self: &Arc<Self>, struct_index: u32) -> Result<LazyStruct, GffError> {
        let struct_id = self.get_struct_id(struct_index)?;
        Ok(LazyStruct::new(self.clone(), struct_index, struct_id))
    }

    pub fn get_struct_id(&self, struct_index: u32) -> Result<u32, GffError> {
        if struct_index >= self.struct_count {
            return Err(GffError::InvalidStructIndex(struct_index));
        }
        let offset = self.struct_offset + (struct_index as usize * STRUCT_SIZE);
        Ok(LittleEndian::read_u32(
            &self.data.as_slice()[offset..offset + 4],
        ))
    }

    fn list_indices_slice(&self, offset: usize, len: usize) -> Result<&[u8], GffError> {
        self.section_slice(
            self.list_indices_offset,
            self.list_indices_len,
            offset,
            len,
            "List indices",
        )
    }

    fn read_list(
        self: &Arc<Self>,
        list_indices_byte_offset: u32,
    ) -> Result<Vec<LazyStruct>, GffError> {
        let start = list_indices_byte_offset as usize;
        let count = LittleEndian::read_u32(self.list_indices_slice(start, 4)?) as usize;
        let items_len = count
            .checked_mul(4)
            .ok_or_else(|| GffError::BufferOverflow(format!("List count {count}")))?;

        self.list_indices_slice(start + 4, items_len)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| {
                let struct_index = u32::from_le_bytes(*chunk);
                let struct_id = self.get_struct_id(struct_index)?;
                Ok(LazyStruct::new(self.clone(), struct_index, struct_id))
            })
            .collect()
    }

    pub fn get_value<'a>(self: &Arc<Self>, path: &str) -> Result<GffValue<'a>, GffError> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.is_empty() {
            return Err(GffError::FieldNotFound("(empty path)".into()));
        }
        let mut current_value = self.read_field_by_label(0, parts[0])?;

        for part in &parts[1..] {
            match current_value {
                GffValue::Struct(lazy) => {
                    current_value = self.read_field_by_label(lazy.struct_index, part)?;
                }
                GffValue::List(list) => {
                    let idx: usize = part.parse().map_err(|_| {
                        GffError::FieldNotFound(format!("Invalid list index: {part}"))
                    })?;
                    if idx >= list.len() {
                        return Err(GffError::FieldNotFound(format!(
                            "List index out of bounds: {idx}"
                        )));
                    }
                    current_value = GffValue::Struct(list[idx].clone());
                }
                _ => {
                    return Err(GffError::FieldNotFound(format!(
                        "Cannot traverse into non-structural field: {part}"
                    )));
                }
            }
        }

        Ok(current_value)
    }

    pub fn read_field_by_label<'a>(
        self: &Arc<Self>,
        struct_index: u32,
        label_to_find: &str,
    ) -> Result<GffValue<'a>, GffError> {
        for field_idx in self.struct_field_indices(struct_index)? {
            let (_, label_index, _) = self.field_entry(field_idx)?;
            if self.get_label(label_index)? == label_to_find {
                let (_, value) = self.read_field(field_idx)?;
                return Ok(value);
            }
        }

        Err(GffError::FieldNotFound(label_to_find.to_string()))
    }
}
