#[path = "../common/mod.rs"]
mod common;

#[tokio::test]
async fn enhanced_edition_textures_override_original_archives() {
    use std::io::Read;
    let context = common::create_test_context().await;
    let rm = context.resource_manager.read().await;
    let enhanced = common::nwn2_install_path().join("enhanced/data");
    let skeleton_bytes = rm.get_resource_bytes("P_HHM_skel", "gr2").unwrap();
    assert!(
        app_lib::parsers::gr2::parser::Gr2Parser::parse(&skeleton_bytes).is_ok(),
        "EE visual resources must not replace skeletons with an unsupported GR2 layout"
    );
    let model = app_lib::services::model_loader::load_model(&rm, "P_HHM_PF_Body01", "body", "body")
        .unwrap();
    assert!(model.skeleton.is_some(), "plate preview needs its skeleton");
    assert!(
        !model.animations.is_empty(),
        "plate preview needs its idle animations"
    );
    for (archive, resref) in [
        ("upscale_initial.zip", "p_hhm_pf_body01"),
        ("upscale_initial.zip", "p_hhm_pf_body01_n"),
        ("nwn2_materials_noupscale.zip", "p_hhm_pf_body01_t"),
    ] {
        let mut zip =
            zip::ZipArchive::new(std::fs::File::open(enhanced.join(archive)).unwrap()).unwrap();
        let name = zip
            .file_names()
            .find(|name| name.to_lowercase().ends_with(&format!("/{resref}.dds")))
            .unwrap()
            .to_string();
        let mut expected = Vec::new();
        zip.by_name(&name)
            .unwrap()
            .read_to_end(&mut expected)
            .unwrap();
        let actual = rm.get_resource_bytes(resref, "dds").unwrap();
        assert_eq!(
            actual.len(),
            expected.len(),
            "wrong archive selected for {resref}"
        );
        assert!(actual == expected, "wrong texture bytes for {resref}");
    }
}

#[tokio::test]
async fn test_resource_manager_resolution() {
    let context = common::create_test_context().await;
    let resource_manager = context.resource_manager.read().await;

    // Test listing available 2DAs
    let available = resource_manager.get_available_2da_files();
    // In a real environment, this should not be empty, but in CI/test env without game data it might be.
    // However, create_test_context panics if dialog.tlk missing, so we assume some data exists.
    println!("Available 2DAs: {}", available.len());

    // Test getting a known 2DA
    // "feat" matches "feat.2da"
    if available.contains(&"feat".to_string()) {
        let result = resource_manager.get_2da("feat");
        assert!(result.is_ok());
        let parser = result.unwrap();
        assert!(parser.row_count() > 0);
    } else {
        println!("feat.2da not found in available files, skipping get_2da check");
    }

    let s = resource_manager.get_string(100);
    println!("String 100: {s}");
}

#[tokio::test]
async fn test_resource_manager_initialization() {
    let context = common::create_test_context().await;
    let resource_manager = context.resource_manager.read().await;

    let files = resource_manager.get_available_2da_files();
    println!("Indexed 2da count: {}", files.len());
}
