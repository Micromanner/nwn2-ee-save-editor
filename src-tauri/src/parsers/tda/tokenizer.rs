use smallvec::SmallVec;

type LineTokens<'a> = SmallVec<[&'a str; 16]>;

/// Splits a 2DA line into cells the way the engine does: whitespace runs
/// (tabs and spaces alike) separate cells, `"..."` groups one, and an
/// unterminated quote takes the rest of the line. Empty and `#` comment
/// lines have none.
///
/// Verified in-game: `soundset.2da` row 467 (`Male_Seasoned Fighter_PCV` in
/// a tab-separated file) is missing from the voice list, so a space inside a
/// tab field splits it too.
pub fn tokenize_line(line: &str) -> LineTokens<'_> {
    let mut tokens = SmallVec::new();
    let mut rest = line.trim_start();
    if rest.starts_with('#') {
        return tokens;
    }

    while !rest.is_empty() {
        let end = if let Some(quoted) = rest.strip_prefix('"') {
            let close = quoted.find('"').unwrap_or(quoted.len());
            tokens.push(&quoted[..close]);
            (close + 2).min(rest.len())
        } else {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"')
                .unwrap_or(rest.len());
            tokens.push(&rest[..end]);
            end
        };
        rest = rest[end..].trim_start();
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenization() {
        let tokens = tokenize_line("hello world test");
        assert_eq!(tokens.as_slice(), ["hello", "world", "test"]);
    }

    #[test]
    fn test_quoted_tokenization() {
        let tokens = tokenize_line(r#"hello "quoted string" test"#);
        assert_eq!(tokens.as_slice(), ["hello", "quoted string", "test"]);
    }

    #[test]
    fn test_quote_directly_after_cell_starts_new_cell() {
        let tokens = tokenize_line(r#"a"b c" d"#);
        assert_eq!(tokens.as_slice(), ["a", "b c", "d"]);
    }

    #[test]
    fn test_unterminated_quote_takes_rest_of_line() {
        let tokens = tokenize_line(r#"0 label "Unterminated text here"#);
        assert_eq!(tokens.as_slice(), ["0", "label", "Unterminated text here"]);
    }

    #[test]
    fn test_tabs_are_whitespace() {
        let tokens = tokenize_line("\t16\t650\tDamage\t");
        assert_eq!(tokens.as_slice(), ["16", "650", "Damage"]);
    }

    #[test]
    fn test_quoted_cell_in_tab_line() {
        let tokens = tokenize_line("3\t\"Cure Minor Wounds\"\t****");
        assert_eq!(tokens.as_slice(), ["3", "Cure Minor Wounds", "****"]);
    }

    /// Real vanilla lines that mix tabs and spaces.
    #[test]
    fn test_mixed_tab_and_space_lines() {
        // race_feat_drow.2da
        assert_eq!(
            tokenize_line("0     immunitysleep         \t235").as_slice(),
            ["0", "immunitysleep", "235"]
        );
        // iprp_spellshl.2da header
        assert_eq!(
            tokenize_line("       Name         Label           Cost\tLetter").as_slice(),
            ["Name", "Label", "Cost", "Letter"]
        );
        // packspwiz1.2da: the interior empty tab field is not a cell
        assert_eq!(
            tokenize_line("1\t   116\t\tMeteor_Swarm").as_slice(),
            ["1", "116", "Meteor_Swarm"]
        );
        // soundset.2da row 467
        assert_eq!(
            tokenize_line("467\tMale_Seasoned Fighter_PCV\tvs_PC_Male3\t183656\t0\t0").as_slice(),
            [
                "467",
                "Male_Seasoned",
                "Fighter_PCV",
                "vs_PC_Male3",
                "183656",
                "0",
                "0"
            ]
        );
    }

    #[test]
    fn test_empty_line() {
        assert!(tokenize_line("").is_empty());
        assert!(tokenize_line(" \t ").is_empty());
    }

    #[test]
    fn test_comment_line() {
        assert!(tokenize_line("# This is a comment").is_empty());
    }
}
