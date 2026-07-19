//! Diagnostic: determine the DDS *format* and channel coverage of the head +
//! hair tint masks for the Painted Elf (Head 1, Hairstyle 14) repro.
//!
//! Purpose: the frontend uses three.js DDSLoader, which does NOT support BC7
//! (DX10, dxgi 98/99) and returns an empty texture for it, while the backend
//! (texture2ddecoder) decodes BC7 fine. If a mask is BC7, the editor's 3D
//! preview tints from a broken texture -> garbage regions (matches the
//! "eyebrows dead / eyes bleeds into eyebrows" report). This test prints the
//! exact format so we can confirm/deny that per part.
//!
//! Run: cargo test --test debugging diagnose_head_hair_mask_format -- --ignored --nocapture

use app_lib::parsers::mdb::parser::MdbParser;
use app_lib::parsers::tda::types::TDAParser;
use app_lib::services::resource_manager::ResourceManager;
use std::fmt::Write as _;
use std::sync::Arc;
use tokio::sync::RwLock;

const DDS_MAGIC: u32 = 0x2053_4444;
const DDPF_FOURCC: u32 = 0x4;

fn describe_dds_format(bytes: &[u8]) -> String {
    if bytes.len() < 128 {
        return "TOO SMALL".into();
    }
    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    if magic != DDS_MAGIC {
        return "BAD MAGIC".into();
    }
    let height = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let width = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let pf_flags = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
    let fourcc = &bytes[84..88];
    let rgb_bit_count = u32::from_le_bytes(bytes[88..92].try_into().unwrap());
    let has_fourcc = pf_flags & DDPF_FOURCC != 0;

    if has_fourcc && fourcc == b"DX10" {
        let dxgi = if bytes.len() >= 132 {
            u32::from_le_bytes(bytes[128..132].try_into().unwrap())
        } else {
            0
        };
        let name = match dxgi {
            71 | 72 => "BC1",
            77 | 78 => "BC3",
            95 => "BC6H_SF16",
            96 => "BC6H_UF16",
            98 | 99 => "BC7  <-- three.js DDSLoader UNSUPPORTED",
            _ => "unknown DXGI",
        };
        format!("{width}x{height} DX10 dxgi={dxgi} ({name})")
    } else if has_fourcc {
        let cc = String::from_utf8_lossy(fourcc);
        let three_ok = matches!(&cc[..], "DXT1" | "DXT3" | "DXT5");
        format!(
            "{width}x{height} FourCC={cc} (three.js {})",
            if three_ok { "OK" } else { "UNSUPPORTED" }
        )
    } else {
        format!(
            "{width}x{height} UNCOMPRESSED rgbBitCount={rgb_bit_count} (three.js handles 24/32)"
        )
    }
}

/// Decode an uncompressed A8R8G8B8 DDS the same way three.js loadARGBMip does
/// (on-disk byte order B,G,R,A -> RGBA), so we see exactly what the frontend
/// samples for masks the backend BC decoder rejects.
fn decode_uncompressed_argb(bytes: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    if bytes.len() < 128 {
        return None;
    }
    let height = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;
    let width = u32::from_le_bytes(bytes[16..20].try_into().ok()?) as usize;
    let rgb_bit_count = u32::from_le_bytes(bytes[88..92].try_into().ok()?);
    if rgb_bit_count != 32 {
        return None;
    }
    let data = &bytes[128..];
    let n = width * height;
    if data.len() < n * 4 {
        return None;
    }
    let mut rgba = Vec::with_capacity(n * 4);
    for px in data[..n * 4].chunks_exact(4) {
        // A8R8G8B8 little-endian on disk = [B, G, R, A]
        rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
    }
    Some((width, height, rgba))
}

fn coverage_of(rgba: &[u8]) -> String {
    let px = rgba.len() / 4;
    if px == 0 {
        return "0 pixels".into();
    }
    let (mut r, mut g, mut b, mut a0) = (0u64, 0u64, 0u64, 0u64);
    for p in rgba.chunks_exact(4) {
        if p[0] > 10 {
            r += 1;
        }
        if p[1] > 10 {
            g += 1;
        }
        if p[2] > 10 {
            b += 1;
        }
        if p[3] == 0 {
            a0 += 1;
        }
    }
    let pc = px as f64;
    format!(
        "coverage R={:.1}% G={:.1}% B={:.1}% (alpha=0 {:.1}%)",
        r as f64 / pc * 100.0,
        g as f64 / pc * 100.0,
        b as f64 / pc * 100.0,
        a0 as f64 / pc * 100.0,
    )
}

fn mask_coverage(bytes: &[u8]) -> String {
    if let Some((_, _, rgba)) = decode_uncompressed_argb(bytes) {
        return format!("frontend(three.js ARGB) {}", coverage_of(&rgba));
    }
    match app_lib::services::texture_decode::decode_dds_rgba(bytes) {
        Ok(tex) => {
            let px = tex.rgba.len() / 4;
            if px == 0 {
                return "0 pixels".into();
            }
            let (mut r, mut g, mut b, mut a0) = (0u64, 0u64, 0u64, 0u64);
            for p in tex.rgba.chunks_exact(4) {
                if p[0] > 10 {
                    r += 1;
                }
                if p[1] > 10 {
                    g += 1;
                }
                if p[2] > 10 {
                    b += 1;
                }
                if p[3] == 0 {
                    a0 += 1;
                }
            }
            let pc = px as f64;
            format!(
                "backend-decoded coverage R={:.1}% G={:.1}% B={:.1}% (alpha=0 {:.1}%)",
                r as f64 / pc * 100.0,
                g as f64 / pc * 100.0,
                b as f64 / pc * 100.0,
                a0 as f64 / pc * 100.0,
            )
        }
        Err(e) => format!("backend decode ERROR: {e}"),
    }
}

fn tint_maps_of(rm: &ResourceManager, resref: &str) -> Vec<String> {
    let Ok(bytes) = rm.get_resource_bytes(resref, "mdb") else {
        return Vec::new();
    };
    let Ok(mdb) = MdbParser::parse(&bytes) else {
        return Vec::new();
    };
    let mut maps = Vec::new();
    for name in mdb
        .rigid_meshes
        .iter()
        .map(|m| m.material.tint_map_name.trim())
        .chain(
            mdb.skin_meshes
                .iter()
                .map(|m| m.material.tint_map_name.trim()),
        )
    {
        if !name.is_empty() && !maps.iter().any(|t: &String| t.eq_ignore_ascii_case(name)) {
            maps.push(name.to_string());
        }
    }
    maps
}

/// Build a minimal DXT5 DDS (128-byte header + one 16-byte BC3 block) whose
/// color endpoints are both `rgb565`, so every decoded pixel is that color.
fn synthetic_dxt5(rgb565: u16) -> Vec<u8> {
    let mut d = vec![0u8; 160]; // 128 header + 16 block + pad past 148 min
    d[0..4].copy_from_slice(b"DDS ");
    d[4..8].copy_from_slice(&124u32.to_le_bytes());
    d[12..16].copy_from_slice(&4u32.to_le_bytes()); // height
    d[16..20].copy_from_slice(&4u32.to_le_bytes()); // width
    d[80..84].copy_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
    d[84..88].copy_from_slice(b"DXT5");
    // BC3 block: 8 bytes alpha (a0=a1=255 -> opaque), then 8 bytes color.
    d[128] = 0xFF; // alpha0
    d[129] = 0xFF; // alpha1
    // alpha indices (bytes 130..136) = 0 -> all pick alpha0=255
    d[136..138].copy_from_slice(&rgb565.to_le_bytes()); // color0
    d[138..140].copy_from_slice(&rgb565.to_le_bytes()); // color1
    // color indices (140..144) = 0 -> all pick color0
    d
}

/// Decisive test: does the backend decoder preserve R/G/B order for DXT5?
/// A pure-red test (existing suite) cannot catch a G<->B swap; these can.
#[test]
fn backend_dxt5_channel_order() {
    let cases = [
        ("RED  (0xF800)", 0xF800u16, (248u8, 0u8, 0u8)),
        ("GREEN(0x07E0)", 0x07E0u16, (0u8, 252u8, 0u8)),
        ("BLUE (0x001F)", 0x001Fu16, (0u8, 0u8, 248u8)),
    ];
    for (name, c565, (er, eg, eb)) in cases {
        let dds = synthetic_dxt5(c565);
        let tex = app_lib::services::texture_decode::decode_dds_rgba(&dds).expect("decode");
        let (r, g, b) = (tex.rgba[0], tex.rgba[1], tex.rgba[2]);
        eprintln!("{name}: decoded RGB=({r},{g},{b}) expected~({er},{eg},{eb})");
        // Allow rgb565 rounding slop.
        assert!((r as i16 - er as i16).abs() < 12, "{name} R off");
        assert!((g as i16 - eg as i16).abs() < 12, "{name} G off");
        assert!((b as i16 - eb as i16).abs() < 12, "{name} B off");
    }
}

#[tokio::test]
#[ignore]
async fn diagnose_head_hair_mask_format() {
    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut g = rm.write().await;
        let _ = g.initialize().await;
    }
    let rm = rm.read().await;

    let mut out = String::new();

    // 1. Find "Painted" appearance rows to get head/hair prefixes.
    writeln!(out, "=== appearance.2da rows matching 'paint' ===").unwrap();
    let mut head_prefixes: Vec<String> = Vec::new();
    let mut hair_prefixes: Vec<String> = Vec::new();
    if let Ok(bytes) = rm.get_resource_bytes("appearance", "2da") {
        let mut p = TDAParser::new();
        if p.parse_from_bytes(&bytes).is_ok() {
            for i in 0..p.row_count() {
                let label = p
                    .get_cell_by_name(i, "label")
                    .ok()
                    .flatten()
                    .unwrap_or("")
                    .to_string();
                if label.to_lowercase().contains("paint") || label.to_lowercase().contains("elf") {
                    let head = p
                        .get_cell_by_name(i, "nwn2_model_head")
                        .ok()
                        .flatten()
                        .unwrap_or("");
                    let hair = p
                        .get_cell_by_name(i, "nwn2_model_hair")
                        .ok()
                        .flatten()
                        .unwrap_or("");
                    let body = p
                        .get_cell_by_name(i, "nwn2_model_body")
                        .ok()
                        .flatten()
                        .unwrap_or("");
                    writeln!(
                        out,
                        "  row {i}: label='{label}' head='{head}' hair='{hair}' body='{body}'"
                    )
                    .unwrap();
                    if !head.is_empty() {
                        head_prefixes.push(head.to_string());
                    }
                    if !hair.is_empty() {
                        hair_prefixes.push(hair.to_string());
                    }
                }
            }
        }
    }
    head_prefixes.sort();
    head_prefixes.dedup();
    hair_prefixes.sort();
    hair_prefixes.dedup();

    // 2. Build candidate resrefs (both genders) for Head 1 and Hairstyle 14.
    let mut candidates: Vec<(&str, String)> = Vec::new();
    for pfx in &head_prefixes {
        for gender in ['M', 'F'] {
            let base = pfx.replace('?', &gender.to_string());
            candidates.push(("HEAD01", format!("{base}01")));
        }
    }
    for pfx in &hair_prefixes {
        for gender in ['M', 'F'] {
            let base = pfx.replace('?', &gender.to_string());
            candidates.push(("HAIR14", format!("{base}14")));
        }
    }

    // 2b. Per-mesh breakdown for the Wild Elf ("Painted Elf") repro.
    writeln!(out, "\n=== per-mesh breakdown (Wild Elf head + hair) ===").unwrap();
    for resref in [
        "P_EWF_Head01",
        "P_EWM_Head01",
        "P_EEF_Hair14",
        "P_ELM_Head01",
        "P_HHM_Hair14",
    ] {
        let Ok(bytes) = rm.get_resource_bytes(resref, "mdb") else {
            continue;
        };
        let Ok(mdb) = MdbParser::parse(&bytes) else {
            continue;
        };
        writeln!(out, "\n  {resref}:").unwrap();
        for m in &mdb.rigid_meshes {
            writeln!(
                out,
                "    RIGID '{}' tint='{}' diffuse='{}' flags=0x{:02x}",
                m.name, m.material.tint_map_name, m.material.diffuse_map_name, m.material.flags
            )
            .unwrap();
        }
        for m in &mdb.skin_meshes {
            writeln!(
                out,
                "    SKIN  '{}' tint='{}' diffuse='{}' flags=0x{:02x}",
                m.name, m.material.tint_map_name, m.material.diffuse_map_name, m.material.flags
            )
            .unwrap();
        }
    }

    writeln!(out, "\n=== mask formats per candidate model ===").unwrap();
    for (kind, resref) in &candidates {
        if rm.get_resource_bytes(resref, "mdb").is_err() {
            continue; // model doesn't exist for this gender/prefix
        }
        let maps = tint_maps_of(&rm, resref);
        writeln!(out, "\n[{kind}] {resref}  tint_maps={maps:?}").unwrap();
        for map in &maps {
            match rm.get_resource_bytes(map, "dds") {
                Ok(dds) => {
                    writeln!(out, "    {map}: {}", describe_dds_format(&dds)).unwrap();
                    writeln!(out, "        {}", mask_coverage(&dds)).unwrap();
                }
                Err(e) => writeln!(out, "    {map}: NOT FOUND ({e})").unwrap(),
            }
        }
    }

    // 3. Spatial channel/alpha maps for the repro masks. Each cell shows the
    //    dominant tinted channel (r/g/b), '-' if all channels <10, '.' if the
    //    mask alpha is 0 there (our shader gates tint by tintMask.a, so '.'
    //    regions are NOT tinted regardless of color).
    writeln!(out, "\n=== spatial channel/alpha maps (24x16) ===").unwrap();
    for map in ["P_EWF_Head01_t", "P_HHM_Eye01_t", "P_HHF_Hair14_T"] {
        let Ok(bytes) = rm.get_resource_bytes(map, "dds") else {
            writeln!(out, "\n  {map}: NOT FOUND").unwrap();
            continue;
        };
        let decoded = decode_uncompressed_argb(&bytes).or_else(|| {
            app_lib::services::texture_decode::decode_dds_rgba(&bytes)
                .ok()
                .map(|t| (t.width, t.height, t.rgba))
        });
        let Some((w, h, rgba)) = decoded else {
            writeln!(out, "\n  {map}: decode failed").unwrap();
            continue;
        };
        writeln!(out, "\n  {map} ({w}x{h}):  (top=forehead/brows, mid=eyes)").unwrap();
        let (cols, rows) = (24usize, 16usize);
        for ry in 0..rows {
            let y = ry * h / rows;
            let mut line = String::from("    ");
            for rx in 0..cols {
                let x = rx * w / cols;
                let idx = (y * w + x) * 4;
                let (r, g, b, a) = (rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3]);
                let ch = if a == 0 {
                    '.'
                } else if r.max(g).max(b) < 10 {
                    '-'
                } else if r >= g && r >= b {
                    'r'
                } else if g >= b {
                    'g'
                } else {
                    'b'
                };
                line.push(ch);
            }
            writeln!(out, "{line}").unwrap();
        }
    }

    let out_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("head_hair_mask_format.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("\n{out}\nOutput written to: {}", out_path.display());
}
