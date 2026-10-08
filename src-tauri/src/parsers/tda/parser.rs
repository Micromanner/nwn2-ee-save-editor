use std::path::Path;

use tracing::{instrument, trace};

use super::error::{TDAError, TDAResult};
use super::tokenizer::tokenize_line;
use super::types::{CellValue, TDAParser, TDARow};
use crate::utils::text_encoding::decode_text;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

impl TDAParser {
    /// Parses a 2DA. Text that is not UTF-8 is decoded with the editor
    /// language's legacy code page instead of rejecting the table.
    #[instrument(name = "TDAParser::parse_from_bytes", skip_all, fields(size = data.len()))]
    pub fn parse_from_bytes(&mut self, data: &[u8]) -> TDAResult<()> {
        self.security_limits.validate_file_size(data.len())?;

        *self = Self {
            security_limits: std::mem::take(&mut self.security_limits),
            ..Self::default()
        };

        let data = data.strip_prefix(UTF8_BOM).unwrap_or(data);
        match std::str::from_utf8(data) {
            Ok(text) => self.parse_content(text)?,
            Err(_) => self.parse_content(&decode_text(data))?,
        }

        trace!(
            "2DA parsed: {} rows, {} columns",
            self.row_count(),
            self.column_count()
        );

        Ok(())
    }

    pub fn parse_from_file<P: AsRef<Path>>(&mut self, path: P) -> TDAResult<()> {
        let data = std::fs::read(path)?;
        self.parse_from_bytes(&data)
    }

    fn parse_content(&mut self, content: &str) -> TDAResult<()> {
        for line in content.lines() {
            self.security_limits.validate_line_length(line.len())?;
        }

        let mut lines = content
            .lines()
            .map(|line| (line, tokenize_line(line)))
            .filter(|(_, tokens)| !tokens.is_empty());

        let Some((header, _)) = lines.next() else {
            return Ok(());
        };
        Self::parse_header(header.trim())?;

        if let Some((_, columns)) = lines.next() {
            for column in &columns {
                self.add_column(column);
            }
            self.security_limits
                .validate_column_count(self.column_count())?;
        }

        for (_, tokens) in lines {
            self.parse_data_row(&tokens);
            self.security_limits.validate_row_count(self.row_count())?;
        }

        Ok(())
    }

    fn parse_header(line: &str) -> TDAResult<()> {
        let version = line
            .strip_prefix("2DA")
            .filter(|rest| rest.starts_with([' ', '\t']))
            .and_then(|rest| rest.get(1..5));

        match version {
            Some("V2.0" | "V1.0") => Ok(()),
            _ => Err(TDAError::InvalidHeader(line.to_string())),
        }
    }

    /// The first token is the row label, which the engine ignores (rows are
    /// positional). Missing trailing cells read as `****`.
    fn parse_data_row(&mut self, tokens: &[&str]) {
        let column_count = self.column_count();
        let mut row: TDARow = tokens
            .iter()
            .skip(1)
            .take(column_count)
            .map(|token| CellValue::new(token, &mut self.interner))
            .collect();
        row.resize(column_count, CellValue::Null);
        self.rows.push(row);
    }

    pub fn parse_from_string(&mut self, data: &str) -> TDAResult<()> {
        self.parse_from_bytes(data.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_2DA: &str = r#"2DA V2.0

Label       Name        Description
0           test1       "Test Item 1"
1           test2       "Test Item 2"
2           ****        "Empty Label"
"#;

    #[test]
    fn test_basic_parsing() {
        let mut parser = TDAParser::new();
        parser.parse_from_string(SAMPLE_2DA).unwrap();

        assert_eq!(parser.column_count(), 3);
        assert_eq!(parser.row_count(), 3);

        let columns = parser.column_names();
        assert_eq!(columns, vec!["Label", "Name", "Description"]);
    }

    #[test]
    fn test_cell_access() {
        let mut parser = TDAParser::new();
        parser.parse_from_string(SAMPLE_2DA).unwrap();

        assert_eq!(parser.get_cell(0, 0).unwrap(), Some("test1"));
        assert_eq!(parser.get_cell(0, 1).unwrap(), Some("Test Item 1"));
        assert_eq!(parser.get_cell(1, 1).unwrap(), Some("Test Item 2"));

        assert_eq!(parser.get_cell_by_name(0, "Label").unwrap(), Some("test1"));
        assert_eq!(
            parser.get_cell_by_name(0, "Name").unwrap(),
            Some("Test Item 1")
        );
        assert_eq!(parser.get_cell_by_name(2, "Label").unwrap(), None);
    }

    #[test]
    fn test_short_row_pads_with_null() {
        let mut parser = TDAParser::new();
        parser
            .parse_from_string("2DA V2.0\n\nA B C\n0 x\n1 y \"\" z\n")
            .unwrap();

        assert_eq!(parser.get_cell(0, 0).unwrap(), Some("x"));
        assert_eq!(parser.get_cell(0, 1).unwrap(), None);
        assert_eq!(parser.get_cell(0, 2).unwrap(), None);
        assert_eq!(parser.get_cell(1, 1).unwrap(), Some(""));
    }

    #[test]
    fn test_bom_is_stripped() {
        let mut parser = TDAParser::new();
        let data = [UTF8_BOM, b"2DA V2.0\n\nA B\n0 x y\n"].concat();
        parser.parse_from_bytes(&data).unwrap();

        assert_eq!(parser.get_cell_by_name(0, "B").unwrap(), Some("y"));
    }

    #[test]
    fn test_non_utf8_cell_does_not_reject_table() {
        let mut parser = TDAParser::new();
        parser
            .parse_from_bytes(b"2DA V2.0\n\nLabel Value\n0 Caf\xE9 1\n")
            .unwrap();

        assert_eq!(parser.row_count(), 1);
        assert_eq!(parser.get_cell_by_name(0, "Value").unwrap(), Some("1"));
    }

    #[test]
    fn test_invalid_header_rejected() {
        let mut parser = TDAParser::new();
        assert!(parser.parse_from_string("2DAM V2.0\n\nA\n").is_err());
        assert!(parser.parse_from_string("2DA V3.0\n\nA\n").is_err());
        assert!(parser.parse_from_string("2DA\tV2.0\t\t\n\nA\n").is_ok());
    }

    #[test]
    fn test_security_limits() {
        let mut parser = TDAParser::new();
        parser.security_limits.max_file_size = 100;
        let large_data = "x".repeat(200);

        assert!(parser.parse_from_string(&large_data).is_err());
    }
}
