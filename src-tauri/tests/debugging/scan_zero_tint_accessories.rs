use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::services::resource_manager::ResourceManager;
use indexmap::IndexMap;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::RwLock;

type Map = IndexMap<String, GffValue<'static>>;

const SLOTS: [&str; 22] = [
    "ACLtShoulder",
    "ACRtShoulder",
    "ACLtBracer",
    "ACRtBracer",
    "ACLtElbow",
    "ACRtElbow",
    "ACLtArm",
    "ACRtArm",
    "ACLtHip",
    "ACRtHip",
    "ACFtHip",
    "ACBkHip",
    "ACLtLeg",
    "ACRtLeg",
    "ACLtShin",
    "ACRtShin",
    "ACLtKnee",
    "ACRtKnee",
    "ACLtFoot",
    "ACRtFoot",
    "ACLtAnkle",
    "ACRtAnkle",
];

fn num(v: &GffValue) -> i64 {
    match v {
        GffValue::Byte(x) => *x as i64,
        GffValue::Word(x) => *x as i64,
        GffValue::Dword(x) => *x as i64,
        GffValue::Int(x) => *x as i64,
        GffValue::Short(x) => *x as i64,
        _ => 0,
    }
}

fn sub<'a>(m: &'a Map, k: &str) -> Option<&'a Map> {
    match m.get(k) {
        Some(GffValue::StructOwned(s)) => Some(s),
        _ => None,
    }
}

fn tint(m: &Map) -> Option<[i64; 12]> {
    let t = sub(sub(m, "Tintable")?, "Tint")?;
    let mut out = [0i64; 12];
    for (i, ch) in ["1", "2", "3"].iter().enumerate() {
        let c = sub(t, ch)?;
        for (j, k) in ["r", "g", "b", "a"].iter().enumerate() {
            out[i * 4 + j] = c.get(*k).map(num).unwrap_or(0);
        }
    }
    Some(out)
}

fn load(bytes: Vec<u8>) -> Option<Map> {
    let gff = GffParser::from_bytes(bytes).ok()?;
    let f = gff.read_struct_fields(0).ok()?;
    Some(
        f.into_iter()
            .map(|(k, v)| (k, v.force_owned().expect("force owned")))
            .collect(),
    )
}

#[derive(Default)]
struct Stats {
    items: usize,
    slots_acc: usize,
    zero: usize,
    nonzero: usize,
    equal_chest: usize,
    missing_tint: usize,
    rgb_zero: usize,
}

struct Hit {
    src: String,
    tag: String,
    name_ref: i32,
    name: String,
    chest_tint: Option<[i64; 12]>,
    zero_slots: Vec<String>,
    nonzero_slots: usize,
}

fn visit(m: &Map, src: &str, stats: &mut Stats, hits: &mut Vec<Hit>) {
    if SLOTS.iter().any(|k| m.contains_key(*k)) {
        let chest = tint(m);
        let mut zs = vec![];
        let mut nz = 0;
        let mut any = false;
        for slot in SLOTS {
            let Some(sm) = sub(m, slot) else { continue };
            let acc = sm.get("Accessory").map(num).unwrap_or(0);
            if acc <= 0 {
                continue;
            }
            any = true;
            stats.slots_acc += 1;
            match tint(sm) {
                None => stats.missing_tint += 1,
                Some(t) if t.iter().all(|x| *x == 0) => {
                    println!("ZERO {src}");
                    stats.zero += 1;
                    zs.push(format!("{slot}={acc}"));
                }
                Some(t) => {
                    if t.iter().enumerate().all(|(i, x)| i % 4 == 3 || *x == 0) {
                        stats.rgb_zero += 1;
                    }
                    stats.nonzero += 1;
                    nz += 1;
                    if Some(t) == chest {
                        stats.equal_chest += 1;
                    }
                }
            }
        }
        if any {
            stats.items += 1;
        }
        if !zs.is_empty() {
            let (name_ref, name) = match m.get("LocalizedName") {
                Some(GffValue::LocString(ls)) => (
                    ls.string_ref,
                    ls.substrings
                        .first()
                        .map(|x| x.string.to_string())
                        .unwrap_or_default(),
                ),
                _ => (-1, String::new()),
            };
            let resref = match m.get("TemplateResRef") {
                Some(GffValue::ResRef(r)) => r.to_string(),
                _ => String::new(),
            };
            let tag = match m.get("Tag") {
                Some(GffValue::String(r)) => r.to_string(),
                _ => String::new(),
            };
            hits.push(Hit {
                src: src.to_string(),
                tag: format!("{resref}|{tag}"),
                name_ref,
                name,
                chest_tint: chest,
                zero_slots: zs,
                nonzero_slots: nz,
            });
        }
    }
    for v in m.values() {
        match v {
            GffValue::StructOwned(s) => visit(s, src, stats, hits),
            GffValue::ListOwned(l) => {
                for e in l {
                    visit(e, src, stats, hits);
                }
            }
            _ => {}
        }
    }
}

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "backups") {
                continue;
            }
            walk(&p, out);
        } else if let Some(ext) = p.extension().and_then(|x| x.to_str())
            && ["bic", "ros", "ifo", "uti", "zip"].contains(&ext.to_lowercase().as_str())
        {
            out.push(p);
        }
    }
}

#[tokio::test]
#[ignore]
async fn scan_zero_tint_accessories() {
    let mut bp = Stats::default();
    let mut bp_hits = vec![];
    let data = std::path::Path::new(
        r"C:\Program Files (x86)\Steam\steamapps\common\NWN2 Enhanced Edition\data",
    );
    for z in ["templates.zip", "templates_x1.zip", "templates_x2.zip"] {
        let f = std::fs::File::open(data.join(z)).unwrap();
        let mut ar = zip::ZipArchive::new(f).unwrap();
        let mut n = 0;
        for i in 0..ar.len() {
            let mut e = ar.by_index(i).unwrap();
            if !e.name().to_lowercase().ends_with(".uti") {
                continue;
            }
            let name = e.name().to_string();
            let mut b = vec![];
            e.read_to_end(&mut b).unwrap();
            n += 1;
            if let Some(m) = load(b) {
                visit(&m, &name, &mut bp, &mut bp_hits);
            }
        }
        println!("{z}: {n} uti");
    }

    let mut sv = Stats::default();
    let mut sv_hits = vec![];
    let mut files = vec![];
    walk(std::path::Path::new("tests/fixtures/saves"), &mut files);
    for p in &files {
        if p.extension().is_some_and(|e| e == "zip") {
            if p.file_name().is_some_and(|n| n != "resgff.zip") {
                continue;
            }
            let mut ar = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
            for i in 0..ar.len() {
                let mut e = ar.by_index(i).unwrap();
                let name = e.name().to_lowercase();
                if !(name.ends_with(".bic") || name.ends_with(".ros") || name.ends_with(".ifo")) {
                    continue;
                }
                let label = format!("{}!{}", p.to_string_lossy(), e.name());
                let mut b = vec![];
                e.read_to_end(&mut b).unwrap();
                if let Some(m) = load(b) {
                    visit(&m, &label, &mut sv, &mut sv_hits);
                }
            }
        } else if let Ok(b) = std::fs::read(p)
            && let Some(m) = load(b)
        {
            visit(&m, &p.to_string_lossy(), &mut sv, &mut sv_hits);
        }
    }
    println!("save files scanned: {}", files.len());

    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    rm.write().await.initialize().await.ok();
    let rm = rm.read().await;

    for (label, st, hits) in [("BLUEPRINTS", &bp, &bp_hits), ("SAVES", &sv, &sv_hits)] {
        println!(
            "\n=== {label}: items_with_acc={} acc_slots={} zero={} nonzero={} nonzero_equal_chest={} no_tint_struct={} rgb_zero_alpha_nonzero={}",
            st.items,
            st.slots_acc,
            st.zero,
            st.nonzero,
            st.equal_chest,
            st.missing_tint,
            st.rgb_zero
        );
        println!("items with >=1 zero-tint acc slot: {}", hits.len());
        for h in hits {
            let name = if h.name_ref >= 0 && h.name.is_empty() {
                rm.get_string(h.name_ref)
            } else {
                h.name.clone()
            };
            let chest_nz = h.chest_tint.is_some_and(|t| t.iter().any(|x| *x != 0));
            println!(
                "{} | {} | '{}' | chest_nonzero={} chest={:?} | zero_slots={:?} nonzero_slots={}",
                h.src, h.tag, name, chest_nz, h.chest_tint, h.zero_slots, h.nonzero_slots
            );
        }
    }
}
