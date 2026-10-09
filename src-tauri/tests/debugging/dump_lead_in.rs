use app_lib::config::NWN2Paths;
use app_lib::services::campaign::content::extract_module_info;
use app_lib::services::save_graph::editing::{
    LiveCharacters, PreviewSave, QuestEdits, RewardTables,
};
use app_lib::services::save_graph::{self, BuildContext};
use app_lib::services::savegame_handler::SaveGameHandler;
use app_lib::services::toolset_bridge::BridgeClient;
use tempfile::TempDir;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

#[test]
#[ignore = "temporary"]
fn dump_lead_in() {
    let save = std::env::var("QUEST_DECODE_SAVE").unwrap();
    let tag = std::env::var("QUEST_TAG").unwrap();
    let state: u32 = std::env::var("QUEST_STATE").unwrap().parse().unwrap();
    let player: usize = std::env::var("QUEST_PLAYER").map_or(0, |p| p.parse().unwrap());
    let bridge = common::toolset_bridge_exe();
    let install = common::nwn2_install_path();
    let handler = SaveGameHandler::new(std::path::Path::new(&save)).unwrap();
    let mut paths = NWN2Paths::new();
    paths.set_game_folder(&install).unwrap();
    let (module_info, module_vars) = extract_module_info(&handler, &paths).unwrap();
    let cache_dir = TempDir::new().unwrap();
    let client =
        BridgeClient::new(bridge, install.clone(), cache_dir.path().to_path_buf()).unwrap();
    let graph = save_graph::build(BuildContext {
        handler: &handler,
        paths: &paths,
        client: &client,
        player_index: player,
        current_module: &module_info,
        current_module_vars: &module_vars,
        progress: None,
    })
    .unwrap();
    let script = std::env::var("QUEST_SCRIPT").ok();
    let tables = RewardTables::from_graph(&graph, None);
    for quest in &graph.quests {
        if script.is_none() && quest.tag != tag {
            continue;
        }
        for (route, tr) in quest.transitions.iter().enumerate() {
            let runs = |s: &str| {
                tr.actions
                    .iter()
                    .chain(tr.lead_in.iter().flat_map(|n| n.actions.iter()))
                    .chain(tr.follow_up.iter().flat_map(|n| n.actions.iter()))
                    .any(|a| a.script.eq_ignore_ascii_case(s))
            };
            match &script {
                Some(s) if !runs(s) => continue,
                None if tr.new_state != state => continue,
                _ => {}
            }
            let (tag, state) = (quest.tag.clone(), tr.new_state);
            let preview = QuestEdits::default()
                .preview(
                    PreviewSave {
                        graph: &graph,
                        tables: &tables,
                        handler: &handler,
                        paths: &paths,
                        player,
                        characters: LiveCharacters::default(),
                    },
                    &tag,
                    state,
                    Some(route),
                )
                .unwrap();
            println!(
                "== {tag} -> {state} {} {} route {route} node {} lead_in {:?}",
                tr.module,
                tr.dlg,
                tr.node,
                preview.lead_in.iter().map(|l| l.node).collect::<Vec<_>>()
            );
            for w in &preview.writes {
                println!(
                    "   write {}:{} {:?} -> {} line {:?}",
                    w.key.value_type, w.key.name, w.before, w.after, w.line
                );
            }
            for e in &preview.effects {
                println!(
                    "   effect {} {}({:?}) line {:?}",
                    e.reason, e.action.script, e.action.params, e.line
                );
                if let Some(d) = &e.action.decoded {
                    println!("      decoded {d:?}");
                }
            }
        }
    }
}

#[test]
#[ignore = "temporary"]
fn dump_player_entries() {
    use app_lib::character::Character;
    use app_lib::parsers::gff::{GffParser, GffValue};
    let save = std::env::var("QUEST_DECODE_SAVE").unwrap();
    let handler = SaveGameHandler::new(std::path::Path::new(&save)).unwrap();
    let gff = GffParser::from_bytes(handler.extract_player_data().unwrap()).unwrap();
    let root = gff.read_struct_fields(0).unwrap();
    let Some(GffValue::List(players)) = root.get("Mod_PlayerList") else {
        panic!()
    };
    for (i, p) in players.iter().enumerate() {
        let fields = p.force_load().unwrap();
        let c = Character::from_gff(
            fields
                .into_iter()
                .map(|(k, v)| (k, v.into_owned()))
                .collect(),
        )
        .unwrap();
        println!(
            "entry {i}: {:?} xp {:?}",
            c.full_name(),
            c.get_i32("Experience")
        );
    }
}

#[test]
#[ignore = "temporary"]
fn dump_player_bic() {
    use app_lib::character::Character;
    use app_lib::parsers::gff::GffParser;
    let save = std::env::var("QUEST_DECODE_SAVE").unwrap();
    let bytes = std::fs::read(std::path::Path::new(&save).join("player.bic")).unwrap();
    let gff = GffParser::from_bytes(bytes).unwrap();
    let fields = gff
        .read_struct_fields(0)
        .unwrap()
        .into_iter()
        .map(|(k, v)| (k, v.into_owned()))
        .collect();
    let c = Character::from_gff(fields).unwrap();
    println!("bic: {:?} xp {:?}", c.full_name(), c.get_i32("Experience"));
}
