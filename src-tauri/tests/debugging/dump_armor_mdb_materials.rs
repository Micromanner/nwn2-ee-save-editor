use app_lib::parsers::mdb::parser::MdbParser;
use app_lib::parsers::mdb::types::{Material, material_flags};
use app_lib::services::resource_manager::ResourceManager;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::io::Read as _;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

fn flag_names(flags: u32) -> String {
    let names = [
        (material_flags::ALPHA_TEST, "ALPHA_TEST"),
        (material_flags::ALPHA_BLEND, "ALPHA_BLEND"),
        (material_flags::ADDITIVE_BLEND, "ADDITIVE_BLEND"),
        (material_flags::ENVIRONMENT_MAPPING, "ENV_MAP"),
        (material_flags::CUTSCENE_MESH, "CUTSCENE"),
        (material_flags::GLOW, "GLOW"),
        (material_flags::CAST_NO_SHADOWS, "NO_SHADOWS"),
        (material_flags::PROJECTED_TEXTURES, "PROJ_TEX"),
    ];
    let set: Vec<&str> = names
        .iter()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|(_, n)| *n)
        .collect();
    set.join("|")
}

fn material_desc(m: &Material) -> String {
    format!(
        "flags=0x{:02X}[{}] diff_color=[{:.3},{:.3},{:.3}] spec_color=[{:.3},{:.3},{:.3}] spec_level={:.3} spec_power={:.3}",
        m.flags,
        flag_names(m.flags),
        m.diffuse_color[0],
        m.diffuse_color[1],
        m.diffuse_color[2],
        m.specular_color[0],
        m.specular_color[1],
        m.specular_color[2],
        m.specular_level,
        m.specular_power,
    )
}

#[tokio::test]
#[ignore = "diagnostic — run manually with cargo test dump_armor_mdb_materials -- --ignored --nocapture"]
async fn dump_armor_mdb_materials() {
    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut rm_guard = rm.write().await;
        let _ = rm_guard.initialize().await;
    }
    let rm = rm.read().await;

    let mut out = String::new();

    let bodies = [
        // Expected accessory resrefs for darksteel plate UTI
        // (ACLtShoulder=15, ACRtShoulder=15, ACLtArm=14, ACRtArm=12,
        //  ACLtBracer=17, ACRtBracer=17, ACLtShin=11, ACRtShin=11,
        //  ACLtLeg=14, ACRtKnee=11)
        "A_EEM_LShoulder15",
        "A_EEM_RShoulder15",
        "A_EEM_LUpArm14",
        "A_EEM_RUpArm12",
        "A_EEM_LBracer17",
        "A_EEM_RBracer17",
        "A_EEM_LLowLeg11",
        "A_EEM_RLowLeg11",
        "A_EEM_LUpLeg14",
        "A_EEM_RKnee11",
        // Sanity: body mesh for comparison
        "P_EEM_PF_Body03",
    ];

    for name in bodies {
        writeln!(out, "\n=== {name} ===").unwrap();
        let lower = name.to_lowercase();
        match rm.get_resource_bytes(&lower, "mdb") {
            Ok(bytes) => match MdbParser::parse(&bytes) {
                Ok(mdb) => {
                    writeln!(
                        out,
                        "  version: {}.{}, packets: {} (rigid={}, skin={}, hook={}, helm={}, hair={})",
                        mdb.header.major_version,
                        mdb.header.minor_version,
                        mdb.header.packet_count,
                        mdb.rigid_meshes.len(),
                        mdb.skin_meshes.len(),
                        mdb.hooks.len(),
                        mdb.helm.len(),
                        mdb.hair.len(),
                    )
                    .unwrap();
                    for m in &mdb.skin_meshes {
                        writeln!(
                            out,
                            "  SKIN name='{}' skel='{}' diff='{}' norm='{}' tint='{}' glow='{}' verts={} faces={} {}",
                            m.name,
                            m.skeleton_name,
                            m.material.diffuse_map_name,
                            m.material.normal_map_name,
                            m.material.tint_map_name,
                            m.material.glow_map_name,
                            m.vertices.len(),
                            m.faces.len(),
                            material_desc(&m.material),
                        )
                        .unwrap();
                    }
                    for m in &mdb.rigid_meshes {
                        writeln!(
                            out,
                            "  RIGD name='{}' diff='{}' norm='{}' tint='{}' glow='{}' verts={} faces={} {}",
                            m.name,
                            m.material.diffuse_map_name,
                            m.material.normal_map_name,
                            m.material.tint_map_name,
                            m.material.glow_map_name,
                            m.vertices.len(),
                            m.faces.len(),
                            material_desc(&m.material),
                        )
                        .unwrap();
                    }
                }
                Err(e) => writeln!(out, "  PARSE FAILED: {e}").unwrap(),
            },
            Err(e) => writeln!(out, "  NOT FOUND: {e}").unwrap(),
        }
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("armor_mdb_materials.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("Output: {}", out_path.display());
    print!("{out}");
}

#[test]
#[ignore = "diagnostic — run manually with cargo test survey_mdb_specular -- --ignored --nocapture"]
fn survey_mdb_specular() {
    let game_dir = std::env::var("NWN2_GAME_FOLDER")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\NWN2 Enhanced Edition")
        });
    let prefixes: Vec<String> = std::env::var("MDB_SURVEY_PREFIXES")
        .unwrap_or_else(|_| "p_,a_".to_string())
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect();

    let mut csv = String::from(
        "file,zip,mesh_kind,mesh,flags_hex,flag_names,sc_r,sc_g,sc_b,spec_level,spec_power,dc_r,dc_g,dc_b\n",
    );
    let mut combo_hist: BTreeMap<String, u32> = BTreeMap::new();
    let mut env_flagged: Vec<String> = Vec::new();
    let mut total = 0u32;
    let mut parse_failures = 0u32;

    let data_dir = game_dir.join("Data");
    let entries = std::fs::read_dir(&data_dir).expect("game Data dir not found");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        let zip_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let Ok(file) = std::fs::File::open(&path) else {
            continue;
        };
        let Ok(mut archive) = zip::ZipArchive::new(file) else {
            continue;
        };
        for i in 0..archive.len() {
            let (base, bytes) = {
                let Ok(mut e) = archive.by_index(i) else {
                    continue;
                };
                let n = e.name().to_lowercase();
                if !n.ends_with(".mdb") {
                    continue;
                }
                let base = n.rsplit('/').next().unwrap_or(&n).to_string();
                if !prefixes.iter().any(|p| base.starts_with(p.as_str())) {
                    continue;
                }
                let mut buf = Vec::new();
                if e.read_to_end(&mut buf).is_err() {
                    continue;
                }
                (base, buf)
            };
            let Ok(mdb) = MdbParser::parse(&bytes) else {
                parse_failures += 1;
                continue;
            };
            let mut record = |kind: &str, mesh: &str, m: &Material| {
                total += 1;
                if m.flags & material_flags::ENVIRONMENT_MAPPING != 0 {
                    env_flagged.push(format!("{base} / {mesh}"));
                }
                let key = format!(
                    "level={:.2} power={:.2} spec_color=[{:.2},{:.2},{:.2}]",
                    m.specular_level,
                    m.specular_power,
                    m.specular_color[0],
                    m.specular_color[1],
                    m.specular_color[2],
                );
                *combo_hist.entry(key).or_insert(0) += 1;
                csv.push_str(&format!(
                    "{},{},{},{},0x{:02X},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}\n",
                    base,
                    zip_name,
                    kind,
                    mesh,
                    m.flags,
                    flag_names(m.flags),
                    m.specular_color[0],
                    m.specular_color[1],
                    m.specular_color[2],
                    m.specular_level,
                    m.specular_power,
                    m.diffuse_color[0],
                    m.diffuse_color[1],
                    m.diffuse_color[2],
                ));
            };
            for m in &mdb.rigid_meshes {
                record("RIGD", &m.name, &m.material);
            }
            for m in &mdb.skin_meshes {
                record("SKIN", &m.name, &m.material);
            }
        }
    }

    let mut hist: Vec<(String, u32)> = combo_hist.into_iter().collect();
    hist.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("Total mesh materials: {total} (parse failures: {parse_failures})");
    eprintln!(
        "ENVIRONMENT_MAPPING (0x08) set on {} materials:",
        env_flagged.len()
    );
    for name in env_flagged.iter().take(40) {
        eprintln!("  {name}");
    }
    eprintln!("Top (spec_level, spec_power, spec_color) combos:");
    for (k, c) in hist.iter().take(30) {
        eprintln!("  {c:>6}  {k}");
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("mdb_specular_survey.csv");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &csv).unwrap();
    eprintln!("CSV: {}", out_path.display());
}
