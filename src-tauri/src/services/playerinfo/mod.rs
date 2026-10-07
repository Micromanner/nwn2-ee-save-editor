pub mod error;
pub mod types;

use std::io::{Cursor, Read, Write};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use tracing::warn;

use crate::character::types::{AbilityIndex, calculate_modifier};
use crate::character::{Character, resolve_localized_name};
use crate::loaders::GameData;
use crate::utils::text_encoding::decode_text;

pub use error::{PlayerInfoParseError, PlayerInfoResult};
pub use types::{PlayerClassEntry, PlayerInfoData};

/// playerinfo.bin encoding of `Alignment::axis_indices`.
const LAW_CHAOS_AXIS: [u32; 3] = [2, 1, 3];
const GOOD_EVIL_AXIS: [u32; 3] = [4, 1, 5];

#[derive(Default)]
pub struct PlayerInfo {
    pub data: PlayerInfoData,
}

impl PlayerInfo {
    pub fn load(path: impl AsRef<Path>) -> PlayerInfoResult<Self> {
        let buffer = std::fs::read(path)?;
        let data = Self::parse(&mut Cursor::new(buffer.as_slice()))?;
        Ok(Self { data })
    }

    /// Full character name (first + last). Falls back to the leading first-name
    /// string when the rest of the file does not parse.
    pub fn get_player_name(path: impl AsRef<Path>) -> PlayerInfoResult<String> {
        let buffer = std::fs::read(path)?;
        let name = match Self::parse(&mut Cursor::new(buffer.as_slice())) {
            Ok(data) => data.full_name(),
            Err(_) => read_string(&mut Cursor::new(buffer.as_slice()))?,
        };
        if name.trim().is_empty() {
            return Err(PlayerInfoParseError::InvalidString {
                position: 0,
                reason: "Empty name".to_string(),
            });
        }
        Ok(name)
    }

    /// Load-menu summary as the game writes it: every field comes from the
    /// character, scores are stored + racial adjustment (no items).
    pub fn from_character(character: &Character, game_data: &GameData) -> Self {
        let alignment = character.alignment();
        let (law, good) = alignment.axis_indices();
        let scores = character.get_effective_abilities(game_data);
        let scores = AbilityIndex::all().map(|ability| scores.get(ability));

        let data = PlayerInfoData {
            first_name: resolve_localized_name(
                character.first_name(),
                character.first_name_strref(),
                game_data,
            ),
            last_name: resolve_localized_name(
                character.last_name(),
                character.last_name_strref(),
                game_data,
            ),
            // NWN2 matches the load-menu race icon by this TLK name.
            subrace: character.race_display_name(game_data),
            alignment: alignment.display_name(game_data),
            alignment_vertical: GOOD_EVIL_AXIS[good],
            alignment_horizontal: LAW_CHAOS_AXIS[law],
            background_id: character
                .get_i32("CharBackground")
                .and_then(|id| u32::try_from(id).ok())
                .unwrap_or(0),
            classes: character
                .class_entries()
                .into_iter()
                .map(|entry| {
                    let level = entry.level.clamp(0, i32::from(u8::MAX)) as u8;
                    PlayerClassEntry::new(
                        character.get_class_name(entry.class_id, game_data),
                        level,
                    )
                })
                .collect(),
            deity: character.deity(),
            scores: scores.map(|score| u32::try_from(score).unwrap_or(0)),
            modifiers: scores.map(calculate_modifier),
        };
        Self { data }
    }

    fn parse(cursor: &mut Cursor<&[u8]>) -> PlayerInfoResult<PlayerInfoData> {
        let mut data = PlayerInfoData {
            first_name: read_string(cursor)?,
            last_name: read_string(cursor)?,
            subrace: read_string(cursor)?,
            alignment: read_string(cursor)?,
            alignment_vertical: cursor.read_u32::<LittleEndian>()?,
            alignment_horizontal: cursor.read_u32::<LittleEndian>()?,
            background_id: cursor.read_u32::<LittleEndian>()?,
            ..Default::default()
        };

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

        for score in &mut data.scores {
            *score = cursor.read_u32::<LittleEndian>()?;
        }
        for modifier in &mut data.modifiers {
            *modifier = cursor.read_i32::<LittleEndian>()?;
        }

        Ok(data)
    }

    pub fn write(&self, writer: &mut impl Write) -> PlayerInfoResult<()> {
        write_string(writer, &self.data.first_name)?;
        write_string(writer, &self.data.last_name)?;
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

        for score in self.data.scores {
            writer.write_u32::<LittleEndian>(score)?;
        }
        for modifier in self.data.modifiers {
            writer.write_i32::<LittleEndian>(modifier)?;
        }

        Ok(())
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

    Ok(decode_text(&bytes))
}

fn write_string(writer: &mut impl Write, s: &str) -> PlayerInfoResult<()> {
    let bytes = s.as_bytes();

    writer.write_u32::<LittleEndian>(bytes.len() as u32)?;
    writer.write_all(bytes)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
