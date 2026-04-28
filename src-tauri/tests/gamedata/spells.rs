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
async fn test_spells_table_loaded() {
    let ctx = create_test_context().await;

    let table = ctx
        .loader
        .get_table("spells")
        .expect("spells.2da should be loaded");

    assert!(table.row_count() > 0, "Should have spell rows");
    println!("Spells table: {} rows", table.row_count());
}

#[tokio::test]
async fn test_spells_columns() {
    let ctx = create_test_context().await;

    let table = ctx.loader.get_table("spells").expect("spells.2da missing");
    let cols = table.column_names();

    let required_cols = ["Label", "School", "Wiz_Sorc", "Cleric", "Bard", "Druid"];
    for col in required_cols {
        assert!(
            cols.iter().any(|c| c.eq_ignore_ascii_case(col)),
            "Missing required column: {col}"
        );
    }
}

#[tokio::test]
async fn test_dump_cantrips() {
    let ctx = create_test_context().await;

    let table = ctx.loader.get_table("spells").expect("spells.2da missing");

    println!("\n=== Wizard Cantrips (Level 0) ===");
    println!(
        "{:<4} {:<25} {:<8} {:<8}",
        "ID", "Label", "School", "WizLvl"
    );
    println!("{}", "-".repeat(50));

    let mut cantrip_count = 0;
    for row_idx in 0..table.row_count() {
        let wiz_level = cell_int(table, row_idx, "Wiz_Sorc");

        if wiz_level == Some(0) {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();
            let school = cell_value(table, row_idx, "School").unwrap_or_default();

            println!("{:<4} {:<25} {:<8} {:<8}", row_idx, label, school, 0);
            cantrip_count += 1;
        }
    }

    println!("\nTotal cantrips: {cantrip_count}");
    assert!(cantrip_count > 0, "Should have wizard cantrips");
}

#[tokio::test]
async fn test_dump_first_level_spells() {
    let ctx = create_test_context().await;

    let table = ctx.loader.get_table("spells").expect("spells.2da missing");

    println!("\n=== First Level Wizard Spells ===");

    let mut level1_count = 0;
    for row_idx in 0..table.row_count() {
        let wiz_level = cell_int(table, row_idx, "Wiz_Sorc");

        if wiz_level == Some(1) {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();

            if level1_count < 20 {
                println!("  {row_idx} - {label}");
            }
            level1_count += 1;
        }
    }

    println!("Total level 1 wizard spells: {level1_count}");
}

#[tokio::test]
async fn test_spell_schools() {
    let ctx = create_test_context().await;

    let table = ctx.loader.get_table("spells").expect("spells.2da missing");

    let mut schools = std::collections::HashMap::new();

    for row_idx in 0..table.row_count() {
        let school = cell_value(table, row_idx, "School").unwrap_or_default();

        if !school.is_empty() && school != "****" {
            *schools.entry(school).or_insert(0) += 1;
        }
    }

    println!("\n=== Spell Schools ===");
    let mut school_vec: Vec<_> = schools.into_iter().collect();
    school_vec.sort_by_key(|b| std::cmp::Reverse(b.1));

    for (school, count) in school_vec {
        println!("  {school}: {count} spells");
    }
}

#[tokio::test]
async fn test_divine_vs_arcane_spells() {
    let ctx = create_test_context().await;

    let table = ctx.loader.get_table("spells").expect("spells.2da missing");

    let mut wizard_only = 0;
    let mut cleric_only = 0;
    let mut both = 0;

    for row_idx in 0..table.row_count() {
        let wiz = cell_int(table, row_idx, "Wiz_Sorc");
        let cleric = cell_int(table, row_idx, "Cleric");

        match (wiz, cleric) {
            (Some(_), Some(_)) => both += 1,
            (Some(_), None) => wizard_only += 1,
            (None, Some(_)) => cleric_only += 1,
            _ => {}
        }
    }

    println!("\n=== Spell Distribution ===");
    println!("  Wizard-only: {wizard_only}");
    println!("  Cleric-only: {cleric_only}");
    println!("  Both classes: {both}");
}

#[tokio::test]
async fn test_bard_spell_list() {
    let ctx = create_test_context().await;

    let table = ctx.loader.get_table("spells").expect("spells.2da missing");

    let mut bard_spells_by_level: std::collections::HashMap<i32, Vec<String>> =
        std::collections::HashMap::new();

    for row_idx in 0..table.row_count() {
        let bard_level = cell_int(table, row_idx, "Bard");

        if let Some(level) = bard_level {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();

            bard_spells_by_level.entry(level).or_default().push(label);
        }
    }

    println!("\n=== Bard Spells by Level ===");
    for level in 0..=6 {
        if let Some(spells) = bard_spells_by_level.get(&level) {
            println!("  Level {}: {} spells", level, spells.len());
        }
    }
}

#[tokio::test]
async fn test_spell_desc_resolution() {
    let ctx = create_test_context().await;
    let table = ctx.loader.get_table("spells").expect("spells.2da missing");
    let cols = table.column_names();

    assert!(
        cols.iter().any(|c| c.eq_ignore_ascii_case("SpellDesc")),
        "SpellDesc column should exist in spells.2da"
    );

    // Verify core spells have resolvable descriptions
    let search_labels = ["Shield", "Web", "Magic_Missile", "Acid_Fog"];
    for label in search_labels {
        for row_idx in 0..table.row_count() {
            if cell_value(table, row_idx, "Label").as_deref() != Some(label) {
                continue;
            }
            let desc_strref =
                cell_value(table, row_idx, "SpellDesc").and_then(|s| s.parse::<i32>().ok());
            let resolved = desc_strref.and_then(|strref| ctx.loader.get_string(strref));
            assert!(
                resolved.as_ref().is_some_and(|s| !s.is_empty()),
                "Spell {label} should have a resolved description (strref={desc_strref:?})"
            );
            break;
        }
    }

    // Count resolution stats
    let mut resolved_count = 0;
    let mut total_with_strref = 0;
    for row_idx in 0..table.row_count() {
        let desc_val = cell_value(table, row_idx, "SpellDesc");
        if let Some(ref s) = desc_val
            && let Ok(strref) = s.parse::<i32>()
        {
            total_with_strref += 1;
            if ctx
                .loader
                .get_string(strref)
                .as_ref()
                .is_some_and(|r| !r.is_empty())
            {
                resolved_count += 1;
            }
        }
    }

    let resolve_rate = f64::from(resolved_count) / f64::from(total_with_strref) * 100.0;
    println!("SpellDesc resolution: {resolved_count}/{total_with_strref} ({resolve_rate:.1}%)");
    assert!(
        resolve_rate > 95.0,
        "At least 95% of SpellDesc strrefs should resolve"
    );
}

#[tokio::test]
async fn test_metamagic_flags() {
    let ctx = create_test_context().await;

    let table = ctx.loader.get_table("spells").expect("spells.2da missing");

    let cols = table.column_names();
    let has_metamagic = cols.iter().any(|c| c.eq_ignore_ascii_case("MetaMagic"));

    if has_metamagic {
        let mut no_meta = 0;
        let mut has_meta = 0;

        for row_idx in 0..table.row_count() {
            let meta = cell_int(table, row_idx, "MetaMagic").unwrap_or(0);

            if meta == 0 {
                no_meta += 1;
            } else {
                has_meta += 1;
            }
        }

        println!("Spells with metamagic allowed: {has_meta}");
        println!("Spells without metamagic: {no_meta}");
    }
}
