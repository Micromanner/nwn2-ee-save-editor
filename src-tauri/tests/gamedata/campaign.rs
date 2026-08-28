use std::path::PathBuf;

use app_lib::parsers::xml::RustXmlParser;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load_campaign_globals(campaign: &str) -> Option<String> {
    let path = fixtures_path()
        .join("saves")
        .join(campaign)
        .join("globals.xml");
    if path.exists() {
        std::fs::read_to_string(&path).ok()
    } else {
        None
    }
}

#[test]
fn test_motb_campaign_globals() {
    let Some(content) = load_campaign_globals("MOTB") else {
        println!("MOTB globals.xml not found, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse MOTB globals.xml");

    println!("\n=== MOTB Campaign Data ===");

    let info = parser.get_general_info();
    println!("General info keys: {:?}", info.keys().collect::<Vec<_>>());

    let overview = parser.get_quest_overview_struct();
    println!("Total quest variables: {}", overview.total_quest_vars);

    for (group_name, group) in overview.quest_groups.iter().take(5) {
        println!(
            "  {}: {} completed, {} active",
            group_name,
            group.completed.len(),
            group.active.len()
        );
    }
}

#[test]
fn test_classic_campaign_globals() {
    let Some(content) = load_campaign_globals("Classic_Campaign") else {
        println!("Classic_Campaign globals.xml not found, skipping");
        return;
    };

    let parser =
        RustXmlParser::from_string(&content).expect("Failed to parse Classic Campaign globals.xml");

    println!("\n=== Classic (OC) Campaign Data ===");

    let info = parser.get_general_info();
    println!("General info entries: {}", info.len());

    let companions = parser.get_companion_status(&[]);
    println!("Companions tracked: {}", companions.len());

    for (name, status) in &companions {
        println!("  {} - influence: {:?}", name, status.influence);
    }
}

#[test]
fn test_soz_campaign_globals() {
    let Some(content) = load_campaign_globals("STORM_Campaign") else {
        println!("STORM_Campaign (SoZ) globals.xml not found, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse SoZ globals.xml");

    println!("\n=== Storm of Zehir Campaign Data ===");

    let info = parser.get_general_info();
    println!("General info entries: {}", info.len());

    let overview = parser.get_quest_overview_struct();
    println!("Quest groups: {}", overview.quest_groups.len());
}

#[test]
fn test_westgate_campaign_globals() {
    let Some(content) = load_campaign_globals("Westgate_Campaign") else {
        println!("Westgate_Campaign globals.xml not found, skipping");
        return;
    };

    let parser =
        RustXmlParser::from_string(&content).expect("Failed to parse Westgate globals.xml");

    println!("\n=== Mysteries of Westgate Campaign Data ===");

    let overview = parser.get_quest_overview_struct();
    println!("Total variables: {}", overview.total_quest_vars);
}

#[test]
fn test_community_campaign_globals() {
    let Some(content) = load_campaign_globals("Community_Campaign") else {
        println!("Community_Campaign globals.xml not found, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content)
        .expect("Failed to parse Community Campaign globals.xml");

    println!("\n=== Community Campaign Data ===");

    let info = parser.get_general_info();
    println!("Campaign entries: {}", info.len());
}

#[test]
fn test_compare_campaign_quest_counts() {
    println!("\n=== Campaign Quest Comparison ===");

    let campaigns = [
        "MOTB",
        "Classic_Campaign",
        "STORM_Campaign",
        "Westgate_Campaign",
    ];

    for campaign in campaigns {
        if let Some(content) = load_campaign_globals(campaign)
            && let Ok(parser) = RustXmlParser::from_string(&content)
        {
            let overview = parser.get_quest_overview_struct();
            let companions = parser.get_companion_status(&[]);

            println!(
                "{}: {} vars, {} groups, {} companions",
                campaign,
                overview.total_quest_vars,
                overview.quest_groups.len(),
                companions.len()
            );
        }
    }
}

#[test]
fn test_campaign_companion_influence() {
    let Some(content) = load_campaign_globals("MOTB") else {
        println!("MOTB globals.xml not found, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let companions = parser.get_companion_status(&[]);

    println!("\n=== MOTB Companion Influence ===");

    let mut sorted: Vec<_> = companions.iter().collect();
    sorted.sort_by(|a, b| {
        let inf_a = b.1.influence.unwrap_or(i32::MIN);
        let inf_b = a.1.influence.unwrap_or(i32::MIN);
        inf_a.cmp(&inf_b)
    });

    for (name, status) in sorted {
        println!(
            "  {} ({}) - Influence: {:?}",
            name, status.recruitment, status.influence
        );
    }
}

#[test]
fn test_save_directory_structure() {
    let saves_path = fixtures_path().join("saves");

    if !saves_path.exists() {
        println!("Saves fixtures not found");
        return;
    }

    println!("\n=== Save Directory Structure ===");

    for entry in std::fs::read_dir(&saves_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            let campaign_name = path.file_name().unwrap().to_string_lossy();

            let has_globals = path.join("globals.xml").exists();
            let has_playerinfo = path.join("playerinfo.bin").exists();
            let has_resgff = path.join("resgff.zip").exists();

            println!(
                "  {} - globals:{} playerinfo:{} resgff:{}",
                campaign_name,
                if has_globals { "Y" } else { "N" },
                if has_playerinfo { "Y" } else { "N" },
                if has_resgff { "Y" } else { "N" }
            );
        }
    }
}
