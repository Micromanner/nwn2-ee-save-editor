use super::super::common::create_test_context;
use app_lib::character::Character;
use app_lib::character::pets::PET_TYPE_NONE;
use indexmap::IndexMap;

#[tokio::test]
async fn test_pet_tables_loaded() {
    let ctx = create_test_context().await;

    for name in ["hen_companion", "hen_familiar"] {
        let table = ctx
            .loader
            .get_table(name)
            .unwrap_or_else(|| panic!("{name}.2da should be loaded"));
        assert!(table.row_count() > 0, "{name} should have rows");
    }
}

#[tokio::test]
async fn test_pet_options_resolve_tlk_names() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("game data loaded");

    let state = Character::from_gff(IndexMap::new())
        .expect("character from gff")
        .pets_state(game_data);

    assert_eq!(state.animal_companion.type_id, PET_TYPE_NONE);
    assert_eq!(state.familiar.type_id, PET_TYPE_NONE);
    assert!(!state.animal_companion.has_feat);
    assert!(!state.familiar.has_feat);

    let companions: Vec<&str> = state
        .animal_companion
        .options
        .iter()
        .map(|o| o.name.as_str())
        .collect();
    let familiars: Vec<&str> = state
        .familiar
        .options
        .iter()
        .map(|o| o.name.as_str())
        .collect();

    assert!(
        companions.contains(&"Wolf"),
        "expected a TLK-resolved Wolf companion, got {companions:?}"
    );
    assert!(
        familiars.contains(&"Bat"),
        "expected a TLK-resolved Bat familiar, got {familiars:?}"
    );

    // Padding rows carry no blueprint prefix and must not be selectable.
    assert!(!companions.contains(&"padding"));
    assert!(!familiars.contains(&"padding"));
}
