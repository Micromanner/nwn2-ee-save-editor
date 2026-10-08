use super::super::common::create_test_context;

use app_lib::utils::PrerequisiteGraph;

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
async fn test_build_prerequisite_graph_from_real_feats() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let mut feat_data = Vec::new();

        for row_idx in 0..table.row_count() {
            let mut feat_row = std::collections::HashMap::new();

            feat_row.insert("__row_id__".to_string(), serde_json::json!(row_idx));

            if let Some(label) = cell_value(table, row_idx, "LABEL") {
                feat_row.insert("label".to_string(), serde_json::json!(label));
            }

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT1") {
                feat_row.insert("prereqfeat1".to_string(), serde_json::json!(id));
            }

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT2") {
                feat_row.insert("prereqfeat2".to_string(), serde_json::json!(id));
            }

            if let Some(val) = cell_int(table, row_idx, "MINSTR")
                && val > 0
            {
                feat_row.insert("minstr".to_string(), serde_json::json!(val));
            }

            if let Some(val) = cell_int(table, row_idx, "MINDEX")
                && val > 0
            {
                feat_row.insert("mindex".to_string(), serde_json::json!(val));
            }

            if let Some(val) = cell_int(table, row_idx, "MINLEVEL")
                && val > 0
            {
                feat_row.insert("minlevel".to_string(), serde_json::json!(val));
            }

            if let Some(val) = cell_int(table, row_idx, "MINATTACKBONUS")
                && val > 0
            {
                feat_row.insert("minattackbonus".to_string(), serde_json::json!(val));
            }

            feat_data.push(feat_row);
        }

        let mut graph = PrerequisiteGraph::new();
        let result = graph.build_from_data(&feat_data);

        assert!(result.is_ok(), "Should build graph from real feat data");

        let stats = graph.get_statistics();
        println!("=== Prerequisite Graph Statistics ===");
        for (key, value) in &stats {
            println!("  {key}: {value}");
        }

        let total_feats = stats
            .get("total_feats")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(
            total_feats > 100,
            "Should have many feats (got {total_feats})"
        );
    }
}

#[tokio::test]
async fn test_power_attack_cleave_chain() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let power_attack_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "LABEL")
                .map(|l| l.eq_ignore_ascii_case("Power_Attack"))
                .unwrap_or(false)
        });

        let cleave_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "LABEL")
                .map(|l| l.eq_ignore_ascii_case("Cleave"))
                .unwrap_or(false)
        });

        if let (Some(pa_idx), Some(cleave_idx)) = (power_attack_row, cleave_row) {
            println!("Power Attack: row {pa_idx}");
            println!("Cleave: row {cleave_idx}");

            let cleave_prereq = cell_int(table, cleave_idx, "PREREQFEAT1");

            if let Some(prereq) = cleave_prereq {
                assert_eq!(
                    prereq as usize, pa_idx,
                    "Cleave should require Power Attack"
                );
                println!("Verified: Cleave requires Power Attack (feat {prereq})");
            }
        }
    }
}

#[tokio::test]
async fn test_validate_feat_with_real_graph() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let mut feat_data = Vec::new();

        for row_idx in 0..table.row_count().min(500) {
            let mut feat_row = std::collections::HashMap::new();
            feat_row.insert("__row_id__".to_string(), serde_json::json!(row_idx));

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT1") {
                feat_row.insert("prereqfeat1".to_string(), serde_json::json!(id));
            }
            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT2") {
                feat_row.insert("prereqfeat2".to_string(), serde_json::json!(id));
            }

            feat_data.push(feat_row);
        }

        let mut graph = PrerequisiteGraph::new();
        graph.build_from_data(&feat_data).unwrap();

        let cleave_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "LABEL")
                .map(|l| l.eq_ignore_ascii_case("Cleave"))
                .unwrap_or(false)
        });

        let power_attack_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "LABEL")
                .map(|l| l.eq_ignore_ascii_case("Power_Attack"))
                .unwrap_or(false)
        });

        if let (Some(cleave_idx), Some(pa_idx)) = (cleave_row, power_attack_row) {
            let (valid_without, reasons) =
                graph.validate_feat_prerequisites_fast(cleave_idx as u32, &[], None);
            assert!(!valid_without, "Cleave should fail without Power Attack");
            println!("Cleave without Power Attack: {reasons:?}");

            let (valid_with, _) =
                graph.validate_feat_prerequisites_fast(cleave_idx as u32, &[pa_idx as u32], None);
            assert!(valid_with, "Cleave should pass with Power Attack");
            println!("Cleave with Power Attack: VALID");
        }
    }
}

#[tokio::test]
async fn test_get_feat_requirements_chain() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let mut feat_data = Vec::new();

        for row_idx in 0..table.row_count().min(500) {
            let mut feat_row = std::collections::HashMap::new();
            feat_row.insert("__row_id__".to_string(), serde_json::json!(row_idx));

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT1") {
                feat_row.insert("prereqfeat1".to_string(), serde_json::json!(id));
            }
            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT2") {
                feat_row.insert("prereqfeat2".to_string(), serde_json::json!(id));
            }

            feat_data.push(feat_row);
        }

        let mut graph = PrerequisiteGraph::new();
        graph.build_from_data(&feat_data).unwrap();

        let great_cleave_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "LABEL")
                .map(|l| l.eq_ignore_ascii_case("Great_Cleave"))
                .unwrap_or(false)
        });

        if let Some(gc_idx) = great_cleave_row {
            let requirements = graph.get_all_feat_requirements(gc_idx as u32);

            println!("Great Cleave (feat {gc_idx}) requires feats: {requirements:?}");

            assert!(
                !requirements.is_empty(),
                "Great Cleave should have prerequisites"
            );
        }
    }
}

#[tokio::test]
async fn test_batch_validation_performance() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let mut feat_data = Vec::new();

        for row_idx in 0..table.row_count() {
            let mut feat_row = std::collections::HashMap::new();
            feat_row.insert("__row_id__".to_string(), serde_json::json!(row_idx));

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT1") {
                feat_row.insert("prereqfeat1".to_string(), serde_json::json!(id));
            }

            feat_data.push(feat_row);
        }

        let mut graph = PrerequisiteGraph::new();
        graph.build_from_data(&feat_data).unwrap();

        let test_feats: Vec<u32> = (0..100).collect();

        let start = std::time::Instant::now();
        let results = graph.validate_batch_fast(test_feats.clone(), &[], None);
        let duration = start.elapsed();

        println!(
            "Batch validated {} feats in {:?}",
            test_feats.len(),
            duration
        );
        println!(
            "Results: {} valid, {} invalid",
            results.values().filter(|(v, _)| *v).count(),
            results.values().filter(|(v, _)| !*v).count()
        );

        assert!(
            duration.as_millis() < 100,
            "Batch validation should be fast"
        );
    }
}

#[tokio::test]
async fn test_graph_statistics() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let mut feat_data = Vec::new();

        for row_idx in 0..table.row_count() {
            let mut feat_row = std::collections::HashMap::new();
            feat_row.insert("__row_id__".to_string(), serde_json::json!(row_idx));

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT1") {
                feat_row.insert("prereqfeat1".to_string(), serde_json::json!(id));
            }
            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT2") {
                feat_row.insert("prereqfeat2".to_string(), serde_json::json!(id));
            }

            feat_data.push(feat_row);
        }

        let mut graph = PrerequisiteGraph::new();
        graph.build_from_data(&feat_data).unwrap();

        let stats = graph.get_statistics();

        assert!(stats.get("is_built").and_then(|v| v.as_bool()) == Some(true));
        assert!(
            stats
                .get("total_feats")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                > 0
        );
        assert!(
            stats
                .get("feats_with_prerequisites")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                > 0
        );
        assert!(
            stats
                .get("max_chain_depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                > 0
        );

        println!("Graph built with:");
        println!("  Total feats: {}", stats.get("total_feats").unwrap());
        println!(
            "  With prereqs: {}",
            stats.get("feats_with_prerequisites").unwrap()
        );
        println!("  Max depth: {}", stats.get("max_chain_depth").unwrap());
        println!("  Build time: {}ms", stats.get("build_time_ms").unwrap());
    }
}

#[tokio::test]
async fn test_direct_prerequisites_lookup() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let mut feat_data = Vec::new();

        for row_idx in 0..table.row_count().min(500) {
            let mut feat_row = std::collections::HashMap::new();
            feat_row.insert("__row_id__".to_string(), serde_json::json!(row_idx));

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT1") {
                feat_row.insert("prereqfeat1".to_string(), serde_json::json!(id));
            }
            if let Some(val) = cell_int(table, row_idx, "MINSTR")
                && val > 0
            {
                feat_row.insert("minstr".to_string(), serde_json::json!(val));
            }

            feat_data.push(feat_row);
        }

        let mut graph = PrerequisiteGraph::new();
        graph.build_from_data(&feat_data).unwrap();

        let power_attack_row = (0..table.row_count()).find(|&row| {
            cell_value(table, row, "LABEL")
                .map(|l| l.eq_ignore_ascii_case("Power_Attack"))
                .unwrap_or(false)
        });

        if let Some(pa_idx) = power_attack_row {
            let prereqs = graph.get_direct_prerequisites(pa_idx as u32);

            println!("Power Attack direct prerequisites:");
            for (key, value) in &prereqs {
                println!("  {key}: {value}");
            }

            assert!(prereqs.contains_key("feats"));
            assert!(prereqs.contains_key("abilities"));
            assert!(prereqs.contains_key("level"));
        }
    }
}

#[tokio::test]
async fn test_epic_feat_prerequisites() {
    let ctx = create_test_context().await;

    if let Some(table) = ctx.game_data.get_table("feat") {
        let mut feat_data = Vec::new();

        for row_idx in 0..table.row_count() {
            let mut feat_row = std::collections::HashMap::new();
            feat_row.insert("__row_id__".to_string(), serde_json::json!(row_idx));

            if let Some(id) = cell_int(table, row_idx, "PREREQFEAT1") {
                feat_row.insert("prereqfeat1".to_string(), serde_json::json!(id));
            }
            if let Some(val) = cell_int(table, row_idx, "MINLEVEL")
                && val > 0
            {
                feat_row.insert("minlevel".to_string(), serde_json::json!(val));
            }

            feat_data.push(feat_row);
        }

        let mut graph = PrerequisiteGraph::new();
        graph.build_from_data(&feat_data).unwrap();

        let epic_feats: Vec<_> = (0..table.row_count())
            .filter(|&row| {
                cell_value(table, row, "LABEL")
                    .map(|l| l.to_uppercase().contains("EPIC"))
                    .unwrap_or(false)
            })
            .take(5)
            .collect();

        println!("Epic feat prerequisite analysis:");
        for row in epic_feats {
            let label = cell_value(table, row, "LABEL").unwrap_or_default();
            let prereqs = graph.get_direct_prerequisites(row as u32);
            let level_req = prereqs.get("level").and_then(|v| v.as_u64()).unwrap_or(0);

            println!("  {label} (row {row}): min level {level_req}");
        }
    }
}
