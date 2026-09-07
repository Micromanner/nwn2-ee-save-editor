use app_lib::parsers::gr2::Gr2Parser;
use app_lib::services::resource_manager::ResourceManager;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
#[ignore = "diagnostic — run with cargo test dump_skeleton_bones -- --ignored --nocapture"]
async fn dump_skeleton_bones() {
    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut rm_guard = rm.write().await;
        let _ = rm_guard.initialize().await;
    }
    let rm = rm.read().await;

    let mut out = String::new();

    for skel_name in ["P_HHM_skel", "P_EEM_skel"] {
        writeln!(out, "\n=== {skel_name} ===").unwrap();
        match rm.get_resource_bytes(skel_name, "gr2") {
            Ok(bytes) => match Gr2Parser::parse(&bytes) {
                Ok(skel) => {
                    writeln!(out, "  name: '{}'", skel.name).unwrap();
                    writeln!(out, "  bone count: {}", skel.bones.len()).unwrap();
                    writeln!(out, "  all bones (idx parent_idx name local_position):").unwrap();
                    for (i, b) in skel.bones.iter().enumerate() {
                        let p = b.transform.position;
                        writeln!(
                            out,
                            "    {i:3}  {:3}  {:<28} [{:8.3} {:8.3} {:8.3}]",
                            b.parent_index, b.name, p[0], p[1], p[2]
                        )
                        .unwrap();
                    }
                    writeln!(out, "\n  bones matching accessory slots:").unwrap();
                    let slot_patterns = [
                        "shoulder",
                        "upper_arm",
                        "uparm",
                        "up_arm",
                        "elbow",
                        "bracer",
                        "forearm",
                        "lower_arm",
                        "hip",
                        "up_leg",
                        "uleg",
                        "upper_leg",
                        "thigh",
                        "knee",
                        "shin",
                        "lower_leg",
                        "lleg",
                        "low_leg",
                        "calf",
                        "r_",
                        "l_",
                        "ap_",
                    ];
                    for b in &skel.bones {
                        let lower = b.name.to_lowercase();
                        if slot_patterns.iter().any(|p| lower.contains(p)) {
                            writeln!(out, "    - {}", b.name).unwrap();
                        }
                    }
                }
                Err(e) => writeln!(out, "  PARSE FAILED: {e}").unwrap(),
            },
            Err(e) => writeln!(out, "  NOT FOUND: {e}").unwrap(),
        }
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("skeleton_bones.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("Output: {}", out_path.display());
    print!("{out}");
}
