use app_lib::config::NWN2Paths;
use app_lib::parsers::tlk::TLKParser;
use app_lib::services::campaign::content::extract_module_info;
use app_lib::services::save_graph::{self, BuildContext};
use app_lib::services::savegame_handler::SaveGameHandler;
use app_lib::services::toolset_bridge::BridgeClient;
use tempfile::TempDir;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

const SCRIPTS: &[&str] = &[
    "ga_influence",
    "ka_influence_magnitude",
    "ka_influence_set",
    "ga_roster_party_add",
    "ga_party_add",
    "ga_roster_party_remove",
    "ga_roster_selectable",
];

#[test]
#[ignore = "temporary"]
fn find_quest_lines() {
    let save = std::env::var("QUEST_DECODE_SAVE").unwrap();
    let scripts: Vec<String> = std::env::var("QUEST_SCRIPTS")
        .map(|v| v.split(',').map(|s| s.to_ascii_lowercase()).collect())
        .unwrap_or_else(|_| SCRIPTS.iter().map(|s| s.to_string()).collect());
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
        player_index: 0,
        current_module: &module_info,
        current_module_vars: &module_vars,
        progress: None,
    })
    .unwrap();
    let mut tlk = TLKParser::default();
    tlk.parse_from_file(install.join("dialog.tlk")).unwrap();
    let text = |strref: Option<i32>| {
        strref
            .and_then(|s| tlk.read_string(s as usize))
            .unwrap_or_default()
            .chars()
            .take(90)
            .collect::<String>()
    };
    for quest in &graph.quests {
        for tr in quest.transitions.iter().filter(|t| !t.debug) {
            let hits: Vec<String> = tr
                .actions
                .iter()
                .map(|a| (a, "line"))
                .chain(
                    tr.follow_up
                        .iter()
                        .flat_map(|f| f.actions.iter().map(|a| (a, "later"))),
                )
                .filter(|(a, _)| scripts.contains(&a.script.to_ascii_lowercase()))
                .map(|(a, at)| format!("{at}: {}({:?})", a.script, a.params))
                .collect();
            if hits.is_empty() {
                continue;
            }
            let spoken = tr.spoken.as_ref().map(|s| s.text_strref);
            println!(
                "[{}] {} live {:?} -> {} ({}) | {} {} node {} | \"{}\"\n  {}",
                quest.category.name,
                quest.tag,
                quest.live_state,
                tr.new_state,
                quest
                    .category
                    .entries
                    .iter()
                    .find(|e| e.id as u32 == tr.new_state)
                    .map(|e| e.text.chars().take(60).collect::<String>())
                    .unwrap_or_default(),
                tr.module,
                tr.dlg,
                tr.node,
                text(spoken.or(tr.text_strref)),
                hits.join("\n  ")
            );
        }
    }
}
