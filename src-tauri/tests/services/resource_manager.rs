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

fn fixture_module(save: &str) -> app_lib::services::ModuleContent {
    let save_dir = common::fixtures_path().join("saves").join(save);
    let module_id = std::fs::read_to_string(save_dir.join("currentmodule.txt")).unwrap();
    app_lib::services::campaign::content::save_module_content(&save_dir, module_id.trim()).unwrap()
}

fn apply(
    rm: &mut app_lib::services::ResourceManager,
    content: Option<app_lib::services::ModuleContent>,
) {
    let paths = app_lib::config::NWN2Paths::new();
    rm.set_module_content(app_lib::services::ResourceManager::scan_module_content(
        &paths, content,
    ));
}

#[tokio::test]
async fn westgate_save_loads_its_haks_and_custom_tlk() {
    let context = common::create_test_context().await;
    let module = fixture_module("Westgate_Campaign");
    assert_eq!(module.name, "westgate_ar1500");
    assert_eq!(module.hak_list, ["westgate", "westgate2"]);
    assert_eq!(module.custom_tlk, "westgate");

    let mut rm = context.resource_manager.write().await;
    let vanilla_custom_ref = rm.get_string(16_777_216);
    assert!(vanilla_custom_ref.starts_with("<StrRef:"));

    apply(&mut rm, Some(module.clone()));
    assert_eq!(rm.module_content(), Some(&module));
    assert!(rm.resource_source_counts().get("hak").copied().unwrap_or(0) > 0);
    assert!(
        rm.resource_source_counts()
            .get("module")
            .copied()
            .unwrap_or(0)
            > 0
    );
    let custom = rm.get_string(16_777_216);
    assert!(
        !custom.starts_with("<StrRef:"),
        "custom TLK strrefs resolve through westgate.tlk"
    );
    assert_ne!(
        rm.get_string(0),
        custom,
        "base strrefs keep resolving through dialog.tlk"
    );

    let hak_2da = rm.get_available_2da_files().into_iter().find(|name| {
        rm.get_2da(name).is_ok()
            && !std::sync::Arc::ptr_eq(
                &rm.get_2da(name).unwrap(),
                &rm.get_2da_with_overrides(name).unwrap(),
            )
    });
    assert!(
        hak_2da.is_some(),
        "some base 2DA must be overridden by the module content"
    );

    apply(&mut rm, None);
    assert_eq!(rm.module_content(), None);
    assert_eq!(rm.resource_source_counts().get("hak"), None);
    assert_eq!(rm.resource_source_counts().get("module"), None);
    assert!(rm.get_custom_tlk_parser().is_none());
    assert_eq!(rm.get_string(16_777_216), vanilla_custom_ref);
}

#[tokio::test]
async fn storm_of_zehir_campaign_folder_overrides_base_skills() {
    let context = common::create_test_context().await;
    let mut rm = context.resource_manager.write().await;
    apply(&mut rm, Some(fixture_module("STORM_Campaign")));

    let base = rm.get_2da("skills").unwrap();
    let resolved = rm.get_2da_with_overrides("skills").unwrap();
    assert!(
        !std::sync::Arc::ptr_eq(&base, &resolved),
        "SoZ ships skills.2da in its campaign folder"
    );
}

#[tokio::test]
async fn directory_module_in_documents_resolves() {
    let context = common::create_test_context().await;
    let module = fixture_module("Community_Campaign");
    assert_eq!(module.name, "poe_intro");

    let mut rm = context.resource_manager.write().await;
    apply(&mut rm, Some(module));
    let Some(module_path) = rm.module_path() else {
        println!("poe_intro is not installed, skipping");
        return;
    };
    assert!(module_path.is_dir());
    assert!(
        rm.resource_source_counts()
            .get("module")
            .copied()
            .unwrap_or(0)
            > 0
    );
}
