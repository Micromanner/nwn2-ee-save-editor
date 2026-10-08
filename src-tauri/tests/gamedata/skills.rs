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
async fn test_skills_table_loaded() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("skills")
        .expect("skills.2da should be loaded");

    assert!(table.row_count() > 0, "Should have skill rows");
    println!("Skills table: {} rows", table.row_count());
}

#[tokio::test]
async fn test_skills_columns() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("skills")
        .expect("skills.2da missing");
    let cols = table.column_names();

    let required_cols = ["Label", "KeyAbility", "ArmorCheckPenalty"];
    for col in required_cols {
        assert!(
            cols.iter().any(|c| c.eq_ignore_ascii_case(col)),
            "Missing required column: {col}"
        );
    }
}

#[tokio::test]
async fn test_dump_all_skills() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("skills")
        .expect("skills.2da missing");

    println!("\n=== All Skills ===");
    println!("{:<4} {:<20} {:<8} {:<6}", "ID", "Label", "Ability", "ACP");
    println!("{}", "-".repeat(45));

    for row_idx in 0..table.row_count() {
        let label = cell_value(table, row_idx, "Label").unwrap_or_default();
        let ability = cell_value(table, row_idx, "KeyAbility").unwrap_or_default();
        let acp = cell_value(table, row_idx, "ArmorCheckPenalty").unwrap_or_default();

        println!("{row_idx:<4} {label:<20} {ability:<8} {acp:<6}");
    }
}

#[tokio::test]
async fn test_skill_key_abilities() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("skills")
        .expect("skills.2da missing");

    let hide_row = (0..table.row_count()).find(|&row| {
        cell_value(table, row, "Label")
            .map(|l| l.eq_ignore_ascii_case("Hide"))
            .unwrap_or(false)
    });

    if let Some(row) = hide_row {
        let ability = cell_value(table, row, "KeyAbility").unwrap_or_default();
        assert!(
            ability == "DEX" || ability == "1",
            "Hide should use DEX (got: {ability})"
        );
    }

    let concentration_row = (0..table.row_count()).find(|&row| {
        cell_value(table, row, "Label")
            .map(|l| l.eq_ignore_ascii_case("Concentration"))
            .unwrap_or(false)
    });

    if let Some(row) = concentration_row {
        let ability = cell_value(table, row, "KeyAbility").unwrap_or_default();
        assert!(
            ability == "CON" || ability == "2",
            "Concentration should use CON (got: {ability})"
        );
    }
}

#[tokio::test]
async fn test_armor_check_penalty_skills() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("skills")
        .expect("skills.2da missing");

    let mut acp_skills = Vec::new();
    let mut no_acp_skills = Vec::new();

    for row_idx in 0..table.row_count() {
        let label = cell_value(table, row_idx, "Label").unwrap_or_default();
        let acp = cell_int(table, row_idx, "ArmorCheckPenalty").unwrap_or(0);

        if acp == 1 {
            acp_skills.push(label);
        } else {
            no_acp_skills.push(label);
        }
    }

    println!("Skills with Armor Check Penalty: {acp_skills:?}");
    println!("Skills without ACP: {} total", no_acp_skills.len());

    assert!(
        acp_skills
            .iter()
            .any(|s| s.to_lowercase().contains("tumble")),
        "Tumble should have armor check penalty"
    );
}

#[tokio::test]
async fn test_class_skills_table() {
    let ctx = create_test_context().await;

    let cls_skill_fight = ctx.game_data.get_table("cls_skill_fight");

    if let Some(table) = cls_skill_fight {
        println!("\n=== Fighter Class Skills ===");
        println!("Rows: {}", table.row_count());

        let cols = table.column_names();
        println!("Columns: {cols:?}");

        let mut class_skills = Vec::new();
        for row_idx in 0..table.row_count() {
            let is_class = cell_int(table, row_idx, "ClassSkill").unwrap_or(0);

            if is_class == 1 {
                class_skills.push(row_idx);
            }
        }

        println!("Fighter class skill IDs: {class_skills:?}");
    } else {
        println!("cls_skill_fight.2da not loaded as priority table");
    }
}

#[tokio::test]
async fn test_skill_untrained_usage() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("skills")
        .expect("skills.2da missing");

    let cols = table.column_names();
    let has_untrained = cols.iter().any(|c| c.eq_ignore_ascii_case("Untrained"));

    if has_untrained {
        let mut trainable_only = Vec::new();

        for row_idx in 0..table.row_count() {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();
            let untrained = cell_int(table, row_idx, "Untrained").unwrap_or(1);

            if untrained == 0 {
                trainable_only.push(label);
            }
        }

        println!("Skills requiring training: {trainable_only:?}");
    }
}
