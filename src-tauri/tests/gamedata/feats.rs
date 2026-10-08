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
async fn test_feat_table_loaded() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("feat")
        .expect("feat.2da should be loaded");

    assert!(table.row_count() > 0, "Should have feat rows");
    println!("Feat table: {} rows", table.row_count());
}

#[tokio::test]
async fn test_feat_columns() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");
    let cols = table.column_names();

    let required_cols = [
        "LABEL",
        "FEAT",
        "PREREQFEAT1",
        "PREREQFEAT2",
        "ALLCLASSESCANUSE",
    ];
    for col in required_cols {
        assert!(
            cols.iter().any(|c| c.eq_ignore_ascii_case(col)),
            "Missing required column: {col}"
        );
    }
}

#[tokio::test]
async fn test_dump_first_feats() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");

    println!("\n=== First 20 Feats ===");
    println!("{:<4} {:<30} {:<10}", "ID", "Label", "AllClasses");
    println!("{}", "-".repeat(50));

    for row_idx in 0..table.row_count().min(20) {
        let label = cell_value(table, row_idx, "LABEL").unwrap_or_default();
        let all_classes = cell_value(table, row_idx, "ALLCLASSESCANUSE").unwrap_or_default();

        println!("{row_idx:<4} {label:<30} {all_classes:<10}");
    }
}

#[tokio::test]
async fn test_feat_prerequisite_chain() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");

    let power_attack_row = (0..table.row_count()).find(|&row| {
        cell_value(table, row, "LABEL")
            .map(|l| l.eq_ignore_ascii_case("Power_Attack"))
            .unwrap_or(false)
    });

    if let Some(pa_row) = power_attack_row {
        println!("Power Attack found at row {pa_row}");

        let prereq1 = cell_value(table, pa_row, "PREREQFEAT1");
        let prereq2 = cell_value(table, pa_row, "PREREQFEAT2");
        let min_str = cell_value(table, pa_row, "MINSTR");

        println!("  Prereq1: {prereq1:?}, Prereq2: {prereq2:?}, MinStr: {min_str:?}");
    }

    let cleave_row = (0..table.row_count()).find(|&row| {
        cell_value(table, row, "LABEL")
            .map(|l| l.eq_ignore_ascii_case("Cleave"))
            .unwrap_or(false)
    });

    if let Some(c_row) = cleave_row {
        let prereq_id = cell_int(table, c_row, "PREREQFEAT1");

        if let Some(prereq) = prereq_id {
            println!("Cleave requires feat ID {prereq}");

            if let Some(pa_row) = power_attack_row {
                assert_eq!(
                    prereq as usize, pa_row,
                    "Cleave should require Power Attack"
                );
            }
        }
    }
}

#[tokio::test]
async fn test_metamagic_feats() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");

    let mut metamagic_feats = Vec::new();

    for row_idx in 0..table.row_count() {
        let label = cell_value(table, row_idx, "LABEL").unwrap_or_default();

        if label.to_lowercase().contains("empower")
            || label.to_lowercase().contains("maximize")
            || label.to_lowercase().contains("quicken")
            || label.to_lowercase().contains("still")
            || label.to_lowercase().contains("silent")
        {
            metamagic_feats.push((row_idx, label));
        }
    }

    println!("\n=== Metamagic Feats ===");
    for (id, label) in &metamagic_feats {
        println!("  {id} - {label}");
    }

    assert!(!metamagic_feats.is_empty(), "Should find metamagic feats");
}

#[tokio::test]
async fn test_class_feat_table() {
    let ctx = create_test_context().await;

    let cls_feat_fighter = ctx.game_data.get_table("cls_feat_fight");

    if let Some(table) = cls_feat_fighter {
        println!("\n=== Fighter Class Feats ===");
        println!("Rows: {}", table.row_count());

        let cols = table.column_names();
        println!("Columns: {cols:?}");

        let mut auto_feats = Vec::new();
        for row_idx in 0..table.row_count().min(50) {
            let list_type = cell_int(table, row_idx, "List").unwrap_or(-1);

            if list_type == 0 {
                let feat_id = cell_value(table, row_idx, "FeatIndex").unwrap_or_default();
                let granted = cell_value(table, row_idx, "GrantedOnLevel").unwrap_or_default();
                auto_feats.push((feat_id, granted));
            }
        }

        println!("Auto-granted feats: {auto_feats:?}");
    } else {
        println!("cls_feat_fight.2da not loaded as priority table");
    }
}

#[tokio::test]
async fn test_feat_strref_lookup() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");

    let name_strref = cell_int(table, 0, "FEAT");

    if let Some(strref) = name_strref {
        let name = ctx.game_data.get_string(strref);
        println!("Feat 0 name (strref {strref}): {name:?}");
    }

    for row_idx in [1, 2, 3, 4, 5] {
        if row_idx < table.row_count() {
            let label = cell_value(table, row_idx, "LABEL");
            let strref = cell_int(table, row_idx, "FEAT");

            if let Some(sr) = strref {
                let name = ctx.game_data.get_string(sr);
                println!(
                    "Feat {}: {} -> {:?}",
                    row_idx,
                    label.as_deref().unwrap_or("?"),
                    name
                );
            }
        }
    }
}

// =============== EPIC FEAT TESTS ===============

#[tokio::test]
async fn test_epic_feats() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");
    let cols = table.column_names();

    let has_prereq_epic = cols.iter().any(|c| c.eq_ignore_ascii_case("PreReqEpic"));
    let has_feat_category = cols.iter().any(|c| c.eq_ignore_ascii_case("FeatCategory"));
    let has_min_level = cols.iter().any(|c| c.eq_ignore_ascii_case("MINLEVEL"));

    println!("\n=== Epic Feat Detection ===");
    println!("Has PreReqEpic column: {has_prereq_epic}");
    println!("Has FeatCategory column: {has_feat_category}");
    println!("Has MINLEVEL column: {has_min_level}");

    let mut epic_feats = Vec::new();

    for row_idx in 0..table.row_count() {
        let label = cell_value(table, row_idx, "LABEL").unwrap_or_default();

        let is_epic_by_prereq = if has_prereq_epic {
            cell_int(table, row_idx, "PreReqEpic").unwrap_or(0) == 1
        } else {
            false
        };

        let is_epic_by_name = label.to_uppercase().contains("EPIC");

        let min_level = if has_min_level {
            cell_int(table, row_idx, "MINLEVEL").unwrap_or(0)
        } else {
            0
        };
        let is_epic_by_level = min_level >= 21;

        if is_epic_by_prereq || is_epic_by_name || is_epic_by_level {
            epic_feats.push((row_idx, label, min_level, is_epic_by_prereq));
        }
    }

    println!("\n=== Epic Feats (first 30) ===");
    println!(
        "{:<5} {:<45} {:>8} {:>10}",
        "ID", "Label", "MinLevel", "EpicFlag"
    );
    println!("{}", "-".repeat(75));

    for (id, label, min_level, epic_flag) in epic_feats.iter().take(30) {
        println!("{id:<5} {label:<45} {min_level:>8} {epic_flag:>10}");
    }

    println!("\nTotal epic feats found: {}", epic_feats.len());
    assert!(epic_feats.len() > 50, "Should have many epic feats");
}

#[tokio::test]
async fn test_epic_feat_categories() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");

    let mut epic_by_category: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for row_idx in 0..table.row_count() {
        let label = cell_value(table, row_idx, "LABEL").unwrap_or_default();

        if !label.to_uppercase().contains("EPIC") {
            continue;
        }

        let category = if label.contains("PROWESS") {
            "Prowess"
        } else if label.contains("FOCUS") {
            "Skill Focus"
        } else if label.contains("SPELL") {
            "Spellcasting"
        } else if label.contains("WEAPON") || label.contains("ARMOR") {
            "Combat"
        } else if label.contains("FORTITUDE") || label.contains("REFLEX") || label.contains("WILL")
        {
            "Saves"
        } else if label.contains("TOUGHNESS") || label.contains("RESIST") {
            "Defense"
        } else {
            "Other"
        };

        epic_by_category
            .entry(category.to_string())
            .or_default()
            .push(label);
    }

    println!("\n=== Epic Feats by Category ===");
    for (cat, feats) in &epic_by_category {
        println!("  {} ({} feats)", cat, feats.len());
        for feat in feats.iter().take(3) {
            println!("    - {feat}");
        }
        if feats.len() > 3 {
            println!("    ... and {} more", feats.len() - 3);
        }
    }
}

#[tokio::test]
async fn test_great_epic_feats() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");

    println!("\n=== Great [Ability] Feats ===");

    for ability in ["STR", "DEX", "CON", "INT", "WIS", "CHA"] {
        let pattern = format!("GREAT_{ability}");

        let matches: Vec<_> = (0..table.row_count())
            .filter_map(|row| {
                let label = cell_value(table, row, "LABEL")?;
                if label.to_uppercase().contains(&pattern) {
                    Some((row, label))
                } else {
                    None
                }
            })
            .collect();

        println!(
            "  Great {}: {} feats (IDs: {:?})",
            ability,
            matches.len(),
            matches.iter().map(|(id, _)| *id).collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn test_epic_spell_feats() {
    let ctx = create_test_context().await;

    let table = ctx.game_data.get_table("feat").expect("feat.2da missing");

    let epic_spell_feats: Vec<_> = (0..table.row_count())
        .filter_map(|row| {
            let label = cell_value(table, row, "LABEL")?;
            let upper = label.to_uppercase();

            if upper.contains("EPIC")
                && (upper.contains("SPELL") || upper.contains("MAGE") || upper.contains("WARDING"))
            {
                let prereq1 = cell_int(table, row, "PREREQFEAT1").unwrap_or(-1);
                let min_level = cell_int(table, row, "MINLEVEL").unwrap_or(0);
                Some((row, label, prereq1, min_level))
            } else {
                None
            }
        })
        .collect();

    println!("\n=== Epic Spellcasting Feats ===");
    for (id, label, prereq, min_lvl) in &epic_spell_feats {
        println!("  {label} (ID {id}): prereq={prereq}, minlvl={min_lvl}");
    }
}
