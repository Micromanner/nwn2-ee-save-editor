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
async fn test_baseitems_table_loaded() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da should be loaded");

    assert!(table.row_count() > 0, "Should have item rows");
    println!("Baseitems table: {} rows", table.row_count());
}

#[tokio::test]
async fn test_baseitems_columns() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da missing");
    let cols = table.column_names();

    let required_cols = ["Label", "EquipableSlots", "WeaponType"];
    for col in required_cols {
        assert!(
            cols.iter().any(|c| c.eq_ignore_ascii_case(col)),
            "Missing required column: {col}"
        );
    }
}

#[tokio::test]
async fn test_dump_weapon_types() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da missing");

    println!("\n=== Weapon Types ===");
    println!(
        "{:<4} {:<25} {:<10} {:<10}",
        "ID", "Label", "WeapType", "NumDice"
    );
    println!("{}", "-".repeat(55));

    let mut weapon_count = 0;
    for row_idx in 0..table.row_count() {
        let weapon_type = cell_int(table, row_idx, "WeaponType").unwrap_or(0);

        if weapon_type > 0 {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();
            let num_dice = cell_value(table, row_idx, "NumDice").unwrap_or_default();

            if weapon_count < 30 {
                println!("{row_idx:<4} {label:<25} {weapon_type:<10} {num_dice:<10}");
            }
            weapon_count += 1;
        }
    }

    println!("\nTotal weapon types: {weapon_count}");
}

#[tokio::test]
async fn test_armor_types() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da missing");

    let cols = table.column_names();
    let has_ac_bonus = cols.iter().any(|c| c.eq_ignore_ascii_case("AC_Enchant"));

    if has_ac_bonus {
        println!("\n=== Armor Types ===");

        for row_idx in 0..table.row_count() {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();

            if label.to_lowercase().contains("armor") || label.to_lowercase().contains("shield") {
                let ac = cell_value(table, row_idx, "AC_Enchant").unwrap_or_default();
                println!("  {label} - AC: {ac}");
            }
        }
    }
}

#[tokio::test]
async fn test_equipment_slots() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da missing");

    let mut slot_items: std::collections::HashMap<i32, Vec<String>> =
        std::collections::HashMap::new();

    for row_idx in 0..table.row_count() {
        let slots = cell_int(table, row_idx, "EquipableSlots").unwrap_or(0);

        if slots > 0 {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();
            slot_items.entry(slots).or_default().push(label);
        }
    }

    println!("\n=== Equipment Slot Distribution ===");
    let mut sorted: Vec<_> = slot_items.iter().collect();
    sorted.sort_by_key(|(slot, _)| *slot);

    for (slot, items) in sorted.iter().take(10) {
        println!(
            "  Slot {}: {} items (e.g. {})",
            slot,
            items.len(),
            items.first().unwrap_or(&String::new())
        );
    }
}

#[tokio::test]
async fn test_item_property_tables() {
    let ctx = create_test_context().await;

    let iprp_abilities = ctx.game_data.get_table("iprp_abilities");
    if let Some(table) = iprp_abilities {
        println!("\n=== Item Property: Abilities ===");
        println!("Rows: {}", table.row_count());
        for row_idx in 0..table.row_count().min(10) {
            let label = cell_value(table, row_idx, "Label").unwrap_or_default();
            println!("  {row_idx} - {label}");
        }
    }

    let iprp_bonuscost = ctx.game_data.get_table("iprp_bonuscost");
    if let Some(table) = iprp_bonuscost {
        println!("\n=== Item Property: Bonus Cost ===");
        println!("Rows: {}", table.row_count());
    }
}

#[tokio::test]
async fn test_weapon_damage_types() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da missing");

    let cols = table.column_names();
    let has_damage_type = cols.iter().any(|c| c.eq_ignore_ascii_case("WeaponType"));

    if has_damage_type {
        let mut weapon_types: std::collections::HashMap<i32, usize> =
            std::collections::HashMap::new();

        for row_idx in 0..table.row_count() {
            let wtype = cell_int(table, row_idx, "WeaponType").unwrap_or(0);

            if wtype > 0 {
                *weapon_types.entry(wtype).or_insert(0) += 1;
            }
        }

        println!("\n=== Weapon Type Distribution ===");
        let mut sorted: Vec<_> = weapon_types.into_iter().collect();
        sorted.sort_by_key(|(k, _)| *k);

        for (wtype, count) in sorted {
            println!("  Type {wtype}: {count} weapons");
        }
    }
}

#[tokio::test]
async fn test_critical_threat_ranges() {
    let ctx = create_test_context().await;

    let table = ctx
        .game_data
        .get_table("baseitems")
        .expect("baseitems.2da missing");

    let cols = table.column_names();
    let has_crit = cols.iter().any(|c| c.eq_ignore_ascii_case("CritThreat"));

    if has_crit {
        println!("\n=== Critical Threat Ranges ===");

        let sword_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "Label")
                .map(|l| l.to_lowercase().contains("longsword"))
                .unwrap_or(false)
        });

        if let Some(row) = sword_row {
            let crit = cell_value(table, row, "CritThreat").unwrap_or_default();
            let mult = cell_value(table, row, "CritHitMult").unwrap_or_default();
            println!("  Longsword: threat={crit}, mult={mult}");
        }

        let scythe_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "Label")
                .map(|l| l.to_lowercase().contains("scythe"))
                .unwrap_or(false)
        });

        if let Some(row) = scythe_row {
            let crit = cell_value(table, row, "CritThreat").unwrap_or_default();
            let mult = cell_value(table, row, "CritHitMult").unwrap_or_default();
            println!("  Scythe: threat={crit}, mult={mult}");
        }
    }
}
