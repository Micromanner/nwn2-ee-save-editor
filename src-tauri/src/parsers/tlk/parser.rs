use super::error::{TLKError, TLKResult};
use super::types::{TLKParser, TLKStringEntry};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::utils::text_encoding::decode_localized_text;

const HEADER_SIZE: usize = 20;
const ENTRY_SIZE: usize = 40;
const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;
const MAX_STRINGS: usize = 1_000_000;

struct Header {
    language_id: u32,
    string_count: usize,
    string_data_offset: usize,
}

fn le_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4-byte slice"))
}

fn parse_header(data: &[u8]) -> TLKResult<Header> {
    if data.len() < HEADER_SIZE {
        return Err(TLKError::FileTooShort {
            expected: HEADER_SIZE,
            actual: data.len(),
        });
    }
    if &data[..8] != b"TLK V3.0" {
        return Err(TLKError::InvalidHeader {
            found: String::from_utf8_lossy(&data[..8]).into_owned(),
        });
    }
    Ok(Header {
        language_id: le_u32(data, 8),
        string_count: le_u32(data, 12) as usize,
        string_data_offset: le_u32(data, 16) as usize,
    })
}

impl TLKParser {
    pub fn parse_from_file<P: AsRef<Path>>(&mut self, path: P) -> TLKResult<()> {
        let data = std::fs::read(path)?;
        self.parse_from_bytes(&data)
    }

    pub fn parse_from_bytes(&mut self, data: &[u8]) -> TLKResult<()> {
        *self = Self::default();

        if data.len() > MAX_FILE_SIZE {
            return Err(TLKError::FileSizeExceeded {
                actual: data.len(),
                limit: MAX_FILE_SIZE,
            });
        }

        let header = parse_header(data)?;
        if header.string_count > MAX_STRINGS {
            return Err(TLKError::TooManyStrings {
                count: header.string_count,
                limit: MAX_STRINGS,
            });
        }

        let table_end = HEADER_SIZE + header.string_count * ENTRY_SIZE;
        // The string data offset equals the file length when every string is empty.
        let required = table_end.max(header.string_data_offset);
        if required > data.len() {
            return Err(TLKError::FileTooShort {
                expected: required,
                actual: data.len(),
            });
        }

        // Entry: flags, sound resref (16), volume and pitch variance, data
        // offset, string size, sound length.
        self.entries = data[HEADER_SIZE..table_end]
            .as_chunks::<ENTRY_SIZE>()
            .0
            .iter()
            .map(|entry| TLKStringEntry {
                flags: le_u32(entry, 0),
                data_offset: le_u32(entry, 28),
                string_size: le_u32(entry, 32),
            })
            .collect();
        self.string_data = data[header.string_data_offset..].to_vec();
        self.language_id = header.language_id;

        Ok(())
    }

    /// String count from the header alone, without parsing the table.
    pub fn string_count_of_file<P: AsRef<Path>>(path: P) -> TLKResult<usize> {
        let mut header = [0u8; HEADER_SIZE];
        File::open(path)?.read_exact(&mut header)?;
        Ok(parse_header(&header)?.string_count)
    }

    /// The string at `str_ref`; empty for entries without text, `None` when
    /// out of range or the entry points outside the string data.
    pub fn read_string(&self, str_ref: usize) -> Option<String> {
        let entry = self.entries.get(str_ref)?;
        if !entry.is_present() || entry.string_size == 0 {
            return Some(String::new());
        }

        let start = entry.data_offset as usize;
        let end = start.checked_add(entry.string_size as usize)?;
        let string_bytes = self.string_data.get(start..end)?;

        // EE TLKs are UTF-8, pre-EE TLKs use the code page of the header language.
        Some(decode_localized_text(string_bytes, self.language_id))
    }

    /// A TLK holding `strings` as strrefs 0.., for tests.
    #[cfg(test)]
    pub fn from_strings(strings: &[&str]) -> Self {
        let bytes: Vec<&[u8]> = strings.iter().map(|s| s.as_bytes()).collect();
        let mut parser = Self::new();
        parser
            .parse_from_bytes(&tlk_bytes(&bytes))
            .expect("valid test TLK");
        parser
    }
}

#[cfg(test)]
fn tlk_bytes(strings: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TLK V3.0");
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    out.extend_from_slice(&((HEADER_SIZE + ENTRY_SIZE * strings.len()) as u32).to_le_bytes());
    let mut offset = 0u32;
    for text in strings {
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&[0u8; 16 + 8]);
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&(text.len() as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 4]);
        offset += text.len() as u32;
    }
    for text in strings {
        out.extend_from_slice(text);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_strings() {
        let mut tlk = TLKParser::new();
        tlk.parse_from_bytes(&tlk_bytes(&[b"Bad Strref", b"Hello"]))
            .unwrap();

        assert_eq!(tlk.string_count(), 2);
        assert_eq!(tlk.read_string(1).as_deref(), Some("Hello"));
        assert_eq!(tlk.read_string(2), None);
    }

    #[test]
    fn accepts_empty_string_data_section() {
        let mut tlk = TLKParser::new();
        tlk.parse_from_bytes(&tlk_bytes(&[b"", b""])).unwrap();
        assert_eq!(tlk.read_string(1).as_deref(), Some(""));

        tlk.parse_from_bytes(&tlk_bytes(&[])).unwrap();
        assert_eq!(tlk.string_count(), 0);
    }

    #[test]
    fn accepts_strings_over_64_kib() {
        let long = vec![b'a'; 70 * 1024];
        let mut tlk = TLKParser::new();
        tlk.parse_from_bytes(&tlk_bytes(&[&long])).unwrap();

        assert_eq!(tlk.read_string(0).map(|s| s.len()), Some(long.len()));
    }

    #[test]
    fn entry_outside_string_data_reads_none() {
        let mut bytes = tlk_bytes(&[b"abc"]);
        bytes.truncate(bytes.len() - 1);
        let mut tlk = TLKParser::new();
        tlk.parse_from_bytes(&bytes).unwrap();

        assert_eq!(tlk.read_string(0), None);
    }
}
