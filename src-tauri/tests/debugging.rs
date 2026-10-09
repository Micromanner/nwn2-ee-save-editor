//! Debugging / diagnostic test suite.
//!
//! These tests are diagnostics — most are `#[ignore]` and exist for inspecting
//! real save data. Run with:
//!   cargo test --test debugging -- --ignored --nocapture
//!
//! Add new diagnostics by creating `tests/debugging/<name>.rs` and adding a
//! `mod <name>;` line below.

#[path = "debugging/diagnostic_list_types.rs"]
mod diagnostic_list_types;

#[path = "debugging/diagnose_npc_heads.rs"]
mod diagnose_npc_heads;

#[path = "debugging/dump_armor_meshes.rs"]
mod dump_armor_meshes;

#[path = "debugging/dump_armor_mdb_materials.rs"]
mod dump_armor_mdb_materials;

#[path = "debugging/dump_full_armor_item.rs"]
mod dump_full_armor_item;

#[path = "debugging/dump_skeleton_bones.rs"]
mod dump_skeleton_bones;

#[path = "debugging/diagnose_item_models.rs"]
mod diagnose_item_models;

#[path = "debugging/diagnose_model_scale.rs"]
mod diagnose_model_scale;

#[path = "debugging/armor_debug_dump.rs"]
mod armor_debug_dump;

#[path = "debugging/dump_gff_reference.rs"]
mod dump_gff_reference;

#[path = "debugging/dump_ros_reference.rs"]
mod dump_ros_reference;

#[path = "debugging/diff_class_edits.rs"]
mod diff_class_edits;

#[path = "debugging/investigate_saves.rs"]
mod investigate_saves;

#[path = "debugging/diagnose_linux_user.rs"]
mod diagnose_linux_user;

#[path = "debugging/diagnose_hair_tint_roundtrip.rs"]
mod diagnose_hair_tint_roundtrip;

#[path = "debugging/diagnose_head_hair_mask_format.rs"]
mod diagnose_head_hair_mask_format;

#[path = "debugging/make_tint_sentinel_save.rs"]
mod make_tint_sentinel_save;

#[path = "debugging/dump_roster_flags.rs"]
mod dump_roster_flags;

#[path = "debugging/diagnose_pending_spells.rs"]
mod diagnose_pending_spells;

#[path = "debugging/dump_level_skill_history.rs"]
mod dump_level_skill_history;

#[path = "debugging/verify_skill_take_back.rs"]
mod verify_skill_take_back;

#[path = "debugging/dump_quest_decodes.rs"]
mod dump_quest_decodes;

#[path = "debugging/compare_level_up.rs"]
mod compare_level_up;

#[path = "debugging/prepare_spell_test_saves.rs"]
mod prepare_spell_test_saves;

#[path = "debugging/dump_all_2das.rs"]
mod dump_all_2das;

#[path = "debugging/prepare_quest_review_saves.rs"]
mod prepare_quest_review_saves;

#[path = "debugging/find_quest_lines.rs"]
mod find_quest_lines;

#[path = "debugging/check_area_objects.rs"]
mod check_area_objects;

#[path = "debugging/dump_lead_in.rs"]
mod dump_lead_in;

#[path = "debugging/legacy_bytes_save.rs"]
mod legacy_bytes_save;

#[path = "debugging/prepare_alignment_saves.rs"]
mod prepare_alignment_saves;

#[path = "debugging/prepare_playerinfo_saves.rs"]
mod prepare_playerinfo_saves;

#[path = "debugging/scan_zero_tint_accessories.rs"]
mod scan_zero_tint_accessories;
