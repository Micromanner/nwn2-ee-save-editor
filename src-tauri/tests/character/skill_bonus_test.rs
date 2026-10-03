#[cfg(test)]
mod tests {
    use app_lib::character::types::SkillId;
    use app_lib::character::{Character, EquipmentSlot};
    use app_lib::parsers::gff::GffValue;
    use app_lib::parsers::tlk::TLKParser;
    use app_lib::services::item_property_decoder::ItemPropertyDecoder;
    use indexmap::IndexMap;
    use std::borrow::Cow;
    use std::sync::Arc;

    fn setup_test_environment() -> (Character, ItemPropertyDecoder) {
        let decoder = ItemPropertyDecoder::default();

        let fields = IndexMap::new();
        let mut char = Character::from_gff(fields);

        // Add a skill (Concentration = ID 0) with rank 5
        char.set_skill_rank(SkillId(0), 5).unwrap();

        (char, decoder)
    }

    #[tokio::test]
    async fn test_equipment_skill_bonus_application() {
        let (mut char, decoder) = setup_test_environment();

        // 1. Create an item with Skill Bonus (Property 52), Subtype 0 (Concentration), Cost 5 (Value +5)
        let mut item = IndexMap::new();
        item.insert("BaseItem".to_string(), GffValue::Int(1)); // Arbitrary base item
        item.insert(
            "Tag".to_string(),
            GffValue::String(Cow::Owned("TestHelmet".to_string())),
        );
        item.insert("Identified".to_string(), GffValue::Byte(1));

        let mut prop = IndexMap::new();
        prop.insert("PropertyName".to_string(), GffValue::Word(52)); // Skill Bonus
        prop.insert("Subtype".to_string(), GffValue::Word(0)); // Concentration
        prop.insert("CostValue".to_string(), GffValue::Word(5)); // +5 Bonus
        prop.insert("ChanceAppear".to_string(), GffValue::Byte(100));

        item.insert(
            "PropertiesList".to_string(),
            GffValue::ListOwned(vec![prop]),
        );

        // 2. Equip the item (manually constructing Equip_ItemList to skip equip_item validation)
        let mut equip_item = item.clone();
        equip_item.insert(
            "__struct_id__".to_string(),
            GffValue::Dword(EquipmentSlot::Head.to_bitmask()),
        );

        char.set_list("Equip_ItemList", vec![equip_item]);

        let tlk = Arc::new(std::sync::RwLock::new(TLKParser::default()));
        let mut game_data = app_lib::loaders::GameData::new(tlk);
        let mut skills = app_lib::parsers::tda::TDAParser::new();
        skills
            .parse_from_bytes(
                b"2DA V2.0

   Label          KeyAbility
0  Concentration  STR
",
            )
            .unwrap();
        game_data.tables.insert(
            "skills".to_string(),
            app_lib::loaders::LoadedTable::new("skills.2da".to_string(), Arc::new(skills)),
        );
        char.set_ability(app_lib::character::types::AbilityIndex::STR, 10)
            .unwrap();

        // 4. Verify Total
        // Expected: Rank 5 + Ability 0 + Bonus 5 = 10.
        // Current Bug: Rank 5 + Ability 0 + Bonus 0 (ignored) = 5.

        let total = char.get_skill_summary(&game_data, Some(&decoder))[0].total;
        println!("Skill Total: {total}");

        assert_eq!(
            total, 10,
            "Skill bonus from equipment was not applied! Expected 10, got {total}"
        );
    }
}
