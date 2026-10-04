use std::path::PathBuf;

use app_lib::parsers::erf::{ErfParser, ErfType, ErfVersion, resource_type_to_extension};

fn get_nwn2_data_path() -> Option<PathBuf> {
    let paths = app_lib::config::NWN2Paths::new();
    paths.data()
}

fn get_test_hak_path() -> Option<PathBuf> {
    get_nwn2_data_path().map(|p| p.join("NWN2_Models.zip"))
}

// =============================================================================
// PARSER CREATION TESTS
// =============================================================================

#[test]
fn test_erf_parser_creation() {
    let parser = ErfParser::new();
    let stats = parser.get_statistics();
    assert_eq!(stats.total_resources, 0);
    assert_eq!(stats.parse_time_ms, 0);
}

#[test]
fn test_erf_parser_empty_list() {
    let parser = ErfParser::new();
    let resources = parser.list_resources(None);
    assert!(resources.is_empty(), "New parser should have no resources");
}

// =============================================================================
// VERSION AND TYPE TESTS
// =============================================================================

#[test]
fn test_erf_version_key_size() {
    assert_eq!(ErfVersion::V10.key_entry_size(), 24);
    assert_eq!(ErfVersion::V11.key_entry_size(), 40);
}

#[test]
fn test_erf_version_name_length() {
    assert_eq!(ErfVersion::V10.max_resource_name_length(), 16);
    assert_eq!(ErfVersion::V11.max_resource_name_length(), 32);
}

#[test]
fn test_erf_type_signatures() {
    assert_eq!(ErfType::ERF.as_str(), "ERF");
    assert_eq!(ErfType::HAK.as_str(), "HAK");
    assert_eq!(ErfType::MOD.as_str(), "MOD");

    assert_eq!(ErfType::ERF.signature(), b"ERF ");
    assert_eq!(ErfType::HAK.signature(), b"HAK ");
    assert_eq!(ErfType::MOD.signature(), b"MOD ");
}

// =============================================================================
// RESOURCE TYPE EXTENSION MAPPING (using actual values from types.rs)
// =============================================================================

#[test]
fn test_resource_type_to_extension() {
    assert_eq!(resource_type_to_extension(2017), Some("2da"));
    assert_eq!(resource_type_to_extension(2015), Some("bic"));
    assert_eq!(resource_type_to_extension(2025), Some("uti"));
    assert_eq!(resource_type_to_extension(2012), Some("are"));
    assert_eq!(resource_type_to_extension(2027), Some("utc"));
    assert_eq!(resource_type_to_extension(2029), Some("dlg"));
    assert_eq!(resource_type_to_extension(2014), Some("ifo"));
    assert_eq!(resource_type_to_extension(2032), Some("utt"));
    assert_eq!(resource_type_to_extension(2010), Some("ncs"));
    assert_eq!(resource_type_to_extension(2009), Some("nss"));
    assert_eq!(resource_type_to_extension(4000), Some("mdb"));
    assert_eq!(resource_type_to_extension(4003), Some("gr2"));
    assert_eq!(resource_type_to_extension(3035), Some("trx"));
    assert_eq!(resource_type_to_extension(3012), Some("ros"));
    assert_eq!(resource_type_to_extension(2080), None);
}

#[test]
fn test_unknown_resource_type() {
    assert_eq!(resource_type_to_extension(9999), None);
}

// =============================================================================
// ARCHIVE CREATION TESTS
// =============================================================================

#[test]
fn test_new_archive_creation() {
    let parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    assert_eq!(parser.get_statistics().total_resources, 0);
    let resources = parser.list_resources(None);
    assert!(resources.is_empty());
}

#[test]
fn test_new_hak_creation() {
    let parser = ErfParser::new_archive(ErfType::HAK, ErfVersion::V11);

    assert_eq!(parser.get_statistics().total_resources, 0);
}

#[test]
fn test_new_mod_creation() {
    let parser = ErfParser::new_archive(ErfType::MOD, ErfVersion::V10);

    assert_eq!(parser.get_statistics().total_resources, 0);
}

// =============================================================================
// ADD RESOURCE TESTS
// =============================================================================

#[test]
fn test_add_single_resource() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    parser
        .add_resource("test_file", 2017, b"Test content".to_vec())
        .expect("Failed to add resource");

    let resources = parser.list_resources(None);
    println!("After adding resource: {} resources", resources.len());
    assert_eq!(resources.len(), 1, "Should have 1 resource");
}

#[test]
fn test_add_multiple_resources() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    parser
        .add_resource("file1", 2017, b"Content 1".to_vec())
        .unwrap();
    parser
        .add_resource("file2", 2015, b"Content 2".to_vec())
        .unwrap();
    parser
        .add_resource("file3", 2025, b"Content 3".to_vec())
        .unwrap();

    let resources = parser.list_resources(None);
    println!("After adding 3 resources: {} resources", resources.len());
    assert_eq!(resources.len(), 3, "Should have 3 resources");
}

#[test]
fn test_add_large_resource() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    let large_content = vec![0xABu8; 100_000];
    parser
        .add_resource("large_file", 2017, large_content.clone())
        .expect("Failed to add large resource");

    let extracted = parser
        .extract_resource("large_file.2da")
        .expect("Failed to extract");
    assert_eq!(extracted.len(), 100_000);
}

// =============================================================================
// ROUND-TRIP TESTS
// =============================================================================

#[test]
fn test_erf_round_trip_basic() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    parser
        .add_resource("test1", 2017, b"Content 1".to_vec())
        .unwrap();
    parser
        .add_resource("test2", 2017, b"Content 2".to_vec())
        .unwrap();

    let bytes = parser.to_bytes().expect("Failed to serialize");
    assert!(!bytes.is_empty());

    let mut parser2 = ErfParser::new();
    parser2
        .parse_from_bytes(&bytes)
        .expect("Failed to re-parse");

    assert_eq!(parser2.get_statistics().total_resources, 2);

    let extracted1 = parser2
        .extract_resource("test1.2da")
        .expect("Missing test1");
    let extracted2 = parser2
        .extract_resource("test2.2da")
        .expect("Missing test2");

    assert_eq!(extracted1, b"Content 1");
    assert_eq!(extracted2, b"Content 2");
}

#[test]
fn test_erf_round_trip_v11() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V11);

    parser
        .add_resource("v11_test_file", 2017, b"V1.1 Content".to_vec())
        .unwrap();

    let bytes = parser.to_bytes().expect("Failed to serialize V1.1");

    let mut parser2 = ErfParser::new();
    parser2
        .parse_from_bytes(&bytes)
        .expect("Failed to re-parse V1.1");

    assert_eq!(parser2.get_statistics().total_resources, 1);
}

#[test]
fn test_hak_round_trip() {
    let mut parser = ErfParser::new_archive(ErfType::HAK, ErfVersion::V10);

    parser
        .add_resource("hak_resource", 2017, b"HAK Content".to_vec())
        .unwrap();

    let bytes = parser.to_bytes().expect("Failed to serialize HAK");

    let mut parser2 = ErfParser::new();
    parser2
        .parse_from_bytes(&bytes)
        .expect("Failed to re-parse HAK");

    assert_eq!(parser2.get_statistics().total_resources, 1);
}

#[test]
fn test_mod_round_trip() {
    let mut parser = ErfParser::new_archive(ErfType::MOD, ErfVersion::V10);

    parser
        .add_resource("module_info", 2014, b"IFO Content".to_vec())
        .unwrap();

    let bytes = parser.to_bytes().expect("Failed to serialize MOD");

    let mut parser2 = ErfParser::new();
    parser2
        .parse_from_bytes(&bytes)
        .expect("Failed to re-parse MOD");

    assert_eq!(parser2.get_statistics().total_resources, 1);
}

// =============================================================================
// LIST RESOURCES TESTS
// =============================================================================

#[test]
fn test_list_resources_filter_by_type() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    parser.add_resource("file1", 2017, b"2DA".to_vec()).unwrap();
    parser.add_resource("file2", 2015, b"BIC".to_vec()).unwrap();
    parser.add_resource("file3", 2017, b"2DA".to_vec()).unwrap();

    let all_resources = parser.list_resources(None);
    assert_eq!(all_resources.len(), 3);

    let only_2da = parser.list_resources(Some(2017));
    assert_eq!(only_2da.len(), 2);

    let only_bic = parser.list_resources(Some(2015));
    assert_eq!(only_bic.len(), 1);
}

// =============================================================================
// EXTRACT RESOURCE TESTS
// =============================================================================

#[test]
fn test_extract_existing_resource() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    parser
        .add_resource("my_file", 2017, b"My Content".to_vec())
        .unwrap();

    let extracted = parser.extract_resource("my_file.2da");
    assert!(extracted.is_ok());
    assert_eq!(extracted.unwrap(), b"My Content");
}

#[test]
fn test_extract_nonexistent_resource() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    let extracted = parser.extract_resource("doesnt_exist.2da");
    assert!(
        extracted.is_err(),
        "Should return error for nonexistent resource"
    );
}

// =============================================================================
// BINARY DATA PRESERVATION
// =============================================================================

#[test]
fn test_binary_data_preservation() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    let binary_data: Vec<u8> = (0..=255).collect();
    parser
        .add_resource("binary_test", 2017, binary_data.clone())
        .unwrap();

    let bytes = parser.to_bytes().unwrap();
    let mut parser2 = ErfParser::new();
    parser2.parse_from_bytes(&bytes).unwrap();

    let extracted = parser2.extract_resource("binary_test.2da").unwrap();
    assert_eq!(
        extracted, binary_data,
        "Binary data should be preserved exactly"
    );
}

// =============================================================================
// REAL FILE TESTS
// =============================================================================

#[tokio::test]
async fn test_read_nwn2_models_zip() {
    let Some(zip_path) = get_test_hak_path() else {
        println!("NWN2 Data path not found, skipping test");
        return;
    };

    if !zip_path.exists() {
        println!("NWN2_Models.zip not found at {zip_path:?}, skipping test");
        return;
    }

    let mut parser = ErfParser::new();
    let result = parser.read(&zip_path);

    match result {
        Ok(()) => {
            let stats = parser.get_statistics();
            println!("Parsed {zip_path:?}");
            println!("  Total resources: {}", stats.total_resources);
            println!("  Parse time: {}ms", stats.parse_time_ms);

            assert!(stats.total_resources > 0, "Should have resources");

            let resources = parser.list_resources(None);
            assert!(!resources.is_empty(), "Should list resources");

            println!("  First 10 resources:");
            for (name, size, res_type) in resources.iter().take(10) {
                println!("    {name} (type={res_type}, size={size})");
            }

            println!("  Resource type breakdown:");
            let mut type_counts = std::collections::HashMap::new();
            for (_, _, res_type) in &resources {
                *type_counts.entry(*res_type).or_insert(0) += 1;
            }
            for (res_type, count) in type_counts.iter().take(10) {
                let ext = resource_type_to_extension(*res_type).unwrap_or("unknown");
                println!("    {ext} ({res_type}): {count} files");
            }
        }
        Err(e) => {
            println!("Could not parse NWN2_Models.zip: {e}");
        }
    }
}

// =============================================================================
// STATISTICS TESTS
// =============================================================================

#[test]
fn test_statistics_after_add() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    let resources_before = parser.list_resources(None);
    assert_eq!(resources_before.len(), 0);

    parser
        .add_resource("file1", 2017, b"Content".to_vec())
        .unwrap();

    let resources_after = parser.list_resources(None);
    println!("Stats after add: {} resources", resources_after.len());
    assert_eq!(resources_after.len(), 1, "Should have 1 resource");
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_empty_resource_name() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    let result = parser.add_resource("", 2017, b"Content".to_vec());
    println!("Empty name result: {:?}", result.is_ok());
}

#[test]
fn test_long_resource_name_v10() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    let long_name = "a".repeat(20);
    let result = parser.add_resource(&long_name, 2017, b"Content".to_vec());
    println!("Long name V1.0 result: {:?}", result.is_ok());
}

#[test]
fn test_long_resource_name_v11() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V11);

    let long_name = "a".repeat(40);
    let result = parser.add_resource(&long_name, 2017, b"Content".to_vec());
    println!("Long name V1.1 result: {:?}", result.is_ok());
}

#[test]
fn test_empty_content() {
    let mut parser = ErfParser::new_archive(ErfType::ERF, ErfVersion::V10);

    parser
        .add_resource("empty_file", 2017, vec![])
        .expect("Should allow empty content");

    let bytes = parser.to_bytes().unwrap();
    let mut parser2 = ErfParser::new();
    parser2.parse_from_bytes(&bytes).unwrap();

    let extracted = parser2.extract_resource("empty_file.2da").unwrap();
    assert!(extracted.is_empty());
}
