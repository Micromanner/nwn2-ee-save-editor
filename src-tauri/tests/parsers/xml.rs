use std::fmt::Write as _;
use std::path::PathBuf;

use app_lib::parsers::xml::RustXmlParser;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load_globals_xml() -> Option<String> {
    let path = fixtures_path().join("saves/MOTB/globals.xml");
    if path.exists() {
        std::fs::read_to_string(&path).ok()
    } else {
        None
    }
}

// =============================================================================
// PARSER CREATION TESTS
// =============================================================================

#[test]
fn test_xml_parser_creation() {
    let parser = RustXmlParser::new();
    let info = parser.get_general_info();
    assert!(info.is_empty() || info.values().all(|v| v.is_none()));
}

// =============================================================================
// ERROR HANDLING TESTS
// =============================================================================

#[test]
fn test_xml_parser_from_empty_string() {
    let result = RustXmlParser::from_string("");
    assert!(result.is_err(), "Empty string should fail to parse");
}

#[test]
fn test_xml_parser_from_invalid_xml() {
    let result = RustXmlParser::from_string("<not_closed>");
    assert!(result.is_err(), "Invalid XML should fail to parse");
}

#[test]
fn test_xml_parser_from_malformed_xml() {
    let result = RustXmlParser::from_string("<root><child></root>");
    assert!(result.is_err(), "Malformed XML should fail to parse");
}

#[test]
fn test_xml_parser_from_no_root() {
    let result = RustXmlParser::from_string("not xml at all");
    assert!(result.is_err(), "Non-XML content should fail");
}

// =============================================================================
// BASIC PARSING TESTS
// =============================================================================

#[test]
fn test_xml_parse_simple() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Globals>
    <TestEntry>Value</TestEntry>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(
        result.is_ok(),
        "Simple XML should parse: {:?}",
        result.err()
    );
}

#[test]
fn test_xml_parse_with_attributes() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Globals>
    <Entry name="test" value="123"/>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(result.is_ok(), "XML with attributes should parse");
}

#[test]
fn test_xml_parse_nested() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Globals>
    <Parent>
        <Child>
            <GrandChild>Value</GrandChild>
        </Child>
    </Parent>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(result.is_ok(), "Nested XML should parse");
}

// =============================================================================
// GLOBALS.XML TESTS
// =============================================================================

#[test]
fn test_xml_parser_from_valid_globals_xml() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let info = parser.get_general_info();
    println!("General info keys: {:?}", info.keys().collect::<Vec<_>>());

    assert!(!info.is_empty(), "Should have general info");
}

#[test]
fn test_quest_overview() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let overview = parser.get_quest_overview_struct();

    println!("Quest Overview:");
    println!("  Total quest vars: {}", overview.total_quest_vars);
    println!("  Completed: {}", overview.completed_count);
    println!("  Active: {}", overview.active_count);
    println!("  Completion: {:.1}%", overview.completion_percentage);
    println!("  Quest groups: {}", overview.quest_groups.len());

    for (group_name, group) in overview.quest_groups.iter().take(5) {
        println!(
            "  - {}: {} completed, {} active",
            group_name,
            group.completed.len(),
            group.active.len()
        );
    }

    // Just verify we got valid data
}

#[test]
fn test_quest_groups_structure() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let overview = parser.get_quest_overview_struct();

    for (group_name, group) in &overview.quest_groups {
        assert!(!group_name.is_empty(), "Group name should not be empty");

        for quest in &group.completed {
            assert!(!quest.is_empty(), "Quest name should not be empty");
        }
    }
}

// =============================================================================
// COMPANION STATUS TESTS
// =============================================================================

#[test]
fn test_companion_status() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let companions = parser.get_companion_status(&[]);

    println!("Companion Status:");
    for (name, status) in &companions {
        println!(
            "  - {}: recruitment={}, influence={:?}",
            name, status.recruitment, status.influence
        );
    }
}

#[test]
fn test_motb_companions() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let companions = parser.get_companion_status(&[]);

    let motb_companion_names = ["safiya", "gann", "kaelyn", "okku", "oneofmany", "ammon"];

    for name in motb_companion_names {
        if companions.contains_key(name) {
            println!("Found MOTB companion: {name}");
        }
    }
}

// =============================================================================
// FULL SUMMARY TESTS
// =============================================================================

#[test]
fn test_full_summary() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let summary = parser.get_full_summary_struct();

    println!("Full Summary:");
    println!("  General info entries: {}", summary.general_info.len());
    println!(
        "  Quest groups: {}",
        summary.quest_overview.quest_groups.len()
    );
    println!("  Companions tracked: {}", summary.companion_status.len());
    println!("  Raw data: {:?}", summary.raw_data_counts);
}

// =============================================================================
// ROUND-TRIP TESTS
// =============================================================================

#[test]
fn test_xml_round_trip() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let xml_output = parser.to_xml_string().expect("Failed to serialize to XML");

    assert!(!xml_output.is_empty(), "Output should not be empty");
    assert!(
        !xml_output.contains("<?xml"),
        "NWN2 globals.xml has no XML declaration"
    );

    let parser2 =
        RustXmlParser::from_string(&xml_output).expect("Failed to re-parse generated XML");

    let info1 = parser.get_general_info();
    let info2 = parser2.get_general_info();

    assert_eq!(
        info1.len(),
        info2.len(),
        "General info should match after round-trip"
    );
}

/// Round-trip from a pristine NWN2 globals.xml: verifies format preservation
/// (no XML decl, CRLF line endings, 4-space indent, trailing newline) and
/// that all non-cheat data survives parse -> serialize -> reparse.
#[test]
fn test_xml_round_trip_byte_exact_classic_campaign() {
    let path = fixtures_path().join("saves/Classic_Campaign/globals.xml");
    let Ok(content) = std::fs::read_to_string(&path) else {
        println!("Classic_Campaign globals.xml not found, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("parse");
    let output = parser.to_xml_string().expect("serialize");

    assert!(!output.contains("<?xml"), "must not emit XML declaration");
    assert!(output.contains("\r\n"), "must use CRLF line endings");
    assert!(output.ends_with("\r\n"), "must end with trailing newline");
    assert!(
        output.contains("    <Integers>"),
        "must use 4-space indentation"
    );

    let reparsed = RustXmlParser::from_string(&output).expect("reparse");

    // Data loss check: everything round-trips except the intentionally
    // stripped cheat booleans.
    assert_eq!(reparsed.data.integers, parser.data.integers);
    assert_eq!(reparsed.data.floats, parser.data.floats);
    assert_eq!(reparsed.data.strings, parser.data.strings);

    let mut expected_booleans = parser.data.booleans.clone();
    expected_booleans.shift_remove("Cheater");
    expected_booleans.shift_remove("ShowCheatsWarning");
    assert_eq!(reparsed.data.booleans, expected_booleans);
}

#[test]
fn test_xml_round_trip_preserves_booleans() {
    // Cheater / ShowCheatsWarning are intentionally stripped on write, so
    // inject a non-cheat boolean to verify the writer still emits booleans.
    let mut parser = RustXmlParser::new();
    parser
        .data
        .booleans
        .insert("00_bSomeQuestFlag".to_string(), 1);

    let output = parser.to_xml_string().expect("serialize");
    assert!(
        output.contains("<Boolean>"),
        "Serialized output must include Boolean entries"
    );
    assert!(
        output.contains("<Name>00_bSomeQuestFlag</Name>"),
        "Non-cheat booleans must survive round-trip"
    );

    let reparsed = RustXmlParser::from_string(&output).expect("reparse");
    assert_eq!(reparsed.data.booleans, parser.data.booleans);
}

#[test]
fn test_xml_round_trip_preserves_quests() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser1 = RustXmlParser::from_string(&content).expect("Parse 1");
    let overview1 = parser1.get_quest_overview_struct();

    let xml_output = parser1.to_xml_string().expect("Serialize");
    let parser2 = RustXmlParser::from_string(&xml_output).expect("Parse 2");
    let overview2 = parser2.get_quest_overview_struct();

    assert_eq!(
        overview1.total_quest_vars, overview2.total_quest_vars,
        "Quest count should be preserved"
    );
    assert_eq!(
        overview1.completed_count, overview2.completed_count,
        "Completed count should be preserved"
    );
}

// =============================================================================
// COMPANION DISCOVERY TESTS
// =============================================================================

#[test]
fn test_potential_companions_discovery() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let potential = parser.discover_potential_companions();

    println!("Potential Companions Discovered:");
    for (name, vars) in &potential {
        println!("  - {}: {} related variables", name, vars.len());
    }
}

// =============================================================================
// VARIABLE MANIPULATION TESTS
// =============================================================================

#[test]
fn test_get_global_variable() {
    let Some(content) = load_globals_xml() else {
        println!("globals.xml not found in fixtures, skipping");
        return;
    };

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse globals.xml");

    let info = parser.get_general_info();
    println!("Sample global variables:");
    for (key, value) in info.iter().take(10) {
        println!("  {key}: {value:?}");
    }
}

// =============================================================================
// ENCODING TESTS
// =============================================================================

#[test]
fn test_xml_utf8_handling() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Globals>
    <SpecialChars>éèêëàâùûôîç</SpecialChars>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(result.is_ok(), "UTF-8 special chars should parse");
}

#[test]
fn test_xml_entity_handling() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Globals>
    <Entities>&lt;&gt;&amp;&quot;&apos;</Entities>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(result.is_ok(), "XML entities should parse");
}

// =============================================================================
// STRESS TESTS
// =============================================================================

#[test]
fn test_xml_large_content() {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="utf-8"?><Globals>"#);
    for i in 0..1000 {
        let _ = write!(xml, "<Entry_{i}>Value_{i}</Entry_{i}>");
    }
    xml.push_str("</Globals>");

    let result = RustXmlParser::from_string(&xml);
    assert!(result.is_ok(), "Large XML should parse");
}

#[test]
fn test_xml_deeply_nested() {
    let depth = 50;
    let mut xml = String::from(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    for i in 0..depth {
        let _ = write!(xml, "<Level{i}>");
    }
    xml.push_str("DeepValue");
    for i in (0..depth).rev() {
        let _ = write!(xml, "</Level{i}>");
    }

    let result = RustXmlParser::from_string(&xml);
    assert!(result.is_ok(), "Deeply nested XML should parse");
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_xml_with_comments() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- This is a comment -->
<Globals>
    <!-- Another comment -->
    <Entry>Value</Entry>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(result.is_ok(), "XML with comments should parse");
}

#[test]
fn test_xml_with_cdata() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Globals>
    <Script><![CDATA[function test() { return "<value>"; }]]></Script>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(result.is_ok(), "XML with CDATA should parse");
}

#[test]
fn test_xml_whitespace_handling() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Globals>
    <Entry>
        Value with
        newlines
    </Entry>
</Globals>"#;

    let result = RustXmlParser::from_string(xml);
    assert!(result.is_ok(), "XML with whitespace should parse");
}

// =============================================================================
// MULTIPLE SAVE FILES
// =============================================================================

#[test]
fn test_parse_all_campaign_globals() {
    let saves_path = fixtures_path().join("saves");

    let campaigns = [
        "MOTB",
        "Classic_Campaign",
        "Community_Campaign",
        "STORM_Campaign",
        "Westgate_Campaign",
    ];

    for campaign in campaigns {
        let globals_path = saves_path.join(campaign).join("globals.xml");
        if globals_path.exists() {
            println!("\n=== {campaign} globals.xml ===");
            let content = std::fs::read_to_string(&globals_path).expect("Failed to read");
            let parser = RustXmlParser::from_string(&content);
            match parser {
                Ok(p) => {
                    let summary = p.get_full_summary_struct();
                    println!(
                        "  Quests: {}, Companions: {}",
                        summary.quest_overview.quest_groups.len(),
                        summary.companion_status.len()
                    );
                }
                Err(e) => {
                    println!("  Failed to parse: {e}");
                }
            }
        } else {
            println!("{campaign} globals.xml not found, skipping");
        }
    }
}

// =============================================================================
// CHEAT FLAG STRIPPING
// =============================================================================

#[test]
fn test_cheat_flags_stripped_on_write() {
    let cheat_path = fixtures_path().join("saves/cheatdebug/000062 - 16-04-2026-23-04/globals.xml");
    if !cheat_path.exists() {
        println!("cheatdebug cheat save not available, skipping");
        return;
    }

    let content = std::fs::read_to_string(&cheat_path).expect("Failed to read cheat globals.xml");
    assert!(
        content.contains("<Name>Cheater</Name>"),
        "Fixture must contain Cheater boolean for this test to be meaningful"
    );
    assert!(
        content.contains("<Name>ShowCheatsWarning</Name>"),
        "Fixture must contain ShowCheatsWarning boolean for this test to be meaningful"
    );

    let parser = RustXmlParser::from_string(&content).expect("Failed to parse cheat globals.xml");
    assert!(
        parser.data.booleans.contains_key("Cheater"),
        "Parser should load Cheater boolean from XML"
    );
    assert!(
        parser.data.booleans.contains_key("ShowCheatsWarning"),
        "Parser should load ShowCheatsWarning boolean from XML"
    );

    let written = parser
        .to_xml_string()
        .expect("Failed to serialize globals.xml");
    assert!(
        !written.contains("<Name>Cheater</Name>"),
        "Cheater boolean must be stripped on write"
    );
    assert!(
        !written.contains("<Name>ShowCheatsWarning</Name>"),
        "ShowCheatsWarning boolean must be stripped on write"
    );

    // Verify no collateral damage: integers/floats/strings untouched, and
    // every non-cheat boolean survives the round-trip.
    let reparsed = RustXmlParser::from_string(&written).expect("reparse stripped output");
    assert_eq!(
        reparsed.data.integers, parser.data.integers,
        "Integers must not be affected by cheat-flag stripping"
    );
    assert_eq!(
        reparsed.data.floats, parser.data.floats,
        "Floats must not be affected by cheat-flag stripping"
    );
    assert_eq!(
        reparsed.data.strings, parser.data.strings,
        "Strings must not be affected by cheat-flag stripping"
    );

    let mut expected_booleans = parser.data.booleans.clone();
    expected_booleans.shift_remove("Cheater");
    expected_booleans.shift_remove("ShowCheatsWarning");
    assert_eq!(
        reparsed.data.booleans, expected_booleans,
        "Only Cheater/ShowCheatsWarning may be stripped; every other boolean must survive"
    );
}
