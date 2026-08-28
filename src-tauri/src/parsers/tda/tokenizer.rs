use smallvec::SmallVec;

use super::error::{TDAError, TDAResult};

pub struct TDATokenizer {
    line_number: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> {
    pub content: &'a str,
    pub was_quoted: bool,
    pub position: usize,
}

pub type LineTokens<'a> = SmallVec<[Token<'a>; 16]>;

impl TDATokenizer {
    pub fn new() -> Self {
        Self { line_number: 0 }
    }

    pub fn tokenize_line<'a>(&mut self, line: &'a str) -> TDAResult<LineTokens<'a>> {
        self.line_number += 1;

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(SmallVec::new());
        }

        // NWN2 2DA files use both tab-separated and space-separated formats.
        // Analysis of NWN2 EE base game shows:
        // - 719 out of 1165 files (61.7%) use tabs
        // - Important files like spells.2da, classes.2da, baseitems.2da all use tabs
        // Therefore this detection is critical for correct parsing.
        if line.contains('\t') {
            Self::tokenize_tab_separated(line)
        } else {
            self.tokenize_space_separated(line)
        }
    }

    fn tokenize_tab_separated<'a>(line: &'a str) -> TDAResult<LineTokens<'a>> {
        let mut tokens: LineTokens<'a> = SmallVec::new();
        let mut position = 0;

        for field in line.split('\t') {
            let trimmed = field.trim();

            let token = if trimmed.is_empty() {
                Token {
                    content: trimmed,
                    was_quoted: false,
                    position,
                }
            } else if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                Token {
                    content: &trimmed[1..trimmed.len() - 1],
                    was_quoted: true,
                    position,
                }
            } else {
                Token {
                    content: trimmed,
                    was_quoted: false,
                    position,
                }
            };
            tokens.push(token);

            position += field.len() + 1;
        }

        // The engine splits on whitespace runs, so tabs at either end of a line
        // are not cells. Some third-party 2DAs (e.g. workshop overrides) indent
        // every data row with a leading tab; keeping that empty field would make
        // it the row label and shift every cell one column left.
        let leading = tokens.iter().take_while(|t| t.content.is_empty()).count();
        tokens.drain(..leading);
        while tokens.last().is_some_and(|t| t.content.is_empty()) {
            tokens.pop();
        }

        Ok(tokens)
    }

    pub fn tokenize_space_separated<'a>(&self, line: &'a str) -> TDAResult<LineTokens<'a>> {
        self.tokenize_quoted_part(line, 0)
    }

    fn tokenize_quoted_part<'a>(
        &self,
        input: &'a str,
        base_position: usize,
    ) -> TDAResult<LineTokens<'a>> {
        let mut tokens = SmallVec::new();
        let mut chars = input.char_indices().peekable();

        while let Some((start_idx, ch)) = chars.next() {
            let position = base_position + start_idx;

            if ch.is_whitespace() {
                continue;
            }

            if ch == '"' {
                let (token, end_pos) = self.parse_quoted_string(input, start_idx)?;
                tokens.push(Token {
                    content: token,
                    was_quoted: true,
                    position,
                });

                while chars.peek().is_some_and(|(idx, _)| *idx < end_pos) {
                    chars.next();
                }
            } else {
                let (token, end_pos) = Self::parse_unquoted_token(input, start_idx);
                tokens.push(Token {
                    content: token,
                    was_quoted: false,
                    position,
                });

                while chars.peek().is_some_and(|(idx, _)| *idx < end_pos) {
                    chars.next();
                }
            }
        }

        Ok(tokens)
    }

    fn parse_quoted_string<'a>(&self, input: &'a str, start: usize) -> TDAResult<(&'a str, usize)> {
        let bytes = input.as_bytes();
        let mut pos = start + 1;
        let mut found_closing = false;

        while pos < bytes.len() {
            if bytes[pos] == b'"' {
                found_closing = true;
                break;
            }
            pos += 1;
        }

        if !found_closing {
            return Err(TDAError::InvalidToken {
                position: start,
                token: input[start..].to_string(),
            });
        }

        let content = &input[start + 1..pos];
        Ok((content, pos + 1))
    }

    fn parse_unquoted_token<'a>(input: &'a str, start: usize) -> (&'a str, usize) {
        let bytes = input.as_bytes();
        let mut pos = start;

        while pos < bytes.len() {
            let ch = bytes[pos];
            if ch.is_ascii_whitespace() || ch == b'"' {
                break;
            }
            pos += 1;
        }

        (&input[start..pos], pos)
    }

    pub fn validate_line(&self, line: &str, max_length: usize) -> TDAResult<()> {
        if line.len() > max_length {
            return Err(TDAError::SecurityViolation {
                details: format!(
                    "Line {} exceeds maximum length {}",
                    self.line_number, max_length
                ),
            });
        }

        Ok(())
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }
}

impl Default for TDATokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenization() {
        let mut tokenizer = TDATokenizer::new();
        let tokens = tokenizer.tokenize_line("hello world test").unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].content, "hello");
        assert_eq!(tokens[1].content, "world");
        assert_eq!(tokens[2].content, "test");
    }

    #[test]
    fn test_quoted_tokenization() {
        let mut tokenizer = TDATokenizer::new();
        let tokens = tokenizer
            .tokenize_line(r#"hello "quoted string" test"#)
            .unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].content, "hello");
        assert_eq!(tokens[1].content, "quoted string");
        assert!(tokens[1].was_quoted);
        assert_eq!(tokens[2].content, "test");
    }

    #[test]
    fn test_tab_separated() {
        let mut tokenizer = TDATokenizer::new();
        let tokens = tokenizer.tokenize_line("col1\tcol2\t\tcol4").unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].content, "col1");
        assert_eq!(tokens[1].content, "col2");
        assert_eq!(tokens[2].content, "");
        assert_eq!(tokens[3].content, "col4");
    }

    #[test]
    fn test_tab_separated_ignores_leading_and_trailing_tabs() {
        let mut tokenizer = TDATokenizer::new();
        let tokens = tokenizer.tokenize_line("	16	650	Damage	").unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].content, "16");
        assert_eq!(tokens[2].content, "Damage");
    }

    #[test]
    fn test_empty_line() {
        let mut tokenizer = TDATokenizer::new();
        let tokens = tokenizer.tokenize_line("").unwrap();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_comment_line() {
        let mut tokenizer = TDATokenizer::new();
        let tokens = tokenizer.tokenize_line("# This is a comment").unwrap();
        assert_eq!(tokens.len(), 0);
    }
}
