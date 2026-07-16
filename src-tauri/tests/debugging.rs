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
