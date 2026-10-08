use super::super::common::create_test_context;
use app_lib::parsers::tda::TDAParser;

// =============================================================================
// BASIC 2DA PARSING TESTS
// =============================================================================

#[tokio::test]
async fn test_2da_classes_basic() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");
    assert!(table.row_count() > 0, "classes.2da should have rows");
    assert_eq!(table.name, "classes");
}

#[tokio::test]
async fn test_2da_column_names() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");
    let cols = table.column_names();

    println!("classes.2da columns ({}):", cols.len());
    for col in &cols {
        print!("{col}, ");
    }
    println!();

    assert!(
        cols.iter().any(|c| c.to_lowercase() == "label"),
        "Should have Label column"
    );
    assert!(
        cols.iter().any(|c| c.to_lowercase() == "name"),
        "Should have Name column"
    );
    assert!(
        cols.iter().any(|c| c.to_lowercase() == "hitdie"),
        "Should have HitDie column"
    );
}

#[tokio::test]
async fn test_2da_cell_access_by_name() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");

    let bard_label = table.get_cell(1, "Label").expect("Row 1 Label missing");
    println!("Class 1 Label: {bard_label:?}");
    assert!(
        bard_label
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("bard")
    );
}

#[tokio::test]
async fn test_2da_cell_access_by_index() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");

    let col_idx = table
        .find_column_index("Label")
        .expect("Label column not found");
    let result = table.parser.get_cell(0, col_idx);
    assert!(result.is_ok(), "Should be able to access by column index");
    println!("Row 0, Column {}: {:?}", col_idx, result.unwrap());
}

// =============================================================================
// ROW ITERATION TESTS
// =============================================================================

#[tokio::test]
async fn test_2da_row_dict() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");

    let row0 = table.get_row(0).expect("Failed to get row dict");
    println!("Row 0 as dict: {row0:?}");
    assert!(!row0.is_empty(), "Row dict should not be empty");
}

#[tokio::test]
async fn test_2da_all_rows_dict() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");

    let all_rows = table.parser.get_all_rows_dict();
    println!("Total rows: {}", all_rows.len());
    assert!(!all_rows.is_empty(), "Should have rows");
    assert_eq!(all_rows.len(), table.row_count());
}

// =============================================================================
// FIND ROW TESTS
// =============================================================================

#[tokio::test]
async fn test_2da_find_row() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");

    let result = table.parser.find_row("Label", "Fighter");
    match result {
        Ok(Some(row_idx)) => println!("Fighter found at row {row_idx}"),
        Ok(None) => println!("Fighter not found"),
        Err(e) => println!("Find error: {e}"),
    }
}

#[tokio::test]
async fn test_2da_find_nonexistent_row() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");

    let result = table.parser.find_row("Label", "NonExistentClass12345");
    assert!(
        matches!(result, Ok(None)),
        "Should return None for nonexistent row"
    );
}

// =============================================================================
// NULL/EMPTY VALUE HANDLING
// =============================================================================

#[tokio::test]
async fn test_2da_null_values() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da not found");

    println!("\n=== Checking for **** (null) values ===");
    for row_idx in 0..table.row_count().min(10) {
        let row = table.get_row(row_idx).unwrap();
        for (col, val) in &row {
            if val.is_none() {
                println!("  Row {row_idx}, {col}: NULL");
            }
        }
    }
}

// =============================================================================
// MULTIPLE TABLE TESTS
// =============================================================================

#[tokio::test]
async fn test_2da_feats_table() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da not found");
    println!("feat.2da has {} rows", table.row_count());
    assert!(table.row_count() > 100, "feat.2da should have many feats");

    let cols = table.column_names();
    assert!(
        cols.iter().any(|c| c.to_lowercase() == "label"),
        "Should have Label"
    );
    assert!(
        cols.iter().any(|c| c.to_lowercase() == "prereqfeat1"),
        "Should have PreReqFeat1"
    );
}

#[tokio::test]
async fn test_2da_skills_table() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("skills")
        .expect("skills.2da not found");
    println!(
        "skills.2da has {} rows, {} columns",
        table.row_count(),
        table.parser.column_count()
    );

    let cols = table.column_names();
    assert!(
        cols.iter().any(|c| c.to_lowercase() == "name"),
        "Should have Name column"
    );
}

#[tokio::test]
async fn test_2da_spells_table() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("spells")
        .expect("spells.2da not found");
    println!("spells.2da has {} rows", table.row_count());
    assert!(
        table.row_count() > 500,
        "spells.2da should have many spells"
    );
}

#[tokio::test]
async fn test_2da_races_table() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da not found");
    println!("racialtypes.2da has {} rows", table.row_count());

    let cols = table.column_names();
    println!("Columns: {cols:?}");
}

#[tokio::test]
async fn test_2da_baseitems_table() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da not found");
    println!("baseitems.2da has {} rows", table.row_count());
    assert!(
        table.row_count() > 50,
        "baseitems.2da should have many items"
    );
}

// =============================================================================
// CLASS FEAT TABLES
// =============================================================================

#[tokio::test]
async fn test_2da_cls_feat_fight() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("cls_feat_fight")
        .expect("cls_feat_fight.2da not found");
    println!("cls_feat_fight.2da has {} rows", table.row_count());

    let cols = table.column_names();
    println!("Columns: {cols:?}");

    assert!(
        cols.iter().any(|c| c.to_lowercase() == "featindex"),
        "Should have FeatIndex"
    );
    assert!(
        cols.iter().any(|c| c.to_lowercase() == "list"),
        "Should have List"
    );
    assert!(
        cols.iter().any(|c| c.to_lowercase() == "grantedonlevel"),
        "Should have GrantedOnLevel"
    );
}

// =============================================================================
// DIRECT PARSING TESTS
// =============================================================================

#[test]
fn test_2da_parse_from_string_basic() {
    let content = r"2DA V2.0

Label       Name        Value
0           test1       10
1           test2       20
2           test3       30
";

    let mut parser = TDAParser::new();
    parser
        .parse_from_string(content)
        .expect("Failed to parse 2DA string");

    assert_eq!(parser.row_count(), 3);
    assert_eq!(parser.column_count(), 3);
}

#[test]
fn test_2da_parse_quoted_values() {
    let content = r#"2DA V2.0

Description
0           "A quoted description with spaces"
1           Simple
"#;

    let mut parser = TDAParser::new();
    parser
        .parse_from_string(content)
        .expect("Failed to parse 2DA string");

    assert_eq!(
        parser.get_cell_by_name(0, "Description").unwrap(),
        Some("A quoted description with spaces")
    );
    assert_eq!(
        parser.get_cell_by_name(1, "Description").unwrap(),
        Some("Simple")
    );
}

#[test]
fn test_2da_parse_null_values() {
    let content = r"2DA V2.0

Value       Optional
0           10          ****
1           ****        Value2
2           30
";

    let mut parser = TDAParser::new();
    parser
        .parse_from_string(content)
        .expect("Failed to parse 2DA string");

    assert_eq!(parser.get_cell_by_name(0, "Value").unwrap(), Some("10"));
    assert_eq!(parser.get_cell_by_name(0, "Optional").unwrap(), None);
    assert_eq!(parser.get_cell_by_name(1, "Value").unwrap(), None);
    assert_eq!(
        parser.get_cell_by_name(1, "Optional").unwrap(),
        Some("Value2")
    );
    assert_eq!(parser.get_cell_by_name(2, "Optional").unwrap(), None);
}

// =============================================================================
// ERROR HANDLING
// =============================================================================

#[test]
fn test_2da_empty_string() {
    let mut parser = TDAParser::new();
    let result = parser.parse_from_string("");
    // Empty string may either error or succeed with empty table
    if result.is_ok() {
        assert_eq!(parser.row_count(), 0, "Empty parse should have no rows");
    }
    println!("Empty string parse result: {:?}", result.is_ok());
}

#[test]
fn test_2da_invalid_header() {
    let content = "NOT A 2DA FILE\nSome random content";
    let mut parser = TDAParser::new();
    let result = parser.parse_from_string(content);
    assert!(result.is_err(), "Invalid header should fail");
}

#[test]
fn test_2da_out_of_bounds_access() {
    let content = r"2DA V2.0

Label       Value
0           10
1           20
";

    let mut parser = TDAParser::new();
    parser.parse_from_string(content).expect("Failed to parse");

    let result = parser.get_cell_by_name(100, "Label");
    assert!(result.is_err(), "Out of bounds row should error");
}

#[test]
fn test_2da_nonexistent_column() {
    let content = r"2DA V2.0

Label       Value
0           10
";

    let mut parser = TDAParser::new();
    parser.parse_from_string(content).expect("Failed to parse");

    let result = parser.get_cell_by_name(0, "NonExistent");
    assert!(result.is_err(), "Nonexistent column should error");
}

// =============================================================================
// ROW COUNT TEST
// =============================================================================

#[test]
fn test_2da_row_count() {
    let content = r"2DA V2.0

Label       Value
0           10
1           20
2           30
";

    let mut parser = TDAParser::new();
    parser.parse_from_string(content).expect("Failed to parse");

    assert_eq!(parser.row_count(), 3);
    assert_eq!(parser.column_count(), 2);
}

#[tokio::test]
async fn test_2da_files_tab_character_usage() {
    use std::path::PathBuf;

    let _ctx = create_test_context().await;

    let nwn2_data_dir =
        PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/NWN2 Enhanced Edition/Data");

    if !nwn2_data_dir.exists() {
        println!(
            "WARNING: NWN2 install not found at expected location, skipping tab detection test"
        );
        return;
    }

    const BASE_GAME_ZIPS: &[&str] = &["2da.zip", "2da_x1.zip", "2da_x2.zip"];

    let mut total_files = 0;
    let mut files_with_tabs = Vec::new();
    let mut files_without_tabs = 0;

    println!("\n=== Scanning NWN2 2DA files for tab character usage ===\n");

    for zip_name in BASE_GAME_ZIPS {
        let zip_path = nwn2_data_dir.join(zip_name);
        if !zip_path.exists() {
            println!("Skipping {zip_name}, file not found");
            continue;
        }

        println!("Scanning {zip_name} ...");

        let file = std::fs::File::open(&zip_path).expect("Failed to open zip");
        let mut archive = zip::ZipArchive::new(file).expect("Failed to read zip");

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).expect("Failed to read entry");
            let name = entry.name().to_string();

            if !name.to_lowercase().ends_with(".2da") {
                continue;
            }

            total_files += 1;

            let mut contents = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut contents).expect("Failed to read file");

            let text = String::from_utf8_lossy(&contents);

            if text.contains('\t') {
                files_with_tabs.push((zip_name.to_string(), name.clone()));

                let lines_with_tabs: Vec<usize> = text
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.contains('\t'))
                    .map(|(idx, _)| idx + 1)
                    .collect();

                println!("  TABS FOUND: {name} (lines: {lines_with_tabs:?})");
            } else {
                files_without_tabs += 1;
            }
        }
    }

    println!("\n=== RESULTS ===");
    println!("Total 2DA files scanned: {total_files}");
    println!("Files with tab characters: {}", files_with_tabs.len());
    println!("Files without tabs: {files_without_tabs}");

    if !files_with_tabs.is_empty() {
        println!("\n=== Files containing tabs ===");
        for (zip, file) in &files_with_tabs {
            println!("  {zip} -> {file}");
        }
    } else {
        println!("\nNo 2DA files use tab characters - all use space-separated format");
    }

    assert!(
        total_files > 0,
        "Should have scanned at least some 2DA files"
    );
}

#[tokio::test]
async fn test_2da_tab_separated_parsing_verification() {
    use std::path::PathBuf;

    let ctx = create_test_context().await;

    let nwn2_data_dir =
        PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/NWN2 Enhanced Edition/Data");

    if !nwn2_data_dir.exists() {
        println!("WARNING: NWN2 install not found, skipping tab parsing verification");
        return;
    }

    let zip_path = nwn2_data_dir.join("2da.zip");
    if !zip_path.exists() {
        println!("WARNING: 2da.zip not found, skipping test");
        return;
    }

    let file = std::fs::File::open(&zip_path).expect("Failed to open 2da.zip");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read zip");

    let spells_entry = archive.by_name("2da/spells.2da");
    if spells_entry.is_err() {
        println!("WARNING: spells.2da not found in zip");
        return;
    }

    let mut entry = spells_entry.unwrap();
    let mut contents = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut contents).expect("Failed to read spells.2da");

    let text = String::from_utf8_lossy(&contents);

    println!("\n=== Verifying tab-separated parsing for spells.2da ===");
    println!("File size: {} bytes", contents.len());
    println!("Contains tabs: {}", text.contains('\t'));

    let lines_with_tabs = text.lines().filter(|line| line.contains('\t')).count();

    println!(
        "Lines with tabs: {} / {}",
        lines_with_tabs,
        text.lines().count()
    );

    let table = ctx
        .game_data
        .get_table("spells")
        .expect("spells.2da not found");
    println!("Parsed rows: {}", table.row_count());
    println!("Parsed columns: {}", table.parser.column_count());

    let cols = table.column_names();
    println!("Column count: {}", cols.len());
    println!("First 10 columns: {:?}", &cols[..10.min(cols.len())]);

    let row0 = table.get_row(0).expect("Failed to get row 0");
    println!("Row 0 field count: {}", row0.len());
    println!(
        "Row 0 Label: {:?}",
        row0.get("label").or_else(|| row0.get("Label"))
    );

    assert!(table.row_count() > 0, "Should have parsed rows");
    assert!(table.parser.column_count() > 10, "Should have many columns");
    assert!(text.contains('\t'), "spells.2da should use tabs");
    println!("\nTab-separated parsing verified successfully!");
}

// =============================================================================
// MIXED TAB/SPACE LINES (vanilla data, engine splits on whitespace runs)
// =============================================================================

#[tokio::test]
async fn test_2da_mixed_tab_space_tables() {
    let ctx = create_test_context().await;
    let rm = ctx.resource_manager.read().await;

    let drow = rm.get_2da("race_feat_drow").expect("race_feat_drow.2da");
    for row in 0..drow.row_count() {
        let index = drow.get_cell_by_name(row, "FeatIndex").unwrap();
        assert!(
            index.is_some_and(|v| v.parse::<i32>().is_ok()),
            "race_feat_drow row {row} FeatIndex {index:?}"
        );
    }
    assert_eq!(
        drow.get_cell_by_name(0, "FeatLabel").unwrap(),
        Some("immunitysleep")
    );

    let spell_schools = rm.get_2da("iprp_spellshl").expect("iprp_spellshl.2da");
    assert_eq!(
        spell_schools.column_names(),
        ["Name", "Label", "Cost", "Letter"]
    );
    assert_eq!(spell_schools.get_cell(0, 1).unwrap(), Some("Abjuration"));

    // Verified in-game: "Male, Seasoned Fighter" is missing from the voice list.
    let soundset = rm.get_2da("soundset").expect("soundset.2da");
    assert_eq!(
        soundset.get_cell_by_name(467, "RESREF").unwrap(),
        Some("Fighter_PCV")
    );
}
