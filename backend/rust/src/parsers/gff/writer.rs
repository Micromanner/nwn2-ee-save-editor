use std::io::{Cursor, Write};
use byteorder::{LittleEndian, WriteBytesExt};
use indexmap::IndexMap;
use crate::parsers::gff::types::{GffFieldType, GffValue};
use crate::parsers::gff::error::GffError;

pub struct GffWriter {
    pub file_type: String,
    pub file_version: String,

    structs: Vec<(u32, u32, u32)>, // (id, field_index, field_count)
    fields: Vec<(u32, u32, u32)>,  // (type, label_index, data_or_offset)
    labels: IndexMap<String, u32>,
    field_data: Cursor<Vec<u8>>,
    field_indices: Vec<u32>,
    list_indices: Vec<u32>,

    struct_queue: Vec<IndexMap<String, GffValue<'static>>>,
    struct_ids: Vec<u32>, // Parallel to struct_queue - stores struct IDs
}

impl GffWriter {
    pub fn new(file_type: &str, file_version: &str) -> Self {
        GffWriter {
            file_type: format!("{:4}", file_type).chars().take(4).collect(),
            file_version: format!("{:4}", file_version).chars().take(4).collect(),
            structs: Vec::new(),
            fields: Vec::new(),
            labels: IndexMap::new(),
            field_data: Cursor::new(Vec::new()),
            field_indices: Vec::new(),
            list_indices: Vec::new(),
            struct_queue: Vec::new(),
            struct_ids: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.structs.clear();
        self.fields.clear();
        self.labels.clear();
        self.field_data = Cursor::new(Vec::new());
        self.field_indices.clear();
        self.list_indices.clear();
        self.struct_queue.clear();
        self.struct_ids.clear();
    }

    pub fn write(&mut self, root: IndexMap<String, GffValue<'static>>) -> Result<Vec<u8>, GffError> {
        self.write_with_struct_id(root, 0xFFFFFFFF)
    }

    pub fn write_with_struct_id(&mut self, root: IndexMap<String, GffValue<'static>>, root_struct_id: u32) -> Result<Vec<u8>, GffError> {
        self.reset();

        let mut flat_structs: Vec<IndexMap<String, GffValue<'static>>> = Vec::new();
        let mut struct_ids: Vec<u32> = Vec::new();
        let _root_idx = self.flatten_value_with_id(GffValue::StructOwned(Box::new(root)), root_struct_id, &mut flat_structs, &mut struct_ids)?;

        self.struct_queue = flat_structs;
        self.struct_ids = struct_ids;

        self.structs.resize(self.struct_queue.len(), (0, 0, 0));

        for i in 0..self.struct_queue.len() {
            let fields = std::mem::take(&mut self.struct_queue[i]);
            let struct_id = self.struct_ids[i];
            self.encode_struct_with_id(i as u32, fields, struct_id)?;
        }

        self.finalize()
    }
        
    fn flatten_value_with_id(
        &mut self,
        val: GffValue<'static>,
        struct_id: u32,
        flat_list: &mut Vec<IndexMap<String, GffValue<'static>>>,
        struct_ids: &mut Vec<u32>,
    ) -> Result<u32, GffError> {
        match val {
            GffValue::StructOwned(mut map) => {
                let idx = flat_list.len() as u32;
                flat_list.push(IndexMap::new());

                // Extract __struct_id__ from map if present, otherwise use provided struct_id
                let actual_struct_id = map.shift_remove("__struct_id__")
                    .and_then(|v| match v {
                        GffValue::Dword(id) => Some(id),
                        _ => None,
                    })
                    .unwrap_or(struct_id);

                // Also remove __field_types__ as it's metadata not a real field
                map.shift_remove("__field_types__");

                struct_ids.push(actual_struct_id);

                let mut new_map = IndexMap::new();
                for (k, v) in *map {
                    let processed_v = self.process_field_value_with_id(v, flat_list, struct_ids)?;
                    new_map.insert(k, processed_v);
                }

                flat_list[idx as usize] = new_map;
                Ok(idx)
            },
            _ => Err(GffError::Serialization("Root must be a struct".to_string())),
        }
    }

    fn process_field_value_with_id(
        &mut self,
        val: GffValue<'static>,
        flat_list: &mut Vec<IndexMap<String, GffValue<'static>>>,
        struct_ids: &mut Vec<u32>,
    ) -> Result<GffValue<'static>, GffError> {
        match val {
            GffValue::StructOwned(map) => {
                // Extract struct_id from map during flattening
                let idx = self.flatten_value_with_id(GffValue::StructOwned(map), 0, flat_list, struct_ids)?;
                Ok(GffValue::StructRef(idx))
            },
            GffValue::ListOwned(list) => {
                let mut new_list = Vec::new();
                for item in list {
                    // Each list item may have its own __struct_id__
                    let val = GffValue::StructOwned(Box::new(item));
                    let idx = self.flatten_value_with_id(val, 0, flat_list, struct_ids)?;
                    new_list.push(idx);
                }
                Ok(GffValue::ListRef(new_list))
            },
            v => Ok(v),
        }
    }

    fn encode_struct_with_id(&mut self, struct_idx: u32, fields: IndexMap<String, GffValue<'static>>, struct_id: u32) -> Result<(), GffError> {
        let field_start_idx = self.fields.len() as u32;
        let field_count = fields.len() as u32;

        for (label, value) in fields {
            self.encode_field(label, value)?;
        }

        let final_field_idx = if field_count == 1 {
            field_start_idx
        } else if field_count == 0 {
            0
        } else {
            let start_offset = self.field_indices.len() as u32 * 4;
            for i in 0..field_count {
                self.field_indices.push(field_start_idx + i);
            }
            start_offset
        };

        self.structs[struct_idx as usize] = (struct_id, final_field_idx, field_count);
        Ok(())
    }

    fn encode_field(&mut self, label: String, value: GffValue<'static>) -> Result<(), GffError> {
        let label_idx = self.get_label_index(label);
        let (type_id, data) = match value {
            GffValue::Byte(v) => (GffFieldType::Byte, v as u32),
            GffValue::Char(v) => (GffFieldType::Char, v as u32),
            GffValue::Word(v) => (GffFieldType::Word, v as u32),
            GffValue::Short(v) => (GffFieldType::Short, v as u32), // Casts bits
            GffValue::Dword(v) => (GffFieldType::Dword, v),
            GffValue::Int(v) => (GffFieldType::Int, v as u32),
            
            // Complex types
             GffValue::Dword64(v) => {
                let offset = self.field_data.position() as u32;
                self.field_data.write_u64::<LittleEndian>(v)?;
                (GffFieldType::Dword64, offset)
            },
            GffValue::Int64(v) => {
                let offset = self.field_data.position() as u32;
                self.field_data.write_i64::<LittleEndian>(v)?;
                (GffFieldType::Int64, offset)
            },
            GffValue::Float(v) => {
                 // Float is stored directly if 4 bytes? 
                 // Yes, GFF stores floats in the 4-byte data field.
                 (GffFieldType::Float, v.to_bits())
            },
             GffValue::Double(v) => {
                let offset = self.field_data.position() as u32;
                self.field_data.write_f64::<LittleEndian>(v)?;
                (GffFieldType::Double, offset)
            },
            GffValue::String(v) => {
                 let offset = self.field_data.position() as u32;
                 let bytes = v.as_bytes();
                 self.field_data.write_u32::<LittleEndian>(bytes.len() as u32)?;
                 self.field_data.write_all(bytes)?;
                 (GffFieldType::String, offset)
            },
             GffValue::ResRef(v) => {
                 let offset = self.field_data.position() as u32;
                 let bytes = v.as_bytes();
                 let len = bytes.len().min(32) as u8;
                 self.field_data.write_u8(len)?;
                 self.field_data.write_all(&bytes[..len as usize])?;
                 (GffFieldType::ResRef, offset)
            },
             GffValue::Void(v) => {
                 let offset = self.field_data.position() as u32;
                 self.field_data.write_u32::<LittleEndian>(v.len() as u32)?;
                 self.field_data.write_all(&v)?;
                 (GffFieldType::Void, offset)
             },
             GffValue::LocString(v) => {
                 let offset = self.field_data.position() as u32;
                 
                 // Calculate full size
                 // Header: 4 (TotalSize) + 4 (StringRef) + 4 (Count) = 12 bytes
                 // Per string: 4 (ID) + 4 (Size) + Len
                 let mut content_size = 0;
                 for sub in &v.substrings {
                     content_size += 8 + sub.string.as_bytes().len();
                 }
                 let total_size = 12 + content_size; // Wait, total_size usually *excludes* itself? 
                 // In GFF, the size field usually covers the whole block.
                 // Legacy GFF spec: "Total Size" is size of the structure.
                 
                 self.field_data.write_u32::<LittleEndian>(total_size as u32)?;
                 self.field_data.write_u32::<LittleEndian>(v.string_ref as u32)?;
                 self.field_data.write_u32::<LittleEndian>(v.substrings.len() as u32)?;
                 
                 for sub in &v.substrings {
                     // ID = (Language << 1) | Gender
                     let id = (sub.language << 1) | (sub.gender & 1); 
                     let bytes = sub.string.as_bytes();
                     self.field_data.write_u32::<LittleEndian>(id)?;
                     self.field_data.write_u32::<LittleEndian>(bytes.len() as u32)?;
                     self.field_data.write_all(bytes)?;
                 }
                 (GffFieldType::LocString, offset)
             },
             GffValue::StructRef(idx) => (GffFieldType::Struct, idx),

             GffValue::ListRef(indices) => {
                 let offset = self.list_indices.len() as u32 * 4; // Byte offset
                 self.list_indices.push(indices.len() as u32); // Size
                 self.list_indices.extend(indices);
                 (GffFieldType::List, offset)
             },
             _ => return Err(GffError::Serialization(format!("Unsupported type for writing: {:?}", value))),
        };
        
        self.fields.push((type_id as u32, label_idx, data));
        Ok(())
    }

    fn get_label_index(&mut self, label: String) -> u32 {
         if let Some(&idx) = self.labels.get(&label) {
             idx
         } else {
             let idx = self.labels.len() as u32;
             self.labels.insert(label, idx);
             idx
         }
    }
    
    fn finalize(&mut self) -> Result<Vec<u8>, GffError> {
        let mut buffer = Vec::new();
        
        let struct_offset = 56;
        let struct_size = (self.structs.len() * 12) as u32;
        
        let field_offset = struct_offset + struct_size;
        let field_size = (self.fields.len() * 12) as u32;
        
        let label_offset = field_offset + field_size;
        let label_size = (self.labels.len() * 16) as u32;
        
        let field_data_offset = label_offset + label_size;
        let field_data_size = self.field_data.position() as u32;
        
        let field_indices_offset = field_data_offset + field_data_size;
        let field_indices_size = (self.field_indices.len() * 4) as u32;
        
        let list_indices_offset = field_indices_offset + field_indices_size;
        let list_indices_size = (self.list_indices.len() * 4) as u32;
        
        // Write Header
        buffer.write_all(self.file_type.as_bytes())?; // Already 4 chars
        buffer.write_all(self.file_version.as_bytes())?; // Already 4 chars
        
        buffer.write_u32::<LittleEndian>(struct_offset)?;
        buffer.write_u32::<LittleEndian>(self.structs.len() as u32)?;
        buffer.write_u32::<LittleEndian>(field_offset)?;
        buffer.write_u32::<LittleEndian>(self.fields.len() as u32)?;
        buffer.write_u32::<LittleEndian>(label_offset)?;
        buffer.write_u32::<LittleEndian>(self.labels.len() as u32)?;
        buffer.write_u32::<LittleEndian>(field_data_offset)?;
        buffer.write_u32::<LittleEndian>(field_data_size)?;
        buffer.write_u32::<LittleEndian>(field_indices_offset)?;
        buffer.write_u32::<LittleEndian>(field_indices_size)?;
        buffer.write_u32::<LittleEndian>(list_indices_offset)?;
        buffer.write_u32::<LittleEndian>(list_indices_size)?;
        
        // Write Structs
        for (id, field_idx, count) in &self.structs {
             buffer.write_u32::<LittleEndian>(*id)?;
             buffer.write_u32::<LittleEndian>(*field_idx)?;
             buffer.write_u32::<LittleEndian>(*count)?;
        }
        
        // Write Fields
        for (type_id, label_idx, data) in &self.fields {
            buffer.write_u32::<LittleEndian>(*type_id)?;
            buffer.write_u32::<LittleEndian>(*label_idx)?;
            buffer.write_u32::<LittleEndian>(*data)?;
        }
        
        // Write Labels
        // Labels in IndexMap are ordered by insertion, which matches label indices 0..N
        for (label, _) in &self.labels {
            let mut label_bytes = [0u8; 16];
            let bytes = label.as_bytes();
            let len = bytes.len().min(16);
            label_bytes[..len].copy_from_slice(&bytes[..len]);
            buffer.write_all(&label_bytes)?;
        }
        
        // Write Field Data
        buffer.write_all(self.field_data.get_ref())?;
        
        // Write Field Indices
        for idx in &self.field_indices {
            buffer.write_u32::<LittleEndian>(*idx)?;
        }
        
        // Write List Indices
        for idx in &self.list_indices {
            buffer.write_u32::<LittleEndian>(*idx)?;
        }
        
        Ok(buffer)
    }
}
