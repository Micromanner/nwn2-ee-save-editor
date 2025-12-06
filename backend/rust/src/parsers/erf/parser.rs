use super::error::{ErfError, ErfResult};
use super::types::*;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Instant;
use lasso::Rodeo;

pub struct ErfParser {
    pub header: Option<ErfHeader>,
    pub erf_type: Option<ErfType>,
    pub version: Option<ErfVersion>,
    pub resources: IndexMap<String, ErfResource>,
    pub interner: Rodeo,
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
            interner: Rodeo::default(),
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
            
            let interned_name = self.interner.get_or_intern(&resource_name);
            keys.push(KeyEntry {
                resource_name: self.interner.resolve(&interned_name).to_string(),
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

    pub fn add_resource(&mut self, name: &str, resource_type: u16, data: Vec<u8>) -> ErfResult<()> {
        let version = self.version.unwrap_or(ErfVersion::V11);
        let max_name_len = version.max_resource_name_length();

        let (base_name, ext) = if let Some(dot_pos) = name.rfind('.') {
            (&name[..dot_pos], &name[dot_pos + 1..])
        } else {
            (name, "")
        };

        if base_name.len() > max_name_len {
            return Err(ErfError::InvalidResourceName);
        }

        if !base_name.bytes().all(|b| b.is_ascii()) {
            return Err(ErfError::InvalidResourceName);
        }

        let full_name = if ext.is_empty() {
            format!("{}.{}", base_name, resource_type_to_extension(resource_type))
        } else {
            name.to_string()
        };

        let key = KeyEntry {
            resource_name: base_name.to_string(),
            resource_id: self.resources.len() as u32,
            resource_type,
            reserved: 0,
        };

        let entry = ResourceEntry {
            offset: 0,
            size: data.len() as u32,
        };

        let resource = ErfResource {
            key,
            entry,
            data: Some(data),
        };

        self.resources.insert(full_name.to_lowercase(), resource);

        if let Some(header) = &mut self.header {
            header.entry_count = self.resources.len() as u32;
        }

        Ok(())
    }

    pub fn remove_resource(&mut self, name: &str) -> ErfResult<bool> {
        let name_lower = name.to_lowercase();
        let removed = self.resources.shift_remove(&name_lower).is_some();

        if removed {
            if let Some(header) = &mut self.header {
                header.entry_count = self.resources.len() as u32;
            }
        }

        Ok(removed)
    }

    pub fn update_resource(&mut self, name: &str, data: Vec<u8>) -> ErfResult<()> {
        let name_lower = name.to_lowercase();

        if let Some(resource) = self.resources.get_mut(&name_lower) {
            resource.entry.size = data.len() as u32;
            resource.data = Some(data);
            Ok(())
        } else {
            Err(ErfError::ResourceNotFound { name: name.to_string() })
        }
    }

    pub fn to_bytes(&self) -> ErfResult<Vec<u8>> {
        let version = self.version.ok_or_else(|| ErfError::corrupted_data("No version set"))?;
        let erf_type = self.erf_type.ok_or_else(|| ErfError::corrupted_data("No ERF type set"))?;

        let mut output = Vec::new();

        self.write_header_bytes(&mut output, version, erf_type)?;
        self.write_keys_bytes(&mut output, version)?;
        self.write_resource_list_bytes(&mut output)?;
        self.write_resource_data_bytes(&mut output)?;

        Ok(output)
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> ErfResult<()> {
        let data = self.to_bytes()?;
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&data)?;
        writer.flush()?;
        Ok(())
    }

    fn write_header_bytes(&self, output: &mut Vec<u8>, version: ErfVersion, erf_type: ErfType) -> ErfResult<()> {
        let key_size = version.key_entry_size();
        let resource_count = self.resources.len();

        let header_size = 160u32;
        let offset_to_keys = header_size;
        let offset_to_resources = offset_to_keys + (resource_count as u32 * key_size as u32);

        output.extend_from_slice(erf_type.signature());
        output.extend_from_slice(version.version_bytes());

        output.write_u32::<LittleEndian>(0)?;
        output.write_u32::<LittleEndian>(0)?;
        output.write_u32::<LittleEndian>(resource_count as u32)?;
        output.write_u32::<LittleEndian>(header_size)?;
        output.write_u32::<LittleEndian>(offset_to_keys)?;
        output.write_u32::<LittleEndian>(offset_to_resources)?;

        let header = self.header.as_ref();
        let build_year = header.map(|h| h.build_year).unwrap_or(125);
        let build_day = header.map(|h| h.build_day).unwrap_or(1);
        let description_str_ref = header.map(|h| h.description_str_ref).unwrap_or(0xFFFFFFFF);

        output.write_u32::<LittleEndian>(build_year)?;
        output.write_u32::<LittleEndian>(build_day)?;
        output.write_u32::<LittleEndian>(description_str_ref)?;

        output.extend_from_slice(&[0u8; 116]);

        Ok(())
    }

    fn write_keys_bytes(&self, output: &mut Vec<u8>, version: ErfVersion) -> ErfResult<()> {
        let name_length = version.max_resource_name_length();

        for (index, resource) in self.resources.values().enumerate() {
            let mut name_bytes = vec![0u8; name_length];
            let name = resource.key.resource_name.as_bytes();
            let copy_len = name.len().min(name_length);
            name_bytes[..copy_len].copy_from_slice(&name[..copy_len]);
            output.extend_from_slice(&name_bytes);

            output.write_u32::<LittleEndian>(index as u32)?;
            output.write_u16::<LittleEndian>(resource.key.resource_type)?;
            output.write_u16::<LittleEndian>(resource.key.reserved)?;
        }

        Ok(())
    }

    fn write_resource_list_bytes(&self, output: &mut Vec<u8>) -> ErfResult<()> {
        let version = self.version.ok_or_else(|| ErfError::corrupted_data("No version set"))?;
        let key_size = version.key_entry_size();
        let resource_count = self.resources.len();

        let header_size = 160u32;
        let keys_size = (resource_count * key_size) as u32;
        let resource_list_size = (resource_count * 8) as u32;
        let mut data_offset = header_size + keys_size + resource_list_size;

        for resource in self.resources.values() {
            output.write_u32::<LittleEndian>(data_offset)?;
            output.write_u32::<LittleEndian>(resource.entry.size)?;
            data_offset += resource.entry.size;
        }

        Ok(())
    }

    fn write_resource_data_bytes(&self, output: &mut Vec<u8>) -> ErfResult<()> {
        for resource in self.resources.values() {
            if let Some(data) = &resource.data {
                output.extend_from_slice(data);
            } else {
                return Err(ErfError::corrupted_data(
                    format!("Resource '{}' has no data loaded", resource.key.resource_name)
                ));
            }
        }

        Ok(())
    }

    pub fn new_archive(erf_type: ErfType, version: ErfVersion) -> Self {
        Self {
            header: Some(ErfHeader {
                file_type: erf_type.as_str().to_string() + " ",
                version: match version {
                    ErfVersion::V10 => "V1.0".to_string(),
                    ErfVersion::V11 => "V1.1".to_string(),
                },
                language_count: 0,
                localized_string_size: 0,
                entry_count: 0,
                offset_to_localized_string: 160,
                offset_to_key_list: 160,
                offset_to_resource_list: 160,
                build_year: 125,
                build_day: 1,
                description_str_ref: 0xFFFFFFFF,
            }),
            erf_type: Some(erf_type),
            version: Some(version),
            resources: IndexMap::new(),
            interner: Rodeo::default(),
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

    pub fn load_all_resources(&mut self) -> ErfResult<()> {
        let names: Vec<String> = self.resources.keys().cloned().collect();
        for name in names {
            let _ = self.extract_resource(&name)?;
        }
        Ok(())
    }
}