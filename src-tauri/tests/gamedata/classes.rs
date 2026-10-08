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
async fn test_classes_table_loaded() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da should be loaded");

    assert!(table.row_count() > 0, "Should have class rows");
    println!("Classes table: {} rows", table.row_count());
}

#[tokio::test]
async fn test_classes_columns() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da missing");
    let cols = table.column_names();

    let required_cols = ["Label", "HitDie", "AttackBonusTable", "SkillPointBase"];
    for col in required_cols {
        assert!(
            cols.iter().any(|c| c.eq_ignore_ascii_case(col)),
            "Missing required column: {col}"
        );
    }
}

#[tokio::test]
async fn test_dump_base_classes() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da missing");

    println!("\n=== Base Classes ===");
    println!(
        "{:<4} {:<20} {:<6} {:<15}",
        "ID", "Label", "HitDie", "Description"
    );
    println!("{}", "-".repeat(60));

    for row_idx in 0..table.row_count().min(12) {
        let label = cell_value(table, row_idx, "Label").unwrap_or_default();
        let hit_die = cell_value(table, row_idx, "HitDie").unwrap_or_default();
        let desc_ref = cell_value(table, row_idx, "Description").unwrap_or_default();

        println!("{row_idx:<4} {label:<20} {hit_die:<6} {desc_ref:>15}");
    }
}

#[tokio::test]
async fn test_class_hit_dice() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da missing");

    let fighter_hd = cell_int(table, 4, "HitDie");
    assert_eq!(fighter_hd, Some(10), "Fighter should have d10 hit die");

    let wizard_hd = cell_int(table, 10, "HitDie");
    assert_eq!(wizard_hd, Some(4), "Wizard should have d4 hit die");
}

#[tokio::test]
async fn test_class_bab_tables() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da missing");

    let fighter_bab = cell_value(table, 4, "AttackBonusTable").unwrap_or_default();
    assert!(
        fighter_bab.to_lowercase().contains("high") || fighter_bab.contains("CLS_ATK_1"),
        "Fighter should have high BAB progression"
    );

    let wizard_bab = cell_value(table, 10, "AttackBonusTable").unwrap_or_default();
    assert!(
        wizard_bab.to_lowercase().contains("low") || wizard_bab.contains("CLS_ATK_3"),
        "Wizard should have low BAB progression"
    );
}

#[tokio::test]
async fn test_class_spellcaster_type() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da missing");

    let cleric_arcane = cell_int(table, 2, "ArcSpellLvlMod").unwrap_or(0);
    let cleric_divine = cell_int(table, 2, "DivSpellLvlMod").unwrap_or(0);

    println!("Cleric: Arcane={cleric_arcane}, Divine={cleric_divine}");

    let wizard_arcane = cell_int(table, 10, "ArcSpellLvlMod").unwrap_or(0);

    println!("Wizard: Arcane={wizard_arcane}");
}

#[tokio::test]
async fn test_prestige_classes_exist() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("classes")
        .expect("classes.2da missing");

    let mut prestige_count = 0;
    for row_idx in 0..table.row_count() {
        let player_class = cell_int(table, row_idx, "PlayerClass").unwrap_or(0);

        if player_class == 1 {
            prestige_count += 1;
        }
    }

    println!("Found {prestige_count} player-selectable classes");
    assert!(
        prestige_count > 10,
        "Should have at least 10 player classes"
    );
}
