use super::super::common::create_test_context;

fn cell_value(
    table: &app_lib::loaders::types::LoadedTable,
    row: usize,
    col: &str,
) -> Option<String> {
    table.get_cell(row, col).ok().flatten()
}

fn cell_int(table: &app_lib::loaders::types::LoadedTable, row: usize, col: &str) -> Option<i32> {
    cell_value(table, row, col).and_then(|s| s.parse().ok())
}

#[tokio::test]
async fn test_races_table_loaded() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da should be loaded");

    assert!(table.row_count() > 0, "Should have race rows");
    println!("Racialtypes table: {} rows", table.row_count());
}

#[tokio::test]
async fn test_races_columns() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da missing");
    let cols = table.column_names();

    println!("\n=== Racialtypes Columns ===");
    for col in &cols {
        println!("  {col}");
    }

    let required_cols = [
        "Label",
        "StrAdjust",
        "DexAdjust",
        "ConAdjust",
        "IntAdjust",
        "WisAdjust",
        "ChaAdjust",
        "Favored",
    ];
    for col in required_cols {
        assert!(
            cols.iter().any(|c| c.eq_ignore_ascii_case(col)),
            "Missing required column: {col}"
        );
    }
}

#[tokio::test]
async fn test_dump_playable_races() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da missing");

    println!("\n=== Playable Races ===");
    println!(
        "{:<4} {:<15} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4}",
        "ID", "Label", "STR", "DEX", "CON", "INT", "WIS", "CHA"
    );
    println!("{}", "-".repeat(60));

    for row_idx in 0..table.row_count().min(10) {
        let label = cell_value(table, row_idx, "Label").unwrap_or_default();
        let str_adj = cell_value(table, row_idx, "StrAdjust").unwrap_or_default();
        let dex_adj = cell_value(table, row_idx, "DexAdjust").unwrap_or_default();
        let con_adj = cell_value(table, row_idx, "ConAdjust").unwrap_or_default();
        let int_adj = cell_value(table, row_idx, "IntAdjust").unwrap_or_default();
        let wis_adj = cell_value(table, row_idx, "WisAdjust").unwrap_or_default();
        let cha_adj = cell_value(table, row_idx, "ChaAdjust").unwrap_or_default();

        println!(
            "{row_idx:<4} {label:<15} {str_adj:>4} {dex_adj:>4} {con_adj:>4} {int_adj:>4} {wis_adj:>4} {cha_adj:>4}"
        );
    }
}

#[tokio::test]
async fn test_human_no_adjustments() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da missing");

    let human_row = (0..table.row_count())
        .find(|&row| {
            cell_value(table, row, "Label")
                .map(|l| l.eq_ignore_ascii_case("Human"))
                .unwrap_or(false)
        })
        .expect("Human race not found");

    let human_str = cell_int(table, human_row, "StrAdjust").unwrap_or(0);
    let human_dex = cell_int(table, human_row, "DexAdjust").unwrap_or(0);

    assert_eq!(human_str, 0, "Human should have no STR adjustment");
    assert_eq!(human_dex, 0, "Human should have no DEX adjustment");
}

#[tokio::test]
async fn test_dwarf_ability_adjustments() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da missing");

    let dwarf_row = (0..table.row_count())
        .find(|&row| {
            cell_value(table, row, "Label")
                .map(|l| l.eq_ignore_ascii_case("Dwarf"))
                .unwrap_or(false)
        })
        .unwrap_or(1);

    let con_adj = cell_int(table, dwarf_row, "ConAdjust").unwrap_or(0);
    let cha_adj = cell_int(table, dwarf_row, "ChaAdjust").unwrap_or(0);

    assert_eq!(con_adj, 2, "Dwarf should have +2 CON");
    assert_eq!(cha_adj, -2, "Dwarf should have -2 CHA");
}

#[tokio::test]
async fn test_race_favored_class() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da missing");

    println!("\n=== Favored Classes ===");
    for row_idx in 0..table.row_count().min(8) {
        let label = cell_value(table, row_idx, "Label").unwrap_or_default();
        let favored = cell_value(table, row_idx, "Favored").unwrap_or_default();

        println!("{label}: favored class ID = {favored}");
    }
}

#[tokio::test]
async fn test_playable_races_count() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("racialtypes")
        .expect("racialtypes.2da missing");

    let cols = table.column_names();
    let has_player_race = cols.iter().any(|c| c.eq_ignore_ascii_case("PlayerRace"));

    if has_player_race {
        let playable_count = (0..table.row_count())
            .filter(|&row| cell_int(table, row, "PlayerRace").unwrap_or(0) == 1)
            .count();

        println!("Playable races: {playable_count}");
        assert!(playable_count >= 7, "Should have at least 7 playable races");
    }
}

// =============== SUBRACE TESTS ===============

#[tokio::test]
async fn test_subraces_table_loaded() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("racialsubtypes");

    if let Some(table) = table {
        println!("\n=== Racialsubtypes Table ===");
        println!("Rows: {}", table.row_count());

        let cols = table.column_names();
        println!("Columns: {cols:?}");
    } else {
        println!("racialsubtypes.2da not loaded as priority table, trying subrace.2da");

        if let Some(table) = ctx.game_data.get_table("subrace") {
            println!("Found subrace.2da with {} rows", table.row_count());
        } else {
            println!("No subrace table found in game data");
        }
    }
}

#[tokio::test]
async fn test_dump_subraces() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("racialsubtypes");

    if let Some(table) = table {
        println!("\n=== Subraces ===");
        println!(
            "{:<4} {:<20} {:<15} {:>4} {:>4} {:>4}",
            "ID", "Label", "BaseRace", "STR", "DEX", "CON"
        );
        println!("{}", "-".repeat(65));

        for row_idx in 0..table.row_count().min(20) {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();
            let base_race = cell_value(table, row_idx, "BaseRace")
                .or_else(|| cell_value(table, row_idx, "Race"))
                .unwrap_or_default();
            let str_adj = cell_value(table, row_idx, "StrAdjust").unwrap_or_default();
            let dex_adj = cell_value(table, row_idx, "DexAdjust").unwrap_or_default();
            let con_adj = cell_value(table, row_idx, "ConAdjust").unwrap_or_default();

            println!(
                "{row_idx:<4} {label:<20} {base_race:<15} {str_adj:>4} {dex_adj:>4} {con_adj:>4}"
            );
        }
    } else {
        println!("racialsubtypes.2da not available");
    }
}

#[tokio::test]
async fn test_subrace_columns() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("racialsubtypes") {
        let cols = table.column_names();

        println!("\n=== Racialsubtypes Columns ===");
        for col in &cols {
            println!("  {col}");
        }

        assert!(
            cols.iter().any(|c| c.eq_ignore_ascii_case("Label")),
            "Should have Label column"
        );
    }
}

#[tokio::test]
async fn test_playable_subraces() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("racialsubtypes") {
        let cols = table.column_names();

        let player_col = cols
            .iter()
            .find(|c| c.eq_ignore_ascii_case("PlayerRace") || c.eq_ignore_ascii_case("Playable"));

        if let Some(col) = player_col {
            let playable_count = (0..table.row_count())
                .filter(|&row| cell_int(table, row, col).unwrap_or(0) == 1)
                .count();

            println!("Playable subraces: {playable_count}");
        }

        let mut by_base_race: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for row_idx in 0..table.row_count() {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();
            let base = cell_value(table, row_idx, "BaseRace")
                .or_else(|| cell_value(table, row_idx, "Race"))
                .unwrap_or_else(|| "Unknown".to_string());

            by_base_race.entry(base).or_default().push(label);
        }

        println!("\n=== Subraces by Base Race ===");
        for (base, subs) in &by_base_race {
            println!(
                "  {} ({}): {:?}",
                base,
                subs.len(),
                subs.iter().take(5).collect::<Vec<_>>()
            );
        }
    }
}

#[tokio::test]
async fn test_drow_subrace() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("racialsubtypes") {
        let drow_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "Label")
                .map(|l| l.to_lowercase().contains("drow"))
                .unwrap_or(false)
        });

        if let Some(row) = drow_row {
            println!("\n=== Drow Subrace ===");

            for col in table.column_names() {
                let val = cell_value(table, row, col).unwrap_or_default();
                if !val.is_empty() && val != "****" {
                    println!("  {col}: {val}");
                }
            }
        } else {
            println!("Drow subrace not found");
        }
    }
}

#[tokio::test]
async fn test_aasimar_tiefling_subraces() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("racialsubtypes") {
        println!("\n=== Planetouched Subraces ===");

        for race_name in ["aasimar", "tiefling", "genasi"] {
            let row = (0..table.row_count()).find(|&r| {
                cell_value(table, r, "Label")
                    .map(|l| l.to_lowercase().contains(race_name))
                    .unwrap_or(false)
            });

            if let Some(r) = row {
                let label = cell_value(table, r, "Label").unwrap_or_default();
                let str_adj = cell_int(table, r, "StrAdjust").unwrap_or(0);
                let dex_adj = cell_int(table, r, "DexAdjust").unwrap_or(0);
                let con_adj = cell_int(table, r, "ConAdjust").unwrap_or(0);
                let int_adj = cell_int(table, r, "IntAdjust").unwrap_or(0);
                let wis_adj = cell_int(table, r, "WisAdjust").unwrap_or(0);
                let cha_adj = cell_int(table, r, "ChaAdjust").unwrap_or(0);

                println!(
                    "  {label}: STR={str_adj:+}, DEX={dex_adj:+}, CON={con_adj:+}, INT={int_adj:+}, WIS={wis_adj:+}, CHA={cha_adj:+}"
                );
            }
        }
    }
}
