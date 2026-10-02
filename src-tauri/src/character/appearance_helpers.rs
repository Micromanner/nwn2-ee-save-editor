use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::utils::parsing::row_str;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct TintChannel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct TintChannels {
    pub channel1: TintChannel,
    pub channel2: TintChannel,
    pub channel3: TintChannel,
}

impl TintChannels {
    pub fn has_color(&self) -> bool {
        [&self.channel1, &self.channel2, &self.channel3]
            .iter()
            .any(|ch| ch.r > 0 || ch.g > 0 || ch.b > 0 || ch.a > 0)
    }
}

pub fn read_tint_channel(fields: &IndexMap<String, GffValue<'_>>) -> TintChannel {
    let get_byte = |key: &str| -> u8 {
        match fields.get(key) {
            Some(GffValue::Byte(v)) => *v,
            _ => 0,
        }
    };
    TintChannel {
        r: get_byte("r"),
        g: get_byte("g"),
        b: get_byte("b"),
        a: get_byte("a"),
    }
}

pub fn read_tint_from_tintable(tintable: &IndexMap<String, GffValue<'_>>) -> TintChannels {
    let tint = match tintable.get("Tint") {
        Some(GffValue::StructOwned(s)) => s.as_ref().clone(),
        Some(GffValue::Struct(lazy)) => lazy.force_load(),
        _ => return TintChannels::default(),
    };
    read_tint_channels(&tint)
}

pub(crate) fn read_tint_channels(tint: &IndexMap<String, GffValue<'_>>) -> TintChannels {
    let ch = |key: &str| -> TintChannel {
        match tint.get(key) {
            Some(GffValue::StructOwned(s)) => read_tint_channel(s),
            Some(GffValue::Struct(lazy)) => read_tint_channel(&lazy.force_load()),
            _ => TintChannel::default(),
        }
    };
    TintChannels {
        channel1: ch("1"),
        channel2: ch("2"),
        channel3: ch("3"),
    }
}

pub fn build_tint_channel_struct(ch: &TintChannel) -> IndexMap<String, GffValue<'static>> {
    let mut map = IndexMap::new();
    map.insert("r".to_string(), GffValue::Byte(ch.r));
    map.insert("g".to_string(), GffValue::Byte(ch.g));
    map.insert("b".to_string(), GffValue::Byte(ch.b));
    map.insert("a".to_string(), GffValue::Byte(ch.a));
    map
}

pub fn build_tint_struct(tints: &TintChannels) -> IndexMap<String, GffValue<'static>> {
    let mut tint_map = IndexMap::new();
    tint_map.insert(
        "1".to_string(),
        GffValue::StructOwned(Box::new(build_tint_channel_struct(&tints.channel1))),
    );
    tint_map.insert(
        "2".to_string(),
        GffValue::StructOwned(Box::new(build_tint_channel_struct(&tints.channel2))),
    );
    tint_map.insert(
        "3".to_string(),
        GffValue::StructOwned(Box::new(build_tint_channel_struct(&tints.channel3))),
    );
    tint_map
}

pub fn build_tintable_struct(tints: &TintChannels) -> IndexMap<String, GffValue<'static>> {
    let mut tintable = IndexMap::new();
    tintable.insert(
        "Tint".to_string(),
        GffValue::StructOwned(Box::new(build_tint_struct(tints))),
    );
    tintable
}

pub fn build_nested_tint(tints: &TintChannels) -> IndexMap<String, GffValue<'static>> {
    let mut outer = IndexMap::new();
    outer.insert(
        "Tintable".to_string(),
        GffValue::StructOwned(Box::new(build_tintable_struct(tints))),
    );
    outer
}

/// Convert between raw GFF channels and the viewer's swapped channel order.
pub fn swap_tint_2_3(t: &TintChannels) -> TintChannels {
    TintChannels {
        channel1: t.channel1.clone(),
        channel2: t.channel3.clone(),
        channel3: t.channel2.clone(),
    }
}

/// Build the render-time tint for glove / bracer meshes: force channel 3
/// to white before the 2↔3 swap. A black channel 3 on these items would
/// otherwise darken the entire mesh; the game ignores it in practice.
pub fn cosmetic_gloves_tint(src: &TintChannels) -> TintChannels {
    let mut pre_swap = src.clone();
    pre_swap.channel3 = TintChannel {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    swap_tint_2_3(&pre_swap)
}

/// Mesh number for a GFF `Variation`: the field is 0-indexed (the toolset
/// shows "Variation 0") while armor mesh filenames are 1-indexed (`Body01`).
pub fn mesh_number(variation: i32) -> i32 {
    (variation + 1).max(1)
}

pub fn resolve_armor_prefix(game_data: &GameData, visual_type: i32) -> Option<String> {
    if visual_type < 0 {
        return None;
    }
    let row = game_data.get_table("armor")?.get_by_id(visual_type)?;
    row_str(&row, "prefix")
}
