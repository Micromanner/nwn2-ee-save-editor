use app_lib::character::gff_helpers::gff_value_to_i32;
use app_lib::parsers::gff::{GffParser, GffValue};
use app_lib::services::item_cost_calculator::calculate_item_cost;
use app_lib::utils::parsing::row_str;
use indexmap::IndexMap;
use std::fmt::Write as _;

#[path = "../common/mod.rs"]
mod common;

async fn load_template(
    context: &common::TestContext,
    resref: &str,
) -> IndexMap<String, GffValue<'static>> {
    let rm = context.resource_manager.read().await;
    let bytes = rm
        .get_resource_bytes(resref, "uti")
        .expect("template exists");
    let gff = GffParser::from_bytes(bytes).expect("template parses");
    gff.read_struct_fields(0)
        .expect("root struct")
        .into_iter()
        .map(|(k, v)| (k, v.force_owned().expect("force owned")))
        .collect()
}

/// The toolset stores the computed cost on some templates; those pin the formula.
#[tokio::test]
async fn item_cost_matches_toolset_computed_templates() {
    let context = common::create_test_context().await;
    let game_data = context.loader.game_data().expect("game data");

    // (resref, stored Cost): multiplier 2.0 weapons with 3 properties, a
    // 1.25-multiplier item with 10 immunities whose squared sum dominates, and
    // items whose property cost lives in the SUBTYPE table (Damage Resistance
    // in `iprp_damagetype`, Improved Saving Throws in `iprp_saveelement`).
    for (resref, expected) in [
        ("cmi_bghound_bite", 38720),
        ("cmi_nightmare_bite", 15125),
        ("it_crewpmflyt001", 13260),
        ("iprop_immu_0", 2985957),
        ("nw_ring08", 871201),
        ("nw_helm09", 16405),
        ("nw_amulet06", 2500),
        // Cast Spell is gold added linearly: 1 + 22058 x 3.0.
        ("n2_itset_rog3", 66175),
        ("cmi_coin_jeannie", 4000),
        // Flat properties (no cost table) count at face value: Darkvision,
        // Immunity, Bonus Feat.
        ("kprc_nightchild", 37741),
        ("nw_amulet01", 57025),
        ("kprc_heartwarder", 36001),
        // Several Cast Spell properties: dearest in full, the rest at half.
        ("n2_pca_hscout", 27418),
        ("nx1_boots03", 218950),
        ("x2_it_wmgrd001", 91097),
    ] {
        let item = load_template(&context, resref).await;
        let cost = calculate_item_cost(&item, game_data).expect("cost computes");
        // The toolset stores Cost before ModifyCost; the engine (and we) add it.
        let modify_cost = i64::from(
            item.get("ModifyCost")
                .and_then(gff_value_to_i32)
                .unwrap_or(0),
        );
        let expected = expected + modify_cost;
        let tolerance = 3;
        assert!(
            (cost as i64 - expected).abs() <= tolerance,
            "{resref}: calculated {cost}, template stores {expected} (incl. ModifyCost {modify_cost})"
        );
    }
}

/// Dumps one CSV line per game `.uti` so the item cost formula can be fitted
/// against what the toolset actually stored. Run with `--ignored`.
#[tokio::test]
#[ignore = "diagnostic survey, writes ITEM_COST_SURVEY_OUT"]
async fn survey_item_costs() {
    let out_path = std::env::var("ITEM_COST_SURVEY_OUT").expect("ITEM_COST_SURVEY_OUT not set");
    let context = common::create_test_context().await;
    let rm = context.resource_manager.read().await;
    let game_data = context.loader.game_data().expect("game data");
    let baseitems = game_data.get_table("baseitems").unwrap();
    let itempropdef = game_data.get_table("itempropdef").unwrap();
    let costtable = game_data.get_table("iprp_costtable").unwrap();
    let paramtable = game_data.get_table("iprp_paramtable").unwrap();

    let mut csv = String::from(
        "resref,base_item,stored_cost,add_cost,stack,base_cost,multiplier,charges,props,prop_costs\n",
    );
    for (key, _) in rm.list_resources_by_extension("uti") {
        let resref = key.trim_end_matches(".uti");
        let Ok(bytes) = rm.get_resource_bytes(resref, "uti") else {
            continue;
        };
        let Ok(gff) = GffParser::from_bytes(bytes) else {
            continue;
        };
        let Ok(fields) = gff.read_struct_fields(0) else {
            continue;
        };
        let get = |n: &str| fields.get(n).and_then(gff_value_to_i32);
        let base_item = get("BaseItem").unwrap_or(-1);
        let Some(row) = baseitems.get_by_id(base_item) else {
            continue;
        };
        let base_cost = row_str(&row, "basecost").unwrap_or_default();
        let multiplier = row_str(&row, "itemmultiplier").unwrap_or_default();

        let mut prop_costs = Vec::new();
        if let Some(list) = fields.get("PropertiesList") {
            let list = list.clone().force_owned().expect("force owned");
            if let GffValue::ListOwned(props) = list {
                for prop in props {
                    let pget = |n: &str| prop.get(n).and_then(gff_value_to_i32).unwrap_or(0);
                    let id = pget("PropertyName");
                    let Some(def) = itempropdef.get_by_id(id) else {
                        continue;
                    };
                    let def_cost = row_str(&def, "cost").unwrap_or_default();
                    let ct = row_str(&def, "costtableresref")
                        .and_then(|s| s.parse::<i32>().ok())
                        .and_then(|i| costtable.get_by_id(i))
                        .and_then(|r| row_str(&r, "name"))
                        .map(|n| n.to_lowercase());
                    let ct_cost = ct
                        .as_ref()
                        .and_then(|t| game_data.get_table(t))
                        .and_then(|t| t.get_by_id(pget("CostValue")))
                        .and_then(|r| row_str(&r, "cost"))
                        .unwrap_or_default();
                    let pt = row_str(&def, "param1resref")
                        .and_then(|s| s.parse::<i32>().ok())
                        .and_then(|i| paramtable.get_by_id(i))
                        .and_then(|r| row_str(&r, "tableresref"))
                        .map(|n| n.to_lowercase());
                    let pt_cost = pt
                        .as_ref()
                        .and_then(|t| game_data.get_table(t))
                        .and_then(|t| t.get_by_id(pget("Param1Value")))
                        .and_then(|r| row_str(&r, "cost"))
                        .unwrap_or_default();
                    let sub_cost = row_str(&def, "subtyperesref")
                        .filter(|s| s != "****")
                        .map(|n| n.to_lowercase())
                        .and_then(|t| game_data.get_table(&t))
                        .and_then(|t| t.get_by_id(pget("Subtype")))
                        .and_then(|r| row_str(&r, "cost"))
                        .unwrap_or_default();
                    prop_costs.push(format!(
                        "{id}:{}:{}:{}:{def_cost}:{ct_cost}:{pt_cost}:{sub_cost}",
                        pget("Subtype"),
                        pget("CostValue"),
                        pget("Param1Value")
                    ));
                }
            }
        }
        let _ = writeln!(
            csv,
            "{resref},{base_item},{},{},{},{base_cost},{multiplier},{},{},{}",
            get("Cost").unwrap_or(-1),
            get("AddCost").unwrap_or(0),
            get("StackSize").unwrap_or(1),
            get("Charges").unwrap_or(0),
            prop_costs.len(),
            prop_costs.join(";")
        );
    }
    std::fs::write(&out_path, csv).unwrap();
}
