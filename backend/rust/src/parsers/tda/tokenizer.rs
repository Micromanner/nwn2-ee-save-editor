use smallvec::SmallVec;

use super::error::{TDAError, TDAResult};

/// High-performance tokenizer for 2DA format with SIMD optimizations where available
pub struct TDATokenizer {
    /// Current line number for error reporting
    line_number: usize,
}

/// Token represents a parsed element from a 2DA line
#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> {
    /// The token content as a string slice
    pub content: &'a str,
    /// Whether this token was quoted
    pub was_quoted: bool,
    /// Position in the original line
    pub position: usize,
}

/// Result of tokenizing a single line
pub type LineTokens<'a> = SmallVec<[Token<'a>; 16]>;

impl TDATokenizer {
    /// Create a new tokenizer
    pub fn new() -> Self {
        Self {
            line_number: 0,
        }
    }


    /// Tokenize a single line with optimized parsing
    pub fn tokenize_line<'a>(&mut self, line: &'a str) -> TDAResult<LineTokens<'a>> {
        self.line_number += 1;
        
        // Skip empty lines and comments
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(SmallVec::new());
        }

        // Check for tab-separated format (preserves empty fields)
        if line.contains('\t') {
            self.tokenize_tab_separated(line)
        } else {
            self.tokenize_space_separated(line)
        }
    }

    /// Tokenize tab-separated line (skips empty fields from consecutive tabs, handles quoted strings)
    /// Split by tabs first, then process each field - avoids quote-tracking bugs
    fn tokenize_tab_separated<'a>(&self, line: &'a str) -> TDAResult<LineTokens<'a>> {
        let mut tokens = SmallVec::new();
        let mut position = 0;

        for field in line.split('\t') {
            let trimmed = field.trim();

            if !trimmed.is_empty() {
                let token = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
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
            }

            position += field.len() + 1; // +1 for the tab
        }

        Ok(tokens)
    }

    /// Tokenize space-separated line
    pub fn tokenize_space_separated<'a>(&self, line: &'a str) -> TDAResult<LineTokens<'a>> {
        self.tokenize_quoted_part(line, 0)
    }

    /// Tokenize a part that may contain quoted strings and spaces
    fn tokenize_quoted_part<'a>(&self, input: &'a str, base_position: usize) -> TDAResult<LineTokens<'a>> {
        let mut tokens = SmallVec::new();
        let mut chars = input.char_indices().peekable();

        while let Some((start_idx, ch)) = chars.next() {
            let position = base_position + start_idx;

            // Skip whitespace
            if ch.is_whitespace() {
                continue;
            }

            if ch == '"' {
                // Handle quoted string
                let (token, end_pos) = self.parse_quoted_string(input, start_idx)?;
                tokens.push(Token {
                    content: token,
                    was_quoted: true,
                    position,
                });
                
                // Skip to end position
                while chars.peek().map(|(idx, _)| *idx < end_pos).unwrap_or(false) {
                    chars.next();
                }
            } else {
                // Handle unquoted token
                let (token, end_pos) = self.parse_unquoted_token(input, start_idx);
                tokens.push(Token {
                    content: token,
                    was_quoted: false,
                    position,
                });
                
                // Skip to end position
                while chars.peek().map(|(idx, _)| *idx < end_pos).unwrap_or(false) {
                    chars.next();
                }
            }
        }

        Ok(tokens)
    }

    /// Parse a quoted string, handling escape sequences
    fn parse_quoted_string<'a>(&self, input: &'a str, start: usize) -> TDAResult<(&'a str, usize)> {
        let bytes = input.as_bytes();
        let mut pos = start + 1; // Skip opening quote
        let mut found_closing = false;

        // Find closing quote
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

        // Extract content between quotes
        let content = &input[start + 1..pos];
        Ok((content, pos + 1))
    }

    /// Parse an unquoted token (stops at whitespace or quotes)
    fn parse_unquoted_token<'a>(&self, input: &'a str, start: usize) -> (&'a str, usize) {
        let bytes = input.as_bytes();
        let mut pos = start;

        // Find end of token (whitespace or quote)
        while pos < bytes.len() {
            let ch = bytes[pos];
            if ch.is_ascii_whitespace() || ch == b'"' {
                break;
            }
            pos += 1;
        }

        (&input[start..pos], pos)
    }



    /// High-performance line validation
    pub fn validate_line(&self, line: &str, max_length: usize) -> TDAResult<()> {
        if line.len() > max_length {
            return Err(TDAError::SecurityViolation {
                details: format!("Line {} exceeds maximum length {}", self.line_number, max_length),
            });
        }

        // UTF-8 validation is guaranteed by Rust's &str type, no need to check

        Ok(())
    }

    /// Get current line number for error reporting
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
        let tokens = tokenizer.tokenize_line(r#"hello "quoted string" test"#).unwrap();
        
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].content, "hello");
        assert_eq!(tokens[1].content, "quoted string");
        assert_eq!(tokens[1].was_quoted, true);
        assert_eq!(tokens[2].content, "test");
    }

    #[test]
    fn test_tab_separated() {
        let mut tokenizer = TDATokenizer::new();
        // Empty fields from consecutive tabs should be skipped - 2DA uses **** for nulls
        let tokens = tokenizer.tokenize_line("col1\tcol2\t\tcol4").unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].content, "col1");
        assert_eq!(tokens[1].content, "col2");
        assert_eq!(tokens[2].content, "col4");
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

    #[test]
    fn test_tab_separated_with_quoted_commas() {
        let mut tokenizer = TDATokenizer::new();
        // Quoted field containing commas - common in NWN2 2DA files
        let tokens = tokenizer.tokenize_line("0\tTableName\t\",value with comma\"\t****").unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].content, "0");
        assert_eq!(tokens[1].content, "TableName");
        assert_eq!(tokens[2].content, ",value with comma");
        assert_eq!(tokens[2].was_quoted, true);
        assert_eq!(tokens[3].content, "****");
    }

    #[test]
    fn test_tab_separated_multiple_quoted_fields() {
        let mut tokenizer = TDATokenizer::new();
        // Multiple quoted fields with commas
        let tokens = tokenizer.tokenize_line("1\tName\t\"a,b,c\"\t\"x,y\"").unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[2].content, "a,b,c");
        assert_eq!(tokens[3].content, "x,y");
    }

    #[test]
    fn test_tab_separated_unquoted_with_internal_quotes() {
        let mut tokenizer = TDATokenizer::new();
        // Field that doesn't start with quote but has quote inside - treat as literal
        let tokens = tokenizer.tokenize_line("0\tSome\"Value\t****").unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].content, "Some\"Value");
        assert_eq!(tokens[1].was_quoted, false);
    }
}