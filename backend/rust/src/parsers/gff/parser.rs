use std::borrow::Cow;
use std::fs::File;

use std::path::Path;
use std::sync::Arc;
use memmap2::Mmap;
use byteorder::{ByteOrder, LittleEndian};
use encoding_rs::WINDOWS_1252;
use indexmap::IndexMap;

use crate::parsers::gff::error::GffError;
use crate::parsers::gff::types::{GffValue, LazyStruct, LocalizedString, LocalizedSubstring};

// Constants
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

#[derive(Debug)]
pub struct GffParser {
    data: Arc<DataSource>,

    // Header Data
    pub file_type: String,
    pub file_version: String,

    // Offsets
    struct_offset: usize,
    struct_count: u32,
    field_offset: usize,
    field_count: u32,
    label_offset: usize,
    label_count: u32,
    field_data_offset: usize,
    _field_data_len: u32,
    field_indices_offset: usize,
    _field_indices_len: u32,
    list_indices_offset: usize,
    list_indices_len: u32,
}

impl GffParser {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Arc<Self>, GffError> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let data = Arc::new(DataSource::Mmap(mmap));

        let parser = Self::parse_header(data)?;
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
        let field_data_len = LittleEndian::read_u32(&slice[36..40]);
        let field_indices_offset = LittleEndian::read_u32(&slice[40..44]) as usize;
        let field_indices_len = LittleEndian::read_u32(&slice[44..48]);
        let list_indices_offset = LittleEndian::read_u32(&slice[48..52]) as usize;
        let list_indices_len = LittleEndian::read_u32(&slice[52..56]);

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
            _field_data_len: field_data_len,
            field_indices_offset,
            _field_indices_len: field_indices_len,
            list_indices_offset,
            list_indices_len,
        })
    }
    
    // --- Helper Accessors ---
    
    pub(crate) fn get_label<'a>(&self, index: u32) -> Result<Cow<'a, str>, GffError> {
        if index >= self.label_count {
            return Err(GffError::InvalidLabelIndex(index));
        }
        let slice = self.data.as_slice();
        let offset = self.label_offset + (index as usize * LABEL_SIZE);
        if offset + LABEL_SIZE > self.data.len() {
             return Err(GffError::BufferOverflow("Label array".into()));
        }

        let bytes = &slice[offset..offset + LABEL_SIZE];
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(LABEL_SIZE);
        let label_bytes = &bytes[..len];

        let (cow, _, _) = WINDOWS_1252.decode(label_bytes);
        Ok(Cow::Owned(cow.into_owned()))
    }
    
    // --- Struct Reading ---
    pub fn read_struct_fields<'a>(self: &Arc<Self>, struct_index: u32) -> Result<IndexMap<String, GffValue<'a>>, GffError> {
        if struct_index >= self.struct_count {
             return Err(GffError::InvalidStructIndex(struct_index));
        }

        let slice = self.data.as_slice();
        let offset = self.struct_offset + (struct_index as usize * STRUCT_SIZE);
        if offset + STRUCT_SIZE > self.data.len() {
             return Err(GffError::BufferOverflow("Struct array".into()));
        }

        // Struct: ID (4), FieldDataOrIndex (4), FieldCount (4)
        let _id = LittleEndian::read_u32(&slice[offset..offset+4]);
        let field_data_or_index = LittleEndian::read_u32(&slice[offset+4..offset+8]);
        let field_count = LittleEndian::read_u32(&slice[offset+8..offset+12]);

        let mut map = IndexMap::with_capacity(field_count as usize);

        if field_count == 1 {
            let (label, value) = self.read_field(field_data_or_index)?;
            map.insert(label, value);
        } else if field_count > 1 {
            let indices_offset = self.field_indices_offset + field_data_or_index as usize;
            for i in 0..field_count {
                let read_ptr = indices_offset + (i as usize * 4);
                if read_ptr + 4 > self.data.len() {
                     return Err(GffError::BufferOverflow("Field indices".into()));
                }
                let field_idx = LittleEndian::read_u32(&slice[read_ptr..read_ptr+4]);
                let (label, value) = self.read_field(field_idx)?;
                map.insert(label, value);
            }
        }

        Ok(map)
    }
    
    // Needs Arc<Self> to create LazyStructs
    fn read_field<'a>(self: &Arc<Self>, field_index: u32) -> Result<(String, GffValue<'a>), GffError> {
        if field_index >= self.field_count {
            return Err(GffError::InvalidFieldIndex(field_index));
        }

        let slice = self.data.as_slice();
        let offset = self.field_offset + (field_index as usize * FIELD_SIZE);
        if offset + FIELD_SIZE > self.data.len() { return Err(GffError::BufferOverflow("Field array".into())); }

        let field_type_u32 = LittleEndian::read_u32(&slice[offset..offset+4]);
        let label_index = LittleEndian::read_u32(&slice[offset+4..offset+8]);
        let data_or_offset = LittleEndian::read_u32(&slice[offset+8..offset+12]);
        
        let label = self.get_label(label_index)?.into_owned();
        
        // Types
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
    
    // --- Data Readers ---
    
    fn get_data_slice(&self, offset: u32, len: usize) -> Result<&[u8], GffError> {
        let start = self.field_data_offset + offset as usize;
        let end = start + len;
        if end > self.data.len() {
            return Err(GffError::BufferOverflow(format!("Data read at {}", offset)));
        }
        Ok(&self.data.as_slice()[start..end])
    }

    fn read_u64_data(&self, offset: u32) -> Result<u64, GffError> {
        let slice = self.get_data_slice(offset, 8)?;
        Ok(LittleEndian::read_u64(slice))
    }
    
    fn read_i64_data(&self, offset: u32) -> Result<i64, GffError> {
        let slice = self.get_data_slice(offset, 8)?;
        Ok(LittleEndian::read_i64(slice))
    }
    
    fn read_f64_data(&self, offset: u32) -> Result<f64, GffError> {
        let slice = self.get_data_slice(offset, 8)?;
        Ok(LittleEndian::read_f64(slice))
    }
    
    fn read_string<'a>(&self, offset: u32) -> Result<Cow<'a, str>, GffError> {
        let len_slice = self.get_data_slice(offset, 4)?;
        let len = LittleEndian::read_u32(len_slice) as usize;
        let str_slice = self.get_data_slice(offset + 4, len)?;
        let (cow, _, _) = WINDOWS_1252.decode(str_slice);
        Ok(Cow::Owned(cow.into_owned()))
    }
    
    fn read_resref<'a>(&self, offset: u32) -> Result<Cow<'a, str>, GffError> {
         let len_slice = self.get_data_slice(offset, 1)?;
         let len = len_slice[0] as usize;
         let str_slice = self.get_data_slice(offset + 1, len)?;
         let (cow, _, _) = WINDOWS_1252.decode(str_slice);
         Ok(Cow::Owned(cow.into_owned()))
    }
    
    fn read_void<'a>(&self, offset: u32) -> Result<Cow<'a, [u8]>, GffError> {
        let len_slice = self.get_data_slice(offset, 4)?;
        let len = LittleEndian::read_u32(len_slice) as usize;
        let data = self.get_data_slice(offset + 4, len)?;
        Ok(Cow::Owned(data.to_vec()))
    }
    
    fn read_locstring<'a>(&self, offset: u32) -> Result<LocalizedString<'a>, GffError> {
        let slice = self.get_data_slice(offset, 12)?;
        let string_ref = LittleEndian::read_i32(&slice[4..8]);
        let count = LittleEndian::read_u32(&slice[8..12]);
        
        let mut substrings = Vec::with_capacity(count as usize);
        let mut current_offset = offset + 12; 
        
        for _ in 0..count {
             let sub_header = self.get_data_slice(current_offset, 8)?;
             let id = LittleEndian::read_u32(&sub_header[0..4]);
             let len = LittleEndian::read_u32(&sub_header[4..8]);
             
             let str_slice = self.get_data_slice(current_offset + 8, len as usize)?;
             let (cow, _, _) = WINDOWS_1252.decode(str_slice);
             
             substrings.push(LocalizedSubstring {
                 string: Cow::Owned(cow.into_owned()),
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
        let slice = self.data.as_slice();
        let offset = self.struct_offset + (struct_index as usize * STRUCT_SIZE);
        if offset + 4 > self.data.len() {
            return Err(GffError::BufferOverflow("Struct ID read".into()));
        }
        Ok(LittleEndian::read_u32(&slice[offset..offset+4]))
    }
    
    fn read_list(self: &Arc<Self>, list_indices_byte_offset: u32) -> Result<Vec<LazyStruct>, GffError> {
         if list_indices_byte_offset as usize + 4 > self.list_indices_len as usize {
         }

         let slice = self.data.as_slice();
         let start = self.list_indices_offset + list_indices_byte_offset as usize;
         if start + 4 > self.data.len() { return Err(GffError::BufferOverflow("List count".into())); }

         let count = LittleEndian::read_u32(&slice[start..start+4]);
         let mut structs = Vec::with_capacity(count as usize);

         let mut current = start + 4;
         for _ in 0..count {
             if current + 4 > self.data.len() { return Err(GffError::BufferOverflow("List items".into())); }
             let struct_index = LittleEndian::read_u32(&slice[current..current+4]);
             let struct_id = self.get_struct_id(struct_index)?;
             structs.push(LazyStruct::new(self.clone(), struct_index, struct_id));
             current += 4;
         }

         Ok(structs)
    }

    // --- Path Access ---

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
                 },
                 GffValue::List(list) => {
                     let idx: usize = part.parse().map_err(|_| GffError::FieldNotFound(format!("Invalid list index: {}", part)))?;
                     if idx >= list.len() {
                         return Err(GffError::FieldNotFound(format!("List index out of bounds: {}", idx)));
                     }
                     current_value = GffValue::Struct(list[idx].clone());
                 },
                 _ => return Err(GffError::FieldNotFound(format!("Cannot traverse into non-structural field: {}", part))),
             }
        }

        Ok(current_value)
    }

    // Optimized linear search to avoid parsing all fields
    pub fn read_field_by_label<'a>(self: &Arc<Self>, struct_index: u32, label_to_find: &str) -> Result<GffValue<'a>, GffError> {
        if struct_index >= self.struct_count {
             return Err(GffError::InvalidStructIndex(struct_index));
        }

        let slice = self.data.as_slice();
        let offset = self.struct_offset + (struct_index as usize * STRUCT_SIZE);
        let field_data_or_index = LittleEndian::read_u32(&slice[offset+4..offset+8]);
        let field_count = LittleEndian::read_u32(&slice[offset+8..offset+12]);

        if field_count == 1 {
            let (label, value) = self.read_field(field_data_or_index)?;
            if label == label_to_find {
                return Ok(value);
            }
        } else if field_count > 1 {
            let indices_offset = self.field_indices_offset + field_data_or_index as usize;
            for i in 0..field_count {
                let read_ptr = indices_offset + (i as usize * 4);
                let field_idx = LittleEndian::read_u32(&slice[read_ptr..read_ptr+4]);

                let field_offset = self.field_offset + (field_idx as usize * FIELD_SIZE);
                let label_index = LittleEndian::read_u32(&slice[field_offset+4..field_offset+8]);

                let label_cow = self.get_label(label_index)?;
                if label_cow == label_to_find {
                     let (_, value) = self.read_field(field_idx)?;
                     return Ok(value);
                }
            }
        }

        Err(GffError::FieldNotFound(label_to_find.to_string()))
    }
}

