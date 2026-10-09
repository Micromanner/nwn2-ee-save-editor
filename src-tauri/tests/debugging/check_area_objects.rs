use app_lib::config::NWN2Paths;
use app_lib::services::campaign::content;
use app_lib::services::save_graph::area_objects::{self, AreaField};
use app_lib::services::savegame_handler::SaveGameHandler;

#[test]
#[ignore = "temporary"]
fn check_area_objects() {
    let save = std::env::var("AREA_SAVE").unwrap();
    let module = "1000_Neverwinter_A1";
    let paths = NWN2Paths::new();
    let objects = area_objects::installed_module_objects(&paths, module).unwrap();
    let door = objects
        .unique(AreaField::DoorLocked, "10_hagen_door_ext")
        .unwrap();
    let pin = objects
        .unique(AreaField::MapNoteEnabled, "1000_mn_hagen")
        .unwrap();
    let handler = SaveGameHandler::new(&save).unwrap();
    let (_, mut erf) = content::open_module_state(&handler, module).unwrap();
    println!(
        "before: door locked {:?}, pin {:?}, labels {:?} {:?}",
        area_objects::read_saved(&mut erf, AreaField::DoorLocked, "10_hagen_door_ext", door),
        area_objects::read_saved(&mut erf, AreaField::MapNoteEnabled, "1000_mn_hagen", pin),
        door.label_strref,
        pin.label_strref
    );
}

#[test]
#[ignore = "temporary"]
fn find_companion_area_copies() {
    let save = std::env::var("AREA_SAVE").unwrap();
    let tag = std::env::var("AREA_TAG").unwrap_or_else(|_| "khelgar".into());
    let handler = SaveGameHandler::new(&save).unwrap();
    for module in content::module_state_ids(handler.save_dir()) {
        let Ok((_, mut erf)) = content::open_module_state(&handler, &module) else {
            println!("{module}: unreadable");
            continue;
        };
        for (area, _, _) in erf.list_resources(Some(app_lib::parsers::erf::RES_TYPE_GIT)) {
            let bytes = erf.extract_resource(&area).unwrap();
            let gff = app_lib::parsers::gff::GffParser::from_bytes(bytes).unwrap();
            let root = gff.read_struct_fields(0).unwrap();
            let Some(list) = root.get("Creature List") else {
                continue;
            };
            let list = list.clone().force_owned().unwrap();
            if let app_lib::parsers::gff::GffValue::ListOwned(creatures) = list {
                for c in creatures {
                    if app_lib::parsers::gff::gff_text(&c, "Tag")
                        .is_some_and(|t| t.eq_ignore_ascii_case(&tag))
                    {
                        println!("{module} {area}: {tag} copy");
                    }
                }
            }
        }
    }
}
