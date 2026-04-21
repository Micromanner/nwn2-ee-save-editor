pub mod error;
pub mod types;

use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use encoding_rs::{UTF_8, WINDOWS_1252};
use indexmap::IndexMap;
use tracing::{debug, warn};

use crate::character::gff_helpers::gff_value_to_u32;
use crate::character::types::{
    ALIGNMENT_EVIL_THRESHOLD, ALIGNMENT_GOOD_THRESHOLD, calculate_modifier,
};
use crate::parsers::gff::GffValue;

pub use error::{PlayerInfoParseError, PlayerInfoResult};
pub use types::{PlayerClassEntry, PlayerInfoData};

pub struct PlayerInfo {
    pub file_path: Option<std::path::PathBuf>,
    pub data: PlayerInfoData,
}

impl PlayerInfo {
    pub fn new() -> Self {
        Self {
            file_path: None,
            data: PlayerInfoData::new(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> PlayerInfoResult<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        let mut cursor = Cursor::new(buffer.as_slice());
        let data = Self::parse(&mut cursor)?;

        Ok(Self {
            file_path: Some(path.to_path_buf()),
            data,
        })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> PlayerInfoResult<()> {
        let path = path.as_ref();
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        self.write(&mut writer)?;
        writer.flush()?;

        debug!("Saved playerinfo.bin to {}", path.display());
        Ok(())
    }

    pub fn is_valid_file(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        if !path.exists() {
            return false;
        }

        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() < 20 || metadata.len() > 10000 {
                return false;
            }
        } else {
            return false;
        }

        match Self::load(path) {
            Ok(info) => !info.data.first_name.is_empty(),
            Err(_) => false,
        }
    }

    pub fn get_player_name(path: impl AsRef<Path>) -> PlayerInfoResult<String> {
        let path = path.as_ref();
        let mut file = File::open(path)?;

        let mut len_bytes = [0u8; 4];
        file.read_exact(&mut len_bytes)?;
        let length = u32::from_le_bytes(len_bytes) as usize;

        if length == 0 {
            return Err(PlayerInfoParseError::InvalidString {
                position: 0,
                reason: "Empty name".to_string(),
            });
        }

        if length > 256 {
            return Err(PlayerInfoParseError::InvalidString {
                position: 0,
                reason: format!("Name length {length} exceeds maximum"),
            });
        }

        let mut name_bytes = vec![0u8; length];
        file.read_exact(&mut name_bytes)?;

        let (decoded, _, had_errors) = WINDOWS_1252.decode(&name_bytes);
        if had_errors {
            let (utf8_decoded, _, _) = UTF_8.decode(&name_bytes);
            Ok(utf8_decoded.into_owned())
        } else {
            Ok(decoded.into_owned())
        }
    }

    pub fn update_from_gff_data(
        &mut self,
        fields: &IndexMap<String, GffValue<'_>>,
        subrace_name: &str,
        alignment_name: &str,
        classes: &[(String, u8)],
    ) {
        if let Some(first) = extract_locstring(fields, "FirstName") {
            self.data.first_name = first;
        }
        if let Some(last) = extract_locstring(fields, "LastName") {
            self.data.last_name = last;
        }

        self.data.name = self.data.display_name();

        self.data.subrace = subrace_name.to_string();
        self.data.alignment = alignment_name.to_string();

        if let Some(good_evil) = extract_byte(fields, "GoodEvil") {
            self.data.alignment_vertical = good_evil_to_axis(good_evil);
        }
        if let Some(law_chaos) = extract_byte(fields, "LawfulChaotic") {
            self.data.alignment_horizontal = law_chaos_to_axis(law_chaos);
        }

        if let Some(bg) = fields.get("CharBackground").and_then(gff_value_to_u32) {
            self.data.background_id = bg;
        }

        if let Some(deity) = extract_string(fields, "Deity") {
            self.data.deity = deity;
        }

        if let Some(str_val) = extract_byte(fields, "Str") {
            self.data.str_score = u32::from(str_val);
        }
        if let Some(dex_val) = extract_byte(fields, "Dex") {
            self.data.dex_score = u32::from(dex_val);
        }
        if let Some(con_val) = extract_byte(fields, "Con") {
            self.data.con_score = u32::from(con_val);
        }
        if let Some(int_val) = extract_byte(fields, "Int") {
            self.data.int_score = u32::from(int_val);
        }
        if let Some(wis_val) = extract_byte(fields, "Wis") {
            self.data.wis_score = u32::from(wis_val);
        }
        if let Some(cha_val) = extract_byte(fields, "Cha") {
            self.data.cha_score = u32::from(cha_val);
        }

        self.data.str_mod = calculate_modifier(self.data.str_score as i32);
        self.data.dex_mod = calculate_modifier(self.data.dex_score as i32);
        self.data.con_mod = calculate_modifier(self.data.con_score as i32);
        self.data.int_mod = calculate_modifier(self.data.int_score as i32);
        self.data.wis_mod = calculate_modifier(self.data.wis_score as i32);
        self.data.cha_mod = calculate_modifier(self.data.cha_score as i32);

        self.data.classes = classes
            .iter()
            .map(|(name, level)| PlayerClassEntry::new(name.clone(), *level))
            .collect();
    }

    fn parse(cursor: &mut Cursor<&[u8]>) -> PlayerInfoResult<PlayerInfoData> {
        let mut data = PlayerInfoData::new();

        data.first_name = read_string(cursor)?;

        let has_last_name = Self::detect_last_name_presence(cursor)?;

        if has_last_name {
            data.last_name = read_string(cursor)?;
        }

        data.name = if data.last_name.is_empty() {
            data.first_name.clone()
        } else {
            format!("{} {}", data.first_name, data.last_name)
        };

        if !has_last_name {
            data.unknown1 = cursor.read_u32::<LittleEndian>()?;
        }

        data.subrace = read_string(cursor)?;
        data.alignment = read_string(cursor)?;

        data.alignment_vertical = cursor.read_u32::<LittleEndian>()?;
        data.alignment_horizontal = cursor.read_u32::<LittleEndian>()?;
        data.background_id = cursor.read_u32::<LittleEndian>()?;

        let class_count = cursor.read_u32::<LittleEndian>()?;
        if class_count > 20 {
            return Err(PlayerInfoParseError::InvalidFormat(format!(
                "Invalid class count: {class_count}"
            )));
        }

        for i in 0..class_count as usize {
            let class_name = read_string(cursor)?;
            let class_level = cursor.read_u8()?;

            if class_level > 60 {
                warn!(
                    "Unusual class level {} for class {} at index {}",
                    class_level, class_name, i
                );
            }

            data.classes
                .push(PlayerClassEntry::new(class_name, class_level));
        }

        data.deity = read_string(cursor)?;

        data.str_score = cursor.read_u32::<LittleEndian>()?;
        data.dex_score = cursor.read_u32::<LittleEndian>()?;
        data.con_score = cursor.read_u32::<LittleEndian>()?;
        data.int_score = cursor.read_u32::<LittleEndian>()?;
        data.wis_score = cursor.read_u32::<LittleEndian>()?;
        data.cha_score = cursor.read_u32::<LittleEndian>()?;

        if cursor.position() < cursor.get_ref().len() as u64 {
            data.str_mod = cursor.read_i32::<LittleEndian>().unwrap_or(0);
            data.dex_mod = cursor.read_i32::<LittleEndian>().unwrap_or(0);
            data.con_mod = cursor.read_i32::<LittleEndian>().unwrap_or(0);
            data.int_mod = cursor.read_i32::<LittleEndian>().unwrap_or(0);
            data.wis_mod = cursor.read_i32::<LittleEndian>().unwrap_or(0);
            data.cha_mod = cursor.read_i32::<LittleEndian>().unwrap_or(0);
        }

        Ok(data)
    }

    fn detect_last_name_presence(cursor: &mut Cursor<&[u8]>) -> PlayerInfoResult<bool> {
        let start_pos = cursor.position();
        let data_len = cursor.get_ref().len();

        if (start_pos as usize) + 4 > data_len {
            return Ok(false);
        }

        let peek_value = cursor.read_u32::<LittleEndian>()?;

        if peek_value == 0 {
            cursor.seek(SeekFrom::Start(start_pos))?;
            return Ok(false);
        }

        if (2..=50).contains(&peek_value) {
            let string_end = start_pos as usize + 4 + peek_value as usize;
            if string_end <= data_len {
                let data = cursor.get_ref();
                let string_bytes = &data[(start_pos as usize + 4)..string_end];

                let printable_count = string_bytes
                    .iter()
                    .filter(|&&b| (32..127).contains(&b) || b >= 128)
                    .count();

                let ratio = if string_bytes.is_empty() {
                    0.0
                } else {
                    printable_count as f64 / string_bytes.len() as f64
                };

                if ratio >= 0.8 {
                    cursor.seek(SeekFrom::Start(start_pos))?;
                    return Ok(true);
                }
            }
        }

        cursor.seek(SeekFrom::Start(start_pos))?;
        Ok(false)
    }

    fn write(&self, writer: &mut impl Write) -> PlayerInfoResult<()> {
        write_string(writer, &self.data.first_name)?;

        let has_last_name = !self.data.last_name.is_empty();

        if has_last_name {
            write_string(writer, &self.data.last_name)?;
        }

        if !has_last_name {
            writer.write_u32::<LittleEndian>(self.data.unknown1)?;
        }

        write_string(writer, &self.data.subrace)?;
        write_string(writer, &self.data.alignment)?;
        writer.write_u32::<LittleEndian>(self.data.alignment_vertical)?;
        writer.write_u32::<LittleEndian>(self.data.alignment_horizontal)?;
        writer.write_u32::<LittleEndian>(self.data.background_id)?;

        writer.write_u32::<LittleEndian>(self.data.classes.len() as u32)?;
        for class in &self.data.classes {
            write_string(writer, &class.name)?;
            writer.write_u8(class.level)?;
        }

        write_string(writer, &self.data.deity)?;

        writer.write_u32::<LittleEndian>(self.data.str_score)?;
        writer.write_u32::<LittleEndian>(self.data.dex_score)?;
        writer.write_u32::<LittleEndian>(self.data.con_score)?;
        writer.write_u32::<LittleEndian>(self.data.int_score)?;
        writer.write_u32::<LittleEndian>(self.data.wis_score)?;
        writer.write_u32::<LittleEndian>(self.data.cha_score)?;

        writer.write_i32::<LittleEndian>(self.data.str_mod)?;
        writer.write_i32::<LittleEndian>(self.data.dex_mod)?;
        writer.write_i32::<LittleEndian>(self.data.con_mod)?;
        writer.write_i32::<LittleEndian>(self.data.int_mod)?;
        writer.write_i32::<LittleEndian>(self.data.wis_mod)?;
        writer.write_i32::<LittleEndian>(self.data.cha_mod)?;

        Ok(())
    }
}

impl Default for PlayerInfo {
    fn default() -> Self {
        Self::new()
    }
}

fn read_string(cursor: &mut Cursor<&[u8]>) -> PlayerInfoResult<String> {
    let len = cursor.read_u32::<LittleEndian>()? as usize;

    if len == 0 {
        return Ok(String::new());
    }

    if len > 1000 {
        return Err(PlayerInfoParseError::InvalidString {
            position: cursor.position(),
            reason: format!("String length too large: {len}"),
        });
    }

    let mut bytes = vec![0u8; len];
    cursor.read_exact(&mut bytes)?;

    let (decoded, _, had_errors) = WINDOWS_1252.decode(&bytes);
    if had_errors {
        let (utf8_decoded, _, _) = UTF_8.decode(&bytes);
        Ok(utf8_decoded.into_owned())
    } else {
        Ok(decoded.into_owned())
    }
}

fn write_string(writer: &mut impl Write, s: &str) -> PlayerInfoResult<()> {
    let (encoded, _, _) = WINDOWS_1252.encode(s);
    let bytes = encoded.as_ref();

    writer.write_u32::<LittleEndian>(bytes.len() as u32)?;
    writer.write_all(bytes)?;

    Ok(())
}

fn extract_string(fields: &IndexMap<String, GffValue<'_>>, key: &str) -> Option<String> {
    match fields.get(key)? {
        GffValue::String(s) | GffValue::ResRef(s) => Some(s.to_string()),
        _ => None,
    }
}

fn extract_locstring(fields: &IndexMap<String, GffValue<'_>>, key: &str) -> Option<String> {
    match fields.get(key)? {
        GffValue::String(s) => Some(s.to_string()),
        GffValue::LocString(ls) => ls.substrings.first().map(|sub| sub.string.to_string()),
        _ => None,
    }
}

fn extract_byte(fields: &IndexMap<String, GffValue<'_>>, key: &str) -> Option<u8> {
    match fields.get(key)? {
        GffValue::Byte(v) => Some(*v),
        GffValue::Word(v) => Some(*v as u8),
        GffValue::Dword(v) => Some(*v as u8),
        GffValue::Int(v) => Some(*v as u8),
        _ => None,
    }
}

/// Good/Evil GFF byte (0-100) -> playerinfo.bin encoding: 4=Good, 1=Neutral, 5=Evil.
fn good_evil_to_axis(good_evil: u8) -> u32 {
    let v = i32::from(good_evil);
    if v >= ALIGNMENT_GOOD_THRESHOLD {
        4
    } else if v <= ALIGNMENT_EVIL_THRESHOLD {
        5
    } else {
        1
    }
}

/// Law/Chaos GFF byte (0-100) -> playerinfo.bin encoding: 2=Lawful, 1=Neutral, 3=Chaotic.
fn law_chaos_to_axis(law_chaos: u8) -> u32 {
    let v = i32::from(law_chaos);
    if v >= ALIGNMENT_GOOD_THRESHOLD {
        2
    } else if v <= ALIGNMENT_EVIL_THRESHOLD {
        3
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_playerinfo() {
        let info = PlayerInfo::new();
        assert!(info.file_path.is_none());
        assert_eq!(info.data.str_score, 10);
    }

    #[test]
    fn test_string_encoding() {
        let mut buffer = Vec::new();
        write_string(&mut buffer, "Test").unwrap();

        let mut cursor = Cursor::new(buffer.as_slice());
        let result = read_string(&mut cursor).unwrap();

        assert_eq!(result, "Test");
    }

    #[test]
    fn test_empty_string_encoding() {
        let mut buffer = Vec::new();
        write_string(&mut buffer, "").unwrap();

        let mut cursor = Cursor::new(buffer.as_slice());
        let result = read_string(&mut cursor).unwrap();

        assert_eq!(result, "");
    }
}
