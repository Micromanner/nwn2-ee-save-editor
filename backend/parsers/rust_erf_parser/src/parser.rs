use crate::error::{ErfError, ErfResult};
use crate::types::*;
use byteorder::{LittleEndian, ReadBytesExt};
use indexmap::IndexMap;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Instant;
use string_interner::{DefaultBackend, StringInterner};

pub struct ErfParser {
    pub header: Option<ErfHeader>,
    pub erf_type: Option<ErfType>,
    pub version: Option<ErfVersion>,
    pub resources: IndexMap<String, ErfResource>,
    pub interner: StringInterner<DefaultBackend>,
    pub security_limits: SecurityLimits,
    pub stats: ErfStatistics,
    pub metadata: Option<FileMetadata>,
    mmap: Option<Mmap>,
    file_data: Option<Vec<u8>>,
}

impl ErfParser {
    pub fn new() -> Self {
        Self {
            header: None,
            erf_type: None,
            version: None,
            resources: IndexMap::new(),
            interner: StringInterner::default(),
            security_limits: SecurityLimits::default(),
            stats: ErfStatistics {
                total_resources: 0,
                total_size: 0,
                resource_types: HashMap::new(),
                largest_resource: None,
                parse_time_ms: 0,
            },
            metadata: None,
            mmap: None,
            file_data: None,
        }
    }
    
    pub fn with_limits(mut self, limits: SecurityLimits) -> Self {
        self.security_limits = limits;
        self
    }
    
    pub fn read<P: AsRef<Path>>(&mut self, path: P) -> ErfResult<()> {
        let start = Instant::now();
        let path = path.as_ref();
        
        let file = File::open(path)?;
        let file_size = file.metadata()?.len() as usize;
        
        if file_size > self.security_limits.max_file_size {
            return Err(ErfError::FileTooLarge {
                size: file_size,
                max: self.security_limits.max_file_size,
            });
        }
        
        // Use memory mapping for better performance
        let mmap = unsafe { Mmap::map(&file)? };
        
        let mut cursor = Cursor::new(&mmap[..]);
        self.parse_header(&mut cursor)?;
        
        if let Some(header) = self.header.clone() {
            self.validate_header(&header, file_size)?;
            
            // Parse key and resource lists
            let keys = self.parse_key_list(&mut cursor, &header)?;
            let resources = self.parse_resource_list(&mut cursor, &header)?;
            
            // Combine keys and resources
            self.build_resource_map(keys, resources)?;
        }
        
        // Store mmap for later resource extraction
        self.mmap = Some(mmap);
        
        // Update metadata
        self.metadata = Some(FileMetadata {
            file_path: path.to_string_lossy().into_owned(),
            file_size,
            erf_type: self.erf_type.map(|t| t.as_str().to_string()).unwrap_or_default(),
            version: self.version.map(|v| match v {
                ErfVersion::V10 => "V1.0",
                ErfVersion::V11 => "V1.1",
            }).unwrap_or_default().to_string(),
            build_date: self.header.as_ref().map(|h| 
                format!("{}/{}", h.build_year + 1900, h.build_day)
            ).unwrap_or_default(),
        });
        
        self.stats.parse_time_ms = start.elapsed().as_millis();
        self.stats.total_resources = self.resources.len();
        self.stats.total_size = file_size;
        
        Ok(())
    }
    
    pub fn parse_from_bytes(&mut self, data: &[u8]) -> ErfResult<()> {
        let start = Instant::now();
        let file_size = data.len();
        
        if file_size > self.security_limits.max_file_size {
            return Err(ErfError::FileTooLarge {
                size: file_size,
                max: self.security_limits.max_file_size,
            });
        }
        
        let mut cursor = Cursor::new(data);
        self.parse_header(&mut cursor)?;
        
        if let Some(header) = self.header.clone() {
            self.validate_header(&header, file_size)?;
            
            let keys = self.parse_key_list(&mut cursor, &header)?;
            let resources = self.parse_resource_list(&mut cursor, &header)?;
            
            self.build_resource_map(keys, resources)?;
        }
        
        // Store data for later resource extraction
        self.file_data = Some(data.to_vec());
        
        self.stats.parse_time_ms = start.elapsed().as_millis();
        self.stats.total_resources = self.resources.len();
        self.stats.total_size = file_size;
        
        Ok(())
    }
    
    fn parse_header<R: Read>(&mut self, reader: &mut R) -> ErfResult<()> {
        let mut sig = [0u8; 4];
        reader.read_exact(&mut sig)?;
        
        self.erf_type = Some(ErfType::from_signature(&sig)
            .ok_or_else(|| ErfError::InvalidSignature {
                found: String::from_utf8_lossy(&sig).into_owned(),
            })?);
        
        let mut ver = [0u8; 4];
        reader.read_exact(&mut ver)?;
        
        self.version = match &ver {
            b"V1.0" => Some(ErfVersion::V10),
            b"V1.1" => Some(ErfVersion::V11),
            _ => return Err(ErfError::InvalidVersion {
                found: String::from_utf8_lossy(&ver).into_owned(),
            }),
        };
        
        let header = ErfHeader {
            file_type: String::from_utf8_lossy(&sig).into_owned(),
            version: String::from_utf8_lossy(&ver).into_owned(),
            language_count: reader.read_u32::<LittleEndian>()?,
            localized_string_size: reader.read_u32::<LittleEndian>()?,
            entry_count: reader.read_u32::<LittleEndian>()?,
            offset_to_localized_string: reader.read_u32::<LittleEndian>()?,
            offset_to_key_list: reader.read_u32::<LittleEndian>()?,
            offset_to_resource_list: reader.read_u32::<LittleEndian>()?,
            build_year: reader.read_u32::<LittleEndian>()?,
            build_day: reader.read_u32::<LittleEndian>()?,
            description_str_ref: reader.read_u32::<LittleEndian>()?,
        };
        
        // Skip reserved bytes (116 bytes)
        let mut reserved = vec![0u8; 116];
        reader.read_exact(&mut reserved)?;
        
        self.header = Some(header);
        Ok(())
    }
    
    fn validate_header(&self, header: &ErfHeader, file_size: usize) -> ErfResult<()> {
        if header.entry_count > self.security_limits.max_resource_count as u32 {
            return Err(ErfError::InvalidResourceCount {
                count: header.entry_count,
                max: self.security_limits.max_resource_count as u32,
            });
        }
        
        if header.offset_to_key_list as usize > file_size {
            return Err(ErfError::InvalidOffset {
                offset: header.offset_to_key_list as usize,
                file_size,
            });
        }
        
        if header.offset_to_resource_list as usize > file_size {
            return Err(ErfError::InvalidOffset {
                offset: header.offset_to_resource_list as usize,
                file_size,
            });
        }
        
        Ok(())
    }
    
    fn parse_key_list<R: Read + Seek>(&mut self, reader: &mut R, header: &ErfHeader) -> ErfResult<Vec<KeyEntry>> {
        reader.seek(SeekFrom::Start(header.offset_to_key_list as u64))?;
        
        let version = self.version.ok_or_else(|| ErfError::corrupted_data("Missing version"))?;
        let entry_size = version.key_entry_size();
        let name_length = version.max_resource_name_length();
        
        let mut keys = Vec::with_capacity(header.entry_count as usize);
        
        for _ in 0..header.entry_count {
            let mut name_bytes = vec![0u8; name_length];
            reader.read_exact(&mut name_bytes)?;
            
            // Convert name, stopping at null terminator
            let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_length);
            let name_slice = &name_bytes[..name_end];
            
            // Validate ASCII
            if !name_slice.iter().all(|&b| b.is_ascii()) {
                return Err(ErfError::InvalidResourceName);
            }
            
            let resource_name = String::from_utf8_lossy(name_slice).into_owned();
            
            let resource_id = reader.read_u32::<LittleEndian>()?;
            let resource_type = reader.read_u16::<LittleEndian>()?;
            let reserved = reader.read_u16::<LittleEndian>()?;
            
            let interned_name = self.interner.get_or_intern(resource_name);
            keys.push(KeyEntry {
                resource_name: self.interner.resolve(interned_name).unwrap().to_string(),
                resource_id,
                resource_type,
                reserved,
            });
        }
        
        Ok(keys)
    }
    
    fn parse_resource_list<R: Read + Seek>(&mut self, reader: &mut R, header: &ErfHeader) -> ErfResult<Vec<ResourceEntry>> {
        reader.seek(SeekFrom::Start(header.offset_to_resource_list as u64))?;
        
        let mut resources = Vec::with_capacity(header.entry_count as usize);
        
        for _ in 0..header.entry_count {
            let offset = reader.read_u32::<LittleEndian>()?;
            let size = reader.read_u32::<LittleEndian>()?;
            
            if size > self.security_limits.max_resource_size as u32 {
                return Err(ErfError::security_violation(
                    format!("Resource size {} exceeds maximum {}", size, self.security_limits.max_resource_size)
                ));
            }
            
            resources.push(ResourceEntry { offset, size });
        }
        
        Ok(resources)
    }
    
    fn build_resource_map(&mut self, keys: Vec<KeyEntry>, resources: Vec<ResourceEntry>) -> ErfResult<()> {
        if keys.len() != resources.len() {
            return Err(ErfError::corrupted_data(
                format!("Key count {} doesn't match resource count {}", keys.len(), resources.len())
            ));
        }
        
        self.resources.clear();
        let mut largest: Option<(String, usize)> = None;
        
        for (key, entry) in keys.into_iter().zip(resources.into_iter()) {
            // Update statistics
            *self.stats.resource_types.entry(key.resource_type).or_insert(0) += 1;
            
            let size = entry.size as usize;
            if largest.as_ref().map_or(true, |(_, s)| size > *s) {
                largest = Some((key.full_name(), size));
            }
            
            let full_name = key.full_name().to_lowercase();
            self.resources.insert(full_name, ErfResource {
                key,
                entry,
                data: None,
            });
        }
        
        self.stats.largest_resource = largest;
        Ok(())
    }
    
    pub fn list_resources(&self, resource_type: Option<u16>) -> Vec<(String, u32, u16)> {
        self.resources
            .iter()
            .filter(|(_, res)| {
                resource_type.map_or(true, |rt| res.key.resource_type == rt)
            })
            .map(|(name, res)| (name.clone(), res.entry.size, res.key.resource_type))
            .collect()
    }
    
    pub fn extract_resource(&mut self, name: &str) -> ErfResult<Vec<u8>> {
        let name_lower = name.to_lowercase();
        
        // Check if we have the resource
        if !self.resources.contains_key(&name_lower) {
            return Err(ErfError::ResourceNotFound { name: name.to_string() });
        }
        
        // Get the entry info (cloned to avoid borrow issues)
        let entry = self.resources.get(&name_lower).unwrap().entry.clone();
        
        // Check if data is already cached
        if let Some(cached_data) = self.resources.get(&name_lower).and_then(|r| r.data.as_ref()) {
            return Ok(cached_data.clone());
        }
        
        // Extract data
        let data = if let Some(mmap) = &self.mmap {
            self.extract_from_mmap(mmap, &entry)?
        } else if let Some(file_data) = &self.file_data {
            self.extract_from_bytes(file_data, &entry)?
        } else {
            return Err(ErfError::corrupted_data("No data source available"));
        };
        
        // Cache the data
        if let Some(resource) = self.resources.get_mut(&name_lower) {
            resource.data = Some(data.clone());
        }
        
        Ok(data)
    }
    
    fn extract_from_mmap(&self, mmap: &Mmap, entry: &ResourceEntry) -> ErfResult<Vec<u8>> {
        let offset = entry.offset as usize;
        let size = entry.size as usize;
        
        if offset + size > mmap.len() {
            return Err(ErfError::InvalidOffset {
                offset: offset + size,
                file_size: mmap.len(),
            });
        }
        
        Ok(mmap[offset..offset + size].to_vec())
    }
    
    fn extract_from_bytes(&self, data: &[u8], entry: &ResourceEntry) -> ErfResult<Vec<u8>> {
        let offset = entry.offset as usize;
        let size = entry.size as usize;
        
        if offset + size > data.len() {
            return Err(ErfError::InvalidOffset {
                offset: offset + size,
                file_size: data.len(),
            });
        }
        
        Ok(data[offset..offset + size].to_vec())
    }
    
    pub fn extract_all_by_type(&mut self, resource_type: u16, output_dir: &Path) -> ErfResult<Vec<String>> {
        std::fs::create_dir_all(output_dir)?;
        
        let resources_to_extract: Vec<String> = self.resources
            .iter()
            .filter(|(_, res)| res.key.resource_type == resource_type)
            .map(|(name, _)| name.clone())
            .collect();
        
        let mut extracted = Vec::new();
        
        for name in resources_to_extract {
            let data = self.extract_resource(&name)?;
            let output_path = output_dir.join(&name);
            
            let mut file = std::fs::File::create(&output_path)?;
            file.write_all(&data)?;
            
            extracted.push(output_path.to_string_lossy().into_owned());
        }
        
        Ok(extracted)
    }
    
    pub fn extract_all_2da(&mut self, output_dir: &Path) -> ErfResult<Vec<String>> {
        self.extract_all_by_type(2017, output_dir)  // 2017 is the 2DA resource type
    }
    
    pub fn get_module_info(&mut self) -> ErfResult<Option<Vec<u8>>> {
        if self.erf_type != Some(ErfType::MOD) {
            return Ok(None);
        }
        
        // Look for module.ifo
        if self.resources.contains_key("module.ifo") {
            Ok(Some(self.extract_resource("module.ifo")?))
        } else {
            Ok(None)
        }
    }
    
    pub fn get_statistics(&self) -> &ErfStatistics {
        &self.stats
    }
    
    pub fn clear_cache(&mut self) {
        for resource in self.resources.values_mut() {
            resource.data = None;
        }
    }
}