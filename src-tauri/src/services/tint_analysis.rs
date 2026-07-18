//! Detects which tint channels a model's tint-mask textures actually use,
//! so the UI can disable color pickers that would have no visible effect.

use std::collections::HashMap;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use specta::Type;
use tracing::{debug, warn};

use crate::character::CharacterModelParts;
use crate::parsers::mdb::MdbParser;
use crate::services::resource_manager::ResourceManager;
use crate::services::texture_decode::decode_dds_rgba;

/// Minimum average alpha-weighted intensity for a mask channel to count as
/// "used" (~0.1% of a fully painted texture).
pub const TINT_CHANNEL_ALIVE_THRESHOLD: f32 = 0.001;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PartTintCapability {
    pub has_tint_map: bool,
    pub channels: [bool; 3],
}

impl PartTintCapability {
    /// Analysis failed or model unavailable — never lock the user out.
    pub fn permissive() -> Self {
        Self {
            has_tint_map: true,
            channels: [true; 3],
        }
    }

    pub fn untintable() -> Self {
        Self {
            has_tint_map: false,
            channels: [false; 3],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ColorCapabilities {
    pub head: PartTintCapability,
    pub hair: PartTintCapability,
    pub tail: Option<PartTintCapability>,
    pub wings: Option<PartTintCapability>,
}

impl Default for ColorCapabilities {
    fn default() -> Self {
        Self {
            head: PartTintCapability::permissive(),
            hair: PartTintCapability::permissive(),
            tail: None,
            wings: None,
        }
    }
}

/// Average alpha-weighted intensity per mask channel (R/G/B), 0.0-1.0.
pub fn channel_usage(rgba: &[u8]) -> [f32; 3] {
    let pixels = rgba.len() / 4;
    if pixels == 0 {
        return [0.0; 3];
    }
    let mut sum = [0.0f64; 3];
    for px in rgba.chunks_exact(4) {
        let a = f64::from(px[3]) / 255.0;
        sum[0] += f64::from(px[0]) / 255.0 * a;
        sum[1] += f64::from(px[1]) / 255.0 * a;
        sum[2] += f64::from(px[2]) / 255.0 * a;
    }
    let n = pixels as f64;
    [
        (sum[0] / n) as f32,
        (sum[1] / n) as f32,
        (sum[2] / n) as f32,
    ]
}

/// Map mask usage to tint channels 1..3. Tint masks are straight R/G/B
/// against the RAW GFF channel order; `swap_gb` is set only for parts whose
/// in-memory TintChannels is UI-reordered relative to the GFF (the head:
/// GFF skin/eyes/eyebrows vs UI skin/eyebrows/eyes), so capability indices
/// line up with what the UI pickers control. Raw-order parts (hair, root
/// body tint for tail/wings) use `swap_gb = false`.
pub fn channels_alive(usage: [f32; 3], swap_gb: bool) -> [bool; 3] {
    let alive = |v: f32| v > TINT_CHANNEL_ALIVE_THRESHOLD;
    if swap_gb {
        [alive(usage[0]), alive(usage[2]), alive(usage[1])]
    } else {
        [alive(usage[0]), alive(usage[1]), alive(usage[2])]
    }
}

fn analyze_model(rm: &ResourceManager, resref: &str, swap_gb: bool) -> PartTintCapability {
    let Ok(bytes) = rm.get_resource_bytes(resref, "mdb") else {
        debug!("tint analysis: no MDB '{resref}', permissive");
        return PartTintCapability::permissive();
    };
    let Ok(mdb) = MdbParser::parse(&bytes) else {
        warn!("tint analysis: MDB parse failed for '{resref}', permissive");
        return PartTintCapability::permissive();
    };

    let mut tint_maps: Vec<String> = Vec::new();
    let names = mdb
        .rigid_meshes
        .iter()
        .map(|m| m.material.tint_map_name.trim())
        .chain(
            mdb.skin_meshes
                .iter()
                .map(|m| m.material.tint_map_name.trim()),
        );
    for name in names {
        if !name.is_empty() && !tint_maps.iter().any(|t| t.eq_ignore_ascii_case(name)) {
            tint_maps.push(name.to_string());
        }
    }
    if tint_maps.is_empty() {
        return PartTintCapability::untintable();
    }

    let mut usage = [0.0f32; 3];
    for map in &tint_maps {
        let Ok(dds) = rm.get_resource_bytes(map, "dds") else {
            warn!("tint analysis: tint map '{map}' not found for '{resref}', permissive");
            return PartTintCapability::permissive();
        };
        match decode_dds_rgba(&dds) {
            Ok(tex) => {
                let u = channel_usage(&tex.rgba);
                for (acc, v) in usage.iter_mut().zip(u) {
                    *acc = acc.max(v);
                }
            }
            Err(e) => {
                warn!("tint analysis: decode failed for '{map}': {e}, permissive");
                return PartTintCapability::permissive();
            }
        }
    }
    PartTintCapability {
        has_tint_map: true,
        channels: channels_alive(usage, swap_gb),
    }
}

pub type TintCapabilityCache = Mutex<HashMap<String, PartTintCapability>>;

fn analyze_cached(
    rm: &ResourceManager,
    cache: &TintCapabilityCache,
    resref: &str,
    swap_gb: bool,
) -> PartTintCapability {
    let key = format!("{}|{swap_gb}", resref.to_lowercase());
    if let Some(hit) = cache.lock().get(&key) {
        return *hit;
    }
    let cap = analyze_model(rm, resref, swap_gb);
    cache.lock().insert(key, cap);
    cap
}

pub fn color_capabilities(
    rm: &ResourceManager,
    cache: &TintCapabilityCache,
    parts: &CharacterModelParts,
) -> ColorCapabilities {
    ColorCapabilities {
        head: analyze_cached(rm, cache, &parts.head_resref, true),
        hair: parts
            .hair_resref
            .as_deref()
            .map(|r| analyze_cached(rm, cache, r, false))
            .unwrap_or_else(PartTintCapability::untintable),
        tail: parts
            .tail_resref
            .as_deref()
            .map(|r| analyze_cached(rm, cache, r, false)),
        wings: parts
            .wings_resref
            .as_deref()
            .map(|r| analyze_cached(rm, cache, r, false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
        [r, g, b, a]
    }

    #[test]
    fn channel_usage_weights_by_alpha() {
        // 2 pixels: full-red opaque + full-green transparent
        let buf: Vec<u8> = [pixel(255, 0, 0, 255), pixel(0, 255, 0, 0)].concat();
        let usage = channel_usage(&buf);
        assert!((usage[0] - 0.5).abs() < 1e-4); // red: 1.0 on half the pixels
        assert!(usage[1] < 1e-6); // green fully masked out by alpha 0
        assert!(usage[2] < 1e-6);
    }

    #[test]
    fn channel_usage_empty_buffer_is_zero() {
        assert_eq!(channel_usage(&[]), [0.0; 3]);
    }

    #[test]
    fn channels_alive_straight_and_swapped() {
        let usage = [0.5, 0.0, 0.2]; // r used, g dead, b used
        assert_eq!(channels_alive(usage, false), [true, false, true]);
        // P_ masks: channel 2 reads BLUE, channel 3 reads GREEN
        assert_eq!(channels_alive(usage, true), [true, true, false]);
    }

    #[test]
    fn channels_alive_threshold() {
        let usage = [
            TINT_CHANNEL_ALIVE_THRESHOLD * 0.5,
            TINT_CHANNEL_ALIVE_THRESHOLD * 2.0,
            0.0,
        ];
        assert_eq!(channels_alive(usage, false), [false, true, false]);
    }

    #[test]
    fn default_capabilities_are_permissive_head_hair_and_no_tail_wings() {
        let caps = ColorCapabilities::default();
        assert_eq!(caps.head, PartTintCapability::permissive());
        assert_eq!(caps.hair, PartTintCapability::permissive());
        assert!(caps.tail.is_none());
        assert!(caps.wings.is_none());
    }
}
