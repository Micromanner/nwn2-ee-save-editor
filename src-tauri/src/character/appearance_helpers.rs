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

/// Swap channels 2 and 3 on a tint. Glove and boot MDBs tag their tint
/// masks with an opposite convention to chest armor; swapping the incoming
/// tint once here lets the same shader code render both.
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

pub fn resolve_armor_prefix(
    game_data: &GameData,
    visual_type: i32,
    one_indexed: bool,
) -> Vec<String> {
    let mut prefixes = Vec::new();
    let Some(armor_table) = game_data.get_table("armor") else {
        return prefixes;
    };

    // Primary index based on context, then try the other as fallback
    let primary = if one_indexed {
        visual_type - 1
    } else {
        visual_type
    };
    let fallback = if one_indexed {
        visual_type
    } else {
        visual_type - 1
    };

    for row_id in [primary, fallback] {
        if row_id >= 0
            && let Some(row) = armor_table.get_by_id(row_id)
            && let Some(prefix) = row_str(&row, "prefix")
            && !prefixes.contains(&prefix)
        {
            prefixes.push(prefix);
        }
    }
    prefixes
}
