use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use tracing::{debug, info};

use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::services::resource_manager::ResourceManager;
use crate::utils::parsing::row_str;

use super::appearance_helpers::{TintChannels, read_tint_from_tintable, resolve_armor_prefix};
use super::gff_helpers::gff_value_to_i32;
use super::inventory::EquipmentSlot;

/// Classification derived from `baseitems.2da`'s `modeltype` column.
///
/// NWN2's real values are `"0"` (single-part), `"2"` (3-part weapon),
/// `"3"` (body armour), or empty (no in-world model).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemModelKind {
    ThreePartWeapon,
    SinglePart,
    BodyArmor,
    None,
}

fn classify_model_type(raw: &str) -> ItemModelKind {
    match raw.trim() {
        "2" => ItemModelKind::ThreePartWeapon,
        "0" => ItemModelKind::SinglePart,
        "3" => ItemModelKind::BodyArmor,
        _ => ItemModelKind::None,
    }
}

/// Bracers ship with `modeltype=0` but render as a fixed glove mesh, not
/// as a weapon. Detected by the base item's label since there is no
/// dedicated modeltype for them.
pub(crate) fn is_bracer_label(s: &str) -> bool {
    s.eq_ignore_ascii_case("bracer")
}

/// Shield MDBs (`W_She_*`) are authored lying flat in their arm-attachment
/// orientation, so the preview has to stand them up. Takes the normalized
/// weapon prefix, e.g. `W_She_Towr`.
fn is_shield_prefix(full_prefix: &str) -> bool {
    full_prefix.to_lowercase().starts_with("w_she")
}

/// Which armor part the item occupies, derived from `baseitems.2da`'s
/// `equipableslots` bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArmorSlot {
    Head,
    Body,
    Boots,
    Gloves,
    Cloak,
}

impl ArmorSlot {
    /// The filename fragment NWN2 uses for this slot
    /// (e.g. `P_HHM_LE_Body01.mdb`, `P_HHM_LE_Helm01.mdb`).
    pub(crate) fn part_name(self) -> &'static str {
        match self {
            Self::Head => "Helm",
            Self::Body => "Body",
            Self::Boots => "Boots",
            Self::Gloves => "Gloves",
            Self::Cloak => "Cloak",
        }
    }
}

pub(crate) fn parse_equip_slots(raw: &str) -> u32 {
    let s = raw.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        s.parse::<u32>().unwrap_or(0)
    }
}

pub(crate) fn detect_armor_slot(equip_slots: u32) -> Option<ArmorSlot> {
    // Order matters: head before chest because head has its own dedicated helmet
    // base items, while a few chest-style items also occupy other bits.
    let has = |slot: EquipmentSlot| equip_slots & slot.to_bitmask() != 0;
    if has(EquipmentSlot::Head) {
        Some(ArmorSlot::Head)
    } else if has(EquipmentSlot::Chest) {
        Some(ArmorSlot::Body)
    } else if has(EquipmentSlot::Boots) {
        Some(ArmorSlot::Boots)
    } else if has(EquipmentSlot::Gloves) {
        Some(ArmorSlot::Gloves)
    } else if has(EquipmentSlot::Cloak) {
        Some(ArmorSlot::Cloak)
    } else {
        None
    }
}

/// Default body prefix for the isolated item viewer. NWN2 armor meshes are
/// stamped per race/gender (e.g. `P_HHM_` for Human Male); without a wearer
/// context we pick a standard one. Files not matching this prefix simply
/// won't load — the caller handles that by showing "No preview available".
pub const DEFAULT_BODY_PREFIX: &str = "P_HHM";

/// Common armor-material prefixes from `armor.2da`, tried as fallbacks when
/// the item's own `ArmorVisualType` doesn't resolve (e.g. for helmets, whose
/// material normally comes from the wearer's chest armor).
const FALLBACK_ARMOR_PREFIXES: &[&str] = &["LE", "CL", "CH", "BA", "PF"];

/// `P_HHM_CL_{Body,Boots,Gloves}99.MDB` are the engine's intentionally empty
/// "wear nothing" meshes: they load fine but draw nothing, so the gallery
/// would show a blank tile for them.
const INVISIBLE_ARMOR_VARIANT: i32 = 99;

/// A body-armour item's nested Boots/Gloves sub-part. NWN2 chest armour items
/// store these inline — each has its own `ArmorVisualType` (indexing `armor.2da`)
/// and `Variation` (mesh variant number).
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct NestedArmorPart {
    pub armor_visual_type: Option<i32>,
    pub variation: i32,
    #[serde(default)]
    pub tints: Option<super::appearance_helpers::TintChannels>,
}

/// One of the 22 accessory slots an NWN2 chest-armor UTI can declare.
/// 16 map to `A_{body}_{slot}{NN}.mdb` rigid meshes (pauldrons, bracers, etc.);
/// the remaining 6 (Ankle/Foot/BkHip/FtHip) are vestigial — the UTI carries
/// the field for round-trip fidelity but no MDB family exists on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub enum AccessorySlot {
    LtShoulder,
    RtShoulder,
    LtArm,
    RtArm,
    LtElbow,
    RtElbow,
    LtBracer,
    RtBracer,
    LtLeg,
    RtLeg,
    LtKnee,
    RtKnee,
    LtShin,
    RtShin,
    LtHip,
    RtHip,
    LtAnkle,
    RtAnkle,
    LtFoot,
    RtFoot,
    BkHip,
    FtHip,
}

impl AccessorySlot {
    /// UTI struct field name used to read (and later write) this slot.
    pub(crate) fn uti_field(self) -> &'static str {
        match self {
            Self::LtShoulder => "ACLtShoulder",
            Self::RtShoulder => "ACRtShoulder",
            Self::LtArm => "ACLtArm",
            Self::RtArm => "ACRtArm",
            Self::LtElbow => "ACLtElbow",
            Self::RtElbow => "ACRtElbow",
            Self::LtBracer => "ACLtBracer",
            Self::RtBracer => "ACRtBracer",
            Self::LtLeg => "ACLtLeg",
            Self::RtLeg => "ACRtLeg",
            Self::LtKnee => "ACLtKnee",
            Self::RtKnee => "ACRtKnee",
            Self::LtShin => "ACLtShin",
            Self::RtShin => "ACRtShin",
            Self::LtHip => "ACLtHip",
            Self::RtHip => "ACRtHip",
            Self::LtAnkle => "ACLtAnkle",
            Self::RtAnkle => "ACRtAnkle",
            Self::LtFoot => "ACLtFoot",
            Self::RtFoot => "ACRtFoot",
            Self::BkHip => "ACBkHip",
            Self::FtHip => "ACFtHip",
        }
    }

    /// Skeleton bone name this slot's rigid mesh parents to. Matches bone
    /// names in `P_HHM_skel` (which all races use, since no per-race
    /// skeleton exists). `None` for vestigial slots.
    ///
    /// Mapping derived by inspecting `P_HHM_skel.gr2` bone list — no
    /// dedicated `ap_shoulder_*`/`ap_elbow_*`/etc. attach points exist, so
    /// the accessories parent to the underlying skeletal bone whose segment
    /// they sit on (collarbone, upper arm, forearm, etc.).
    pub(crate) fn attach_bone(self) -> Option<&'static str> {
        match self {
            // Pauldrons parent to the upper-arm bone (same as LtArm/RtArm) —
            // `*CollarBone` is anatomically at the throat, which puts the
            // pauldron at the gorget/neck instead of the shoulder joint.
            // Multiple accessories sharing the same bone is fine; each
            // carries its own local origin in the MDB.
            Self::LtShoulder => Some("LArm010"),
            Self::RtShoulder => Some("RArm110"),
            Self::LtArm => Some("LArm010"),
            Self::RtArm => Some("RArm110"),
            Self::LtElbow => Some("LArm011"),
            Self::RtElbow => Some("RArm111"),
            Self::LtBracer => Some("LArm02"),
            Self::RtBracer => Some("RArm12"),
            Self::LtLeg => Some("LLeg1"),
            Self::RtLeg => Some("RLeg1"),
            // Kneecaps parent to the shin bone, whose origin IS the knee
            // joint. The `ap_knee_*` effect points sit ~7cm off it (4cm down
            // the thigh, 5cm forward) and carry a ~180 deg Y rotation, which
            // threw the accessory well clear of the knee.
            Self::LtKnee => Some("LLeg2"),
            Self::RtKnee => Some("RLeg2"),
            Self::LtShin => Some("LLeg2"),
            Self::RtShin => Some("RLeg2"),
            Self::LtHip => Some("LHip1"),
            Self::RtHip => Some("RHip1"),
            Self::LtAnkle
            | Self::RtAnkle
            | Self::LtFoot
            | Self::RtFoot
            | Self::BkHip
            | Self::FtHip => None,
        }
    }

    /// Match an MDB slot fragment (`"LShoulder"`, `"LUpArm"`, …) back to the
    /// `AccessorySlot` enum. Used by the item loader to tag rigid accessory
    /// meshes with their `attach_bone`.
    pub(crate) fn from_mdb_slot(slot: &str) -> Option<Self> {
        for s in Self::all() {
            if s.mdb_slot() == Some(slot) {
                return Some(*s);
            }
        }
        None
    }

    /// Filename slot fragment for the `A_{body}_{slot}{NN}.mdb` pattern.
    /// `None` for the 6 vestigial slots that have no MDB family on disk.
    pub(crate) fn mdb_slot(self) -> Option<&'static str> {
        match self {
            Self::LtShoulder => Some("LShoulder"),
            Self::RtShoulder => Some("RShoulder"),
            Self::LtArm => Some("LUpArm"),
            Self::RtArm => Some("RUpArm"),
            Self::LtElbow => Some("LElbow"),
            Self::RtElbow => Some("RElbow"),
            Self::LtBracer => Some("LBracer"),
            Self::RtBracer => Some("RBracer"),
            Self::LtLeg => Some("LUpLeg"),
            Self::RtLeg => Some("RUpLeg"),
            Self::LtKnee => Some("LKnee"),
            Self::RtKnee => Some("RKnee"),
            Self::LtShin => Some("LLowLeg"),
            Self::RtShin => Some("RLowLeg"),
            Self::LtHip => Some("LHip"),
            Self::RtHip => Some("RHip"),
            Self::LtAnkle
            | Self::RtAnkle
            | Self::LtFoot
            | Self::RtFoot
            | Self::BkHip
            | Self::FtHip => None,
        }
    }

    /// Every slot; iteration order matches variant declaration order.
    pub(crate) fn all() -> &'static [AccessorySlot] {
        &[
            Self::LtShoulder,
            Self::RtShoulder,
            Self::LtArm,
            Self::RtArm,
            Self::LtElbow,
            Self::RtElbow,
            Self::LtBracer,
            Self::RtBracer,
            Self::LtLeg,
            Self::RtLeg,
            Self::LtKnee,
            Self::RtKnee,
            Self::LtShin,
            Self::RtShin,
            Self::LtHip,
            Self::RtHip,
            Self::LtAnkle,
            Self::RtAnkle,
            Self::LtFoot,
            Self::RtFoot,
            Self::BkHip,
            Self::FtHip,
        ]
    }
}

/// The full content of one accessory slot: the mesh id plus the per-slot
/// tint channels. Each AC* UTI struct carries its own `Tintable`, so a
/// darksteel-plate's shoulder can be tinted black while its bracers stay
/// steel-grey.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AccessorySlotData {
    pub id: i32,
    pub tints: TintChannels,
}

/// Per-slot accessory ids + tints for a chest-armor item.
///
/// `None` = UTI field absent (or malformed). `Some { id: 0, .. }` = present
/// but empty. `Some { id: n > 0, .. }` = active accessory; the resolver
/// emits an MDB resref and the per-slot `tints` drive that mesh's colour.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ArmorAccessories {
    pub lt_shoulder: Option<AccessorySlotData>,
    pub rt_shoulder: Option<AccessorySlotData>,
    pub lt_arm: Option<AccessorySlotData>,
    pub rt_arm: Option<AccessorySlotData>,
    pub lt_elbow: Option<AccessorySlotData>,
    pub rt_elbow: Option<AccessorySlotData>,
    pub lt_bracer: Option<AccessorySlotData>,
    pub rt_bracer: Option<AccessorySlotData>,
    pub lt_leg: Option<AccessorySlotData>,
    pub rt_leg: Option<AccessorySlotData>,
    pub lt_knee: Option<AccessorySlotData>,
    pub rt_knee: Option<AccessorySlotData>,
    pub lt_shin: Option<AccessorySlotData>,
    pub rt_shin: Option<AccessorySlotData>,
    pub lt_hip: Option<AccessorySlotData>,
    pub rt_hip: Option<AccessorySlotData>,
    pub lt_ankle: Option<AccessorySlotData>,
    pub rt_ankle: Option<AccessorySlotData>,
    pub lt_foot: Option<AccessorySlotData>,
    pub rt_foot: Option<AccessorySlotData>,
    pub bk_hip: Option<AccessorySlotData>,
    pub ft_hip: Option<AccessorySlotData>,
}

/// Everything the mesh loader needs for one active accessory.
pub struct RenderableAccessory<'a> {
    pub slot: AccessorySlot,
    pub resref: String,
    pub attach_bone: &'static str,
    pub tints: &'a TintChannels,
}

impl ArmorAccessories {
    /// Convenience: read just the id for this slot (ignores tints). Returns
    /// `None` for missing/malformed fields and matches the legacy API.
    pub fn get(&self, slot: AccessorySlot) -> Option<i32> {
        self.get_data(slot).map(|d| d.id)
    }

    /// Read the per-slot tints; `None` for slots the UTI didn't carry.
    pub fn get_tints(&self, slot: AccessorySlot) -> Option<&TintChannels> {
        self.get_data(slot).map(|d| &d.tints)
    }

    /// Yield each active rendering accessory with its resref, bone, and
    /// tints. Single source of truth for the item viewer's resref-group
    /// output *and* the character viewer's direct load path.
    pub fn iter_renderable<'a>(
        &'a self,
        body_prefix: &str,
    ) -> impl Iterator<Item = RenderableAccessory<'a>> + 'a {
        let body = body_prefix.to_string();
        AccessorySlot::all().iter().filter_map(move |slot| {
            let data = self.get_data(*slot)?;
            let resref = accessory_resref(&body, *slot, data.id)?;
            let attach_bone = slot.attach_bone()?;
            Some(RenderableAccessory {
                slot: *slot,
                resref,
                attach_bone,
                tints: &data.tints,
            })
        })
    }

    pub fn get_data(&self, slot: AccessorySlot) -> Option<&AccessorySlotData> {
        match slot {
            AccessorySlot::LtShoulder => self.lt_shoulder.as_ref(),
            AccessorySlot::RtShoulder => self.rt_shoulder.as_ref(),
            AccessorySlot::LtArm => self.lt_arm.as_ref(),
            AccessorySlot::RtArm => self.rt_arm.as_ref(),
            AccessorySlot::LtElbow => self.lt_elbow.as_ref(),
            AccessorySlot::RtElbow => self.rt_elbow.as_ref(),
            AccessorySlot::LtBracer => self.lt_bracer.as_ref(),
            AccessorySlot::RtBracer => self.rt_bracer.as_ref(),
            AccessorySlot::LtLeg => self.lt_leg.as_ref(),
            AccessorySlot::RtLeg => self.rt_leg.as_ref(),
            AccessorySlot::LtKnee => self.lt_knee.as_ref(),
            AccessorySlot::RtKnee => self.rt_knee.as_ref(),
            AccessorySlot::LtShin => self.lt_shin.as_ref(),
            AccessorySlot::RtShin => self.rt_shin.as_ref(),
            AccessorySlot::LtHip => self.lt_hip.as_ref(),
            AccessorySlot::RtHip => self.rt_hip.as_ref(),
            AccessorySlot::LtAnkle => self.lt_ankle.as_ref(),
            AccessorySlot::RtAnkle => self.rt_ankle.as_ref(),
            AccessorySlot::LtFoot => self.lt_foot.as_ref(),
            AccessorySlot::RtFoot => self.rt_foot.as_ref(),
            AccessorySlot::BkHip => self.bk_hip.as_ref(),
            AccessorySlot::FtHip => self.ft_hip.as_ref(),
        }
    }

    pub fn set_data(&mut self, slot: AccessorySlot, value: Option<AccessorySlotData>) {
        match slot {
            AccessorySlot::LtShoulder => self.lt_shoulder = value,
            AccessorySlot::RtShoulder => self.rt_shoulder = value,
            AccessorySlot::LtArm => self.lt_arm = value,
            AccessorySlot::RtArm => self.rt_arm = value,
            AccessorySlot::LtElbow => self.lt_elbow = value,
            AccessorySlot::RtElbow => self.rt_elbow = value,
            AccessorySlot::LtBracer => self.lt_bracer = value,
            AccessorySlot::RtBracer => self.rt_bracer = value,
            AccessorySlot::LtLeg => self.lt_leg = value,
            AccessorySlot::RtLeg => self.rt_leg = value,
            AccessorySlot::LtKnee => self.lt_knee = value,
            AccessorySlot::RtKnee => self.rt_knee = value,
            AccessorySlot::LtShin => self.lt_shin = value,
            AccessorySlot::RtShin => self.rt_shin = value,
            AccessorySlot::LtHip => self.lt_hip = value,
            AccessorySlot::RtHip => self.rt_hip = value,
            AccessorySlot::LtAnkle => self.lt_ankle = value,
            AccessorySlot::RtAnkle => self.rt_ankle = value,
            AccessorySlot::LtFoot => self.lt_foot = value,
            AccessorySlot::RtFoot => self.rt_foot = value,
            AccessorySlot::BkHip => self.bk_hip = value,
            AccessorySlot::FtHip => self.ft_hip = value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ItemAppearance {
    /// Weapon parts (ModelPart1, 2, 3) or Armor Variation
    pub variation: i32,
    /// For weapons: ModelPart1, 2, 3 values.
    /// For armor: These might be used for accessories later.
    pub model_parts: [i32; 3],
    /// Tints for the item
    pub tints: TintChannels,
    /// Armor Visual Type (if applicable)
    pub armor_visual_type: Option<i32>,
    /// Nested Boots sub-part (only populated for body armour items that
    /// ship with a matching pair of boots baked into the item GFF).
    #[serde(default)]
    pub boots: Option<NestedArmorPart>,
    /// Nested Gloves sub-part (see `boots`).
    #[serde(default)]
    pub gloves: Option<NestedArmorPart>,
    /// Per-slot accessory ids (pauldrons, bracers, greaves, etc.). Populated
    /// only for chest armor; default (all `None`) otherwise.
    #[serde(default)]
    pub accessories: ArmorAccessories,
}

/// Which GFF field the variation gallery actually drives. SinglePart items
/// (modeltype 0: shields, bows, crossbows) resolve their mesh from
/// `ModelPart1` and treat `Variation` as a UI mirror — see `prepare_for_write`.
/// The UI has to write both, otherwise the live preview keeps the old mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum VariationField {
    Variation,
    ModelPart1,
}

/// How the preview should pose the model. Item MDBs are authored in their
/// attachment orientation, which for shields means lying flat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PreviewPose {
    Weapon,
    Shield,
    Armor,
}

/// One `armor.2da` row the material select can write into
/// `ArmorVisualType`. Purely visual: armor rules come from
/// `ArmorRulesType` and the item properties, so any material can sit on any
/// stats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct MaterialOption {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ItemAppearanceOptions {
    pub available_variations: Vec<i32>,
    /// Materials with at least one mesh on disk for this armor slot. Empty
    /// for anything that is not chest/boots/gloves/helmet.
    pub available_materials: Vec<MaterialOption>,
    pub available_part1: Vec<i32>,
    pub available_part2: Vec<i32>,
    pub available_part3: Vec<i32>,
    /// Whether this base item has an in-world model at all. Rings, amulets
    /// and other modeltype-less items have none — tints and variants are
    /// meaningless for them and the UI hides those controls.
    pub has_model: bool,
    pub variation_field: VariationField,
    pub preview_pose: PreviewPose,
    /// Accessory meshes on disk per slot (`A_{body}_{slot}NN.mdb`), only for
    /// chest armor; slots without a single mesh are omitted.
    pub available_accessories: Vec<AccessorySlotOptions>,
}

/// The accessory ids that exist on disk for one slot, in the character's
/// body prefix.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AccessorySlotOptions {
    pub slot: AccessorySlot,
    pub ids: Vec<i32>,
}

impl ItemAppearance {
    pub fn from_gff(fields: &IndexMap<String, GffValue<'_>>) -> Self {
        // Keep raw values (0 = not set). The resolver picks which field to use
        // based on item type — SinglePart weapons (shields) put the variant in
        // `ModelPart1`; cloaks use `Variation`; a generic clamp would erase
        // the "missing" signal we need to decide between them.
        let read_field = |key: &str| {
            fields
                .get(key)
                .and_then(gff_value_to_i32)
                .filter(|&v| v > 0)
                .unwrap_or(0)
        };
        let variation = read_field("Variation");
        let model_parts = [
            read_field("ModelPart1"),
            read_field("ModelPart2"),
            read_field("ModelPart3"),
        ];

        let tints = fields
            .get("Tintable")
            .and_then(|v| match v {
                GffValue::StructOwned(s) => Some(s.as_ref().clone()),
                GffValue::Struct(lazy) => Some(lazy.force_load()),
                _ => None,
            })
            .map(|t| read_tint_from_tintable(&t))
            .unwrap_or_default();

        let armor_visual_type = fields.get("ArmorVisualType").and_then(gff_value_to_i32);

        let boots = read_nested_armor_part(fields, "Boots");
        let gloves = read_nested_armor_part(fields, "Gloves");

        Self {
            variation,
            model_parts,
            tints,
            armor_visual_type,
            boots,
            gloves,
            accessories: read_accessories(fields),
        }
    }

    /// Adjust raw GFF values into a unified "display variation" the UI can
    /// show and edit. NWN2 stores the variant index in different fields
    /// depending on item type — shields and other modeltype=0 items use
    /// `ModelPart1`; everything else uses `Variation`. Without this mapping
    /// a shield loaded from a save would show "0/N" in the stepper while
    /// the 3D viewer renders the real variant.
    pub fn normalize_for_ui(&mut self, base_item_id: i32, game_data: &GameData) {
        let Some(table) = game_data.get_table("baseitems") else {
            return;
        };
        let Some(row) = table.get_by_id(base_item_id) else {
            return;
        };
        let kind = classify_model_type(&row_str(&row, "modeltype").unwrap_or_default());
        if matches!(kind, ItemModelKind::SinglePart)
            && self.variation == 0
            && self.model_parts[0] > 0
        {
            self.variation = self.model_parts[0];
        }
    }

    /// The armor slot the base item itself occupies, or `None` for anything
    /// that is not body armor. Lets callers tell an item's *own* boots/gloves
    /// mesh apart from the boots/gloves nested inside a chest item.
    pub(crate) fn base_item_armor_slot(
        base_item_id: i32,
        game_data: &GameData,
    ) -> Option<ArmorSlot> {
        let table = game_data.get_table("baseitems")?;
        let row = table.get_by_id(base_item_id)?;
        if classify_model_type(&row_str(&row, "modeltype").unwrap_or_default())
            != ItemModelKind::BodyArmor
        {
            return None;
        }
        detect_armor_slot(parse_equip_slots(
            &row_str(&row, "equipableslots").unwrap_or_default(),
        ))
    }

    /// Route the user-facing `variation` back into the correct GFF field
    /// before the item is serialized. Counterpart to `normalize_for_ui`:
    /// for SinglePart items the game reads `ModelPart1`, so the stepper's
    /// value needs to land there rather than (or in addition to) `Variation`.
    ///
    /// Returns `true` when the item is a SinglePart kind, i.e. `variation`
    /// is a UI-only mirror of `ModelPart1` — the writer must then not
    /// create a `Variation` field the item never had.
    pub fn prepare_for_write(&mut self, base_item_id: i32, game_data: &GameData) -> bool {
        let Some(table) = game_data.get_table("baseitems") else {
            return false;
        };
        let Some(row) = table.get_by_id(base_item_id) else {
            return false;
        };
        let kind = classify_model_type(&row_str(&row, "modeltype").unwrap_or_default());
        if !matches!(kind, ItemModelKind::SinglePart) {
            return false;
        }
        if self.variation > 0 {
            self.model_parts[0] = self.variation;
        }
        true
    }

    pub fn get_options(
        base_item_id: i32,
        armor_visual_type: Option<i32>,
        body_prefix: Option<&str>,
        game_data: &GameData,
        resource_manager: &ResourceManager,
    ) -> ItemAppearanceOptions {
        let Some(table) = game_data.get_table("baseitems") else {
            return ItemAppearanceOptions::new_empty();
        };

        let Some(row) = table.get_by_id(base_item_id) else {
            return ItemAppearanceOptions::new_empty();
        };

        let (prefix, source) = Self::resolve_item_prefix(&row);
        let kind = classify_model_type(&row_str(&row, "modeltype").unwrap_or_default());

        info!(
            "Resolving appearance options for item {base_item_id}: prefix='{prefix}' from {source}, kind={kind:?}"
        );

        // Bracer base items have modeltype=0 but behave like gloves; enumerate
        // glove-slot variants so the stepper offers usable choices.
        let is_bracer = is_bracer_label(&prefix);

        match kind {
            ItemModelKind::ThreePartWeapon => {
                let full_prefix = normalize_weapon_prefix(&prefix);
                ItemAppearanceOptions {
                    available_variations: Vec::new(),
                    available_materials: Vec::new(),
                    available_part1: Self::discover_variants(resource_manager, &full_prefix, "_a"),
                    available_part2: Self::discover_variants(resource_manager, &full_prefix, "_b"),
                    available_part3: Self::discover_variants(resource_manager, &full_prefix, "_c"),
                    has_model: true,
                    variation_field: VariationField::Variation,
                    preview_pose: PreviewPose::Weapon,
                    available_accessories: Vec::new(),
                }
            }
            ItemModelKind::SinglePart if is_bracer => {
                // Bracer items render as a fixed glove mesh (variant 01);
                // the mesh is chosen entirely by AVT → material. Variation
                // / ModelPart1 on these items don't drive mesh selection,
                // so expose no stepper — but the mesh exists and tints apply.
                ItemAppearanceOptions {
                    has_model: true,
                    preview_pose: PreviewPose::Armor,
                    ..ItemAppearanceOptions::new_empty()
                }
            }
            ItemModelKind::SinglePart => {
                let full_prefix = normalize_weapon_prefix(&prefix);
                let available_variations =
                    Self::discover_variants(resource_manager, &full_prefix, "");
                // Rings, amulets and belts also ship modeltype=0 but have no
                // itemclass and no MDB family on disk — zero discovered
                // variants means there is nothing to render or tint.
                let has_model = !available_variations.is_empty();
                ItemAppearanceOptions {
                    available_variations,
                    available_materials: Vec::new(),
                    available_part1: Vec::new(),
                    available_part2: Vec::new(),
                    available_part3: Vec::new(),
                    has_model,
                    variation_field: VariationField::ModelPart1,
                    preview_pose: if is_shield_prefix(&full_prefix) {
                        PreviewPose::Shield
                    } else {
                        PreviewPose::Weapon
                    },
                    available_accessories: Vec::new(),
                }
            }
            ItemModelKind::BodyArmor => {
                let equip_slots =
                    parse_equip_slots(&row_str(&row, "equipableslots").unwrap_or_default());
                let slot = detect_armor_slot(equip_slots);
                let body = body_prefix.unwrap_or(DEFAULT_BODY_PREFIX);
                let available_variations = Self::discover_armor_variants(
                    body,
                    slot,
                    armor_visual_type,
                    game_data,
                    resource_manager,
                );
                let available_materials =
                    Self::discover_armor_materials(body, slot, game_data, resource_manager);
                let available_accessories = if slot == Some(ArmorSlot::Body) {
                    Self::discover_accessories(body, resource_manager)
                } else {
                    Vec::new()
                };
                ItemAppearanceOptions {
                    // Discovery now searches exactly the material prefixes the
                    // loader will try, so an empty list means nothing renders.
                    has_model: slot.is_some() && !available_variations.is_empty(),
                    available_variations,
                    available_materials,
                    available_part1: Vec::new(),
                    available_part2: Vec::new(),
                    available_part3: Vec::new(),
                    variation_field: VariationField::Variation,
                    preview_pose: PreviewPose::Armor,
                    available_accessories,
                }
            }
            ItemModelKind::None => ItemAppearanceOptions::new_empty(),
        }
    }

    /// Accessory ids per slot with a mesh on disk, using the same
    /// `A_{body}_{slot}NN` naming `ArmorAccessories::iter_renderable` resolves
    /// at load time. Vestigial slots (no MDB family) are never listed.
    fn discover_accessories(
        body_prefix: &str,
        resource_manager: &ResourceManager,
    ) -> Vec<AccessorySlotOptions> {
        let body = body_prefix
            .strip_prefix("P_")
            .or_else(|| body_prefix.strip_prefix("p_"))
            .unwrap_or(body_prefix);
        AccessorySlot::all()
            .iter()
            .filter_map(|slot| {
                let mdb_slot = slot.mdb_slot()?;
                let ids =
                    Self::discover_variants(resource_manager, &format!("A_{body}_{mdb_slot}"), "");
                (!ids.is_empty()).then_some(AccessorySlotOptions { slot: *slot, ids })
            })
            .collect()
    }

    /// Discover real mesh variants on disk for a body-armor slot by inspecting
    /// the `P_HHM_{material}_{Part}NN.mdb` files that actually exist.
    ///
    /// The material prefixes searched here must be exactly the ones
    /// `build_armor_resrefs` will try at load time, otherwise the gallery
    /// offers variants the viewer cannot render. Returned values are in the
    /// same 0-indexed space as the GFF `Variation` field (the loader renders
    /// `Variation + 1`), so `Boots01` is reported as `0`.
    fn discover_armor_variants(
        body_prefix: &str,
        slot: Option<ArmorSlot>,
        armor_visual_type: Option<i32>,
        game_data: &GameData,
        resource_manager: &ResourceManager,
    ) -> Vec<i32> {
        let Some(slot) = slot else {
            return Vec::new();
        };
        let part = slot.part_name();

        let item_prefix = armor_visual_type.and_then(|vt| {
            resolve_armor_prefix(game_data, vt, false)
                .into_iter()
                .next()
        });

        let material_prefixes: Vec<String> = match slot {
            // Cloaks are hardcoded to CL by the loader.
            ArmorSlot::Cloak => vec!["CL".to_string()],
            // Helmets take the wearer's chest material, so the loader falls
            // back to the common set when the item carries no usable VT.
            ArmorSlot::Head => match item_prefix {
                Some(pfx) => vec![pfx],
                None => FALLBACK_ARMOR_PREFIXES
                    .iter()
                    .map(|p| p.to_string())
                    .collect(),
            },
            // The loader renders nothing for these without a VT prefix, so an
            // empty list is the honest answer.
            ArmorSlot::Body | ArmorSlot::Boots | ArmorSlot::Gloves => {
                item_prefix.into_iter().collect()
            }
        };

        let mut variants: Vec<i32> = Vec::new();
        for mat in &material_prefixes {
            let full_prefix = format!("{body_prefix}_{mat}_{part}");
            variants.extend(
                Self::discover_variants(resource_manager, &full_prefix, "")
                    .into_iter()
                    .filter(|nn| *nn != INVISIBLE_ARMOR_VARIANT)
                    .filter_map(|nn| nn.checked_sub(1).filter(|v| *v >= 0)),
            );
        }
        variants.sort_unstable();
        variants.dedup();
        variants
    }

    /// Every `armor.2da` material that has at least one real mesh on disk
    /// for this slot, so the material select never offers a look the viewer
    /// (and the game) cannot render. Cloaks are hardcoded to `CL` by the
    /// loader and get no choice.
    fn discover_armor_materials(
        body_prefix: &str,
        slot: Option<ArmorSlot>,
        game_data: &GameData,
        resource_manager: &ResourceManager,
    ) -> Vec<MaterialOption> {
        let Some(slot) = slot else {
            return Vec::new();
        };
        if slot == ArmorSlot::Cloak {
            return Vec::new();
        }
        let Some(armor_table) = game_data.get_table("armor") else {
            return Vec::new();
        };
        let part = slot.part_name();

        (0..armor_table.row_count())
            .filter_map(|idx| {
                let row = armor_table.get_row(idx).ok()?;
                let prefix = row_str(&row, "prefix")?;
                let full_prefix = format!("{body_prefix}_{prefix}_{part}");
                let has_mesh = Self::discover_variants(resource_manager, &full_prefix, "")
                    .into_iter()
                    .any(|nn| nn != INVISIBLE_ARMOR_VARIANT);
                if !has_mesh {
                    return None;
                }
                let name = row_str(&row, "name")
                    .and_then(|s| s.parse::<i32>().ok())
                    .and_then(|strref| game_data.get_string(strref))
                    .or_else(|| row_str(&row, "label"))?;
                Some(MaterialOption {
                    id: idx as i32,
                    name,
                })
            })
            .collect()
    }

    fn discover_variants(
        resource_manager: &ResourceManager,
        prefix: &str,
        suffix: &str,
    ) -> Vec<i32> {
        let prefix_lower = prefix.to_lowercase();
        let suffix_lower = suffix.to_lowercase();
        let mdbs = resource_manager.list_resources_by_prefix(&prefix_lower, "mdb");
        debug!(
            "discover_variants: searching prefix '{}' suffix '{}', found {} MDBs",
            prefix_lower,
            suffix_lower,
            mdbs.len()
        );

        let mut variants: Vec<i32> = mdbs
            .iter()
            .filter_map(|name| {
                let stem = name.trim_end_matches(".mdb");
                let after_prefix = stem.strip_prefix(&prefix_lower)?;
                let num_str = if suffix_lower.is_empty() {
                    after_prefix
                } else {
                    after_prefix.strip_suffix(&suffix_lower)?
                };
                let val = num_str.parse::<i32>().ok();
                if let Some(v) = val {
                    debug!("discover_variants: found variant {} from name {}", v, stem);
                }
                val
            })
            .collect();
        variants.sort_unstable();
        variants.dedup();
        variants
    }

    fn resolve_item_prefix(
        row: &ahash::AHashMap<String, Option<String>>,
    ) -> (String, &'static str) {
        if let Some(ic) = row_str(row, "itemclass") {
            return (ic, "itemclass");
        }
        if let Some(lb) = row_str(row, "label") {
            return (lb, "label");
        }
        if let Some(mt) = row_str(row, "modeltype") {
            return (mt, "modeltype");
        }
        (String::new(), "none")
    }

    /// Resolve the resref for a single weapon part (0=a, 1=b, 2=c) at a given variant number.
    /// Returns None if the base item has no usable prefix.
    pub fn resolve_weapon_part_resref(
        base_item_id: i32,
        part_index: usize,
        variant: i32,
        game_data: &GameData,
    ) -> Option<String> {
        let letter = match part_index {
            0 => 'a',
            1 => 'b',
            2 => 'c',
            _ => return None,
        };
        let table = game_data.get_table("baseitems")?;
        let row = table.get_by_id(base_item_id)?;
        let (prefix, _) = Self::resolve_item_prefix(&row);
        if prefix.is_empty() {
            return None;
        }
        let full_prefix = normalize_weapon_prefix(&prefix);
        Some(format!("{full_prefix}{variant:02}_{letter}"))
    }

    /// Resolve model resrefs as a list of "slot groups".
    ///
    /// Outer list = independent model parts that all need to load (e.g. the
    /// three blade/hilt/pommel pieces of a sword, or the body+boots+gloves of
    /// a chest outfit). Inner list = ordered fallback candidates for that
    /// slot — the loader tries them in order and stops at the first success.
    /// This prevents material-fallback meshes (LE/CL/CH/BA/PF for helmets
    /// and nested armor parts) from rendering on top of each other.
    ///
    /// `body_prefix` controls which race/gender body the armor sits on —
    /// pass the loaded character's prefix (e.g. `P_EEM` for elf male) so the
    /// preview matches in-game; defaults to `DEFAULT_BODY_PREFIX` when None.
    pub fn resolve_model_resrefs(
        &self,
        base_item_id: i32,
        game_data: &GameData,
        body_prefix: Option<&str>,
    ) -> Vec<Vec<String>> {
        let Some(table) = game_data.get_table("baseitems") else {
            return Vec::new();
        };
        let Some(row) = table.get_by_id(base_item_id) else {
            return Vec::new();
        };

        let (prefix, source) = Self::resolve_item_prefix(&row);
        let kind = classify_model_type(&row_str(&row, "modeltype").unwrap_or_default());

        // Bracer base items ship with modeltype=0 (weapon) in baseitems.2da
        // but render as a fixed glove mesh (always variant 01) whose
        // material comes from the item's `ArmorVisualType` via `armor.2da`.
        // `Variation`/`ModelPart1` on these items is the bracer style index
        // (for icons/props), not a mesh number.
        let is_bracer = is_bracer_label(&prefix);
        if is_bracer {
            let body = body_prefix.unwrap_or(DEFAULT_BODY_PREFIX);
            let item_armor_prefix = self.armor_visual_type.and_then(|vt| {
                resolve_armor_prefix(game_data, vt, false)
                    .into_iter()
                    .next()
            });
            let primaries: Vec<&str> = item_armor_prefix.iter().map(String::as_str).collect();
            info!(
                "Resolving bracer item {base_item_id}: body={body}, avt_prefix={item_armor_prefix:?}"
            );
            let candidates = armor_resref_candidates(body, ArmorSlot::Gloves, 1, &primaries, true);
            if candidates.is_empty() {
                return Vec::new();
            }
            return vec![candidates];
        }

        if kind == ItemModelKind::BodyArmor {
            let equip_slots =
                parse_equip_slots(&row_str(&row, "equipableslots").unwrap_or_default());
            let slot = detect_armor_slot(equip_slots);
            let item_armor_prefix = self.armor_visual_type.and_then(|vt| {
                resolve_armor_prefix(game_data, vt, false)
                    .into_iter()
                    .next()
            });

            info!(
                "Resolving armor item {base_item_id}: slot={slot:?}, item_armor_prefix={item_armor_prefix:?}, equip_slots=0x{equip_slots:04x}, boots={:?}, gloves={:?}",
                self.boots, self.gloves
            );

            let body = body_prefix.unwrap_or(DEFAULT_BODY_PREFIX);
            let mut groups = build_armor_resrefs(
                body,
                slot,
                item_armor_prefix.as_deref(),
                self.variation,
                self.model_parts[0],
            );

            // NWN2 stores nested boots/gloves + 22 accessory slots inline
            // on the chest item; emit them so the viewer renders the
            // complete outfit.
            if slot == Some(ArmorSlot::Body) {
                if let Some(g) = nested_part_group(
                    body,
                    self.boots.as_ref(),
                    ArmorSlot::Boots,
                    game_data,
                    item_armor_prefix.as_deref(),
                ) {
                    groups.push(g);
                }
                if let Some(g) = nested_part_group(
                    body,
                    self.gloves.as_ref(),
                    ArmorSlot::Gloves,
                    game_data,
                    item_armor_prefix.as_deref(),
                ) {
                    groups.push(g);
                }
                groups.extend(accessory_resref_groups(body, &self.accessories));
            }

            return groups;
        }

        info!(
            "Resolving weapon item {base_item_id}: prefix='{prefix}' from {source}, kind={kind:?}"
        );

        build_weapon_resrefs(kind, &prefix, self.variation, self.model_parts)
    }
}

/// Pure weapon resref builder. Returns one group per independent part.
///
/// - `ThreePartWeapon` → three groups (_a, _b, _c) that merge, plus a
///   separate single-part fallback group for items like `magicstaff` that
///   are tagged `modeltype=2` but ship as one merged `.mdb`. Each group is
///   tried independently; the fallback only contributes meshes if it resolves
///   to a file that exists (and most 3-part items have no corresponding
///   single-part file, so it's a no-op).
/// - `SinglePart` (shields, crossbows, etc.) → one group whose primary
///   candidate uses `ModelPart1` — real data shows these items store the
///   variant index there, not in `Variation`. Falls back to `Variation` if
///   `ModelPart1` is unset.
fn build_weapon_resrefs(
    kind: ItemModelKind,
    base_prefix: &str,
    variation: i32,
    model_parts: [i32; 3],
) -> Vec<Vec<String>> {
    if base_prefix.is_empty() {
        return Vec::new();
    }
    match kind {
        ItemModelKind::ThreePartWeapon => {
            let full = normalize_weapon_prefix(base_prefix);
            let part = |n: i32, letter: char| {
                if n > 0 {
                    vec![format!("{full}{n:02}_{letter}")]
                } else {
                    Vec::new()
                }
            };
            let mut groups = Vec::new();
            let g_a = part(model_parts[0], 'a');
            let g_b = part(model_parts[1], 'b');
            let g_c = part(model_parts[2], 'c');
            if !g_a.is_empty() {
                groups.push(g_a);
            }
            if !g_b.is_empty() {
                groups.push(g_b);
            }
            if !g_c.is_empty() {
                groups.push(g_c);
            }
            // Single-part fallback for items like magicstaff. Uses the first
            // positive value from ModelPart1 then Variation as the variant nn.
            let fallback_nn = [model_parts[0], variation]
                .into_iter()
                .find(|&v| v > 0)
                .unwrap_or(0);
            if fallback_nn > 0 {
                groups.push(vec![format!("{full}{fallback_nn:02}")]);
            }
            groups
        }
        ItemModelKind::SinglePart => {
            let full = normalize_weapon_prefix(base_prefix);
            // Shields and similar modeltype=0 items store the variant in
            // ModelPart1 (e.g. `w_she_towr03` with ModelPart1=3). Fall back
            // to Variation only if ModelPart1 is unset.
            let nn = [model_parts[0], variation]
                .into_iter()
                .find(|&v| v > 0)
                .unwrap_or(0);
            if nn > 0 {
                vec![vec![format!("{full}{nn:02}")]]
            } else {
                Vec::new()
            }
        }
        ItemModelKind::BodyArmor | ItemModelKind::None => Vec::new(),
    }
}

/// Pure armor resref builder. Returns one group per armor slot.
///
/// Each group is an ordered list of fallback candidates — the loader uses
/// the first one that resolves to an existing MDB. The viewer has no wearer
/// context, so we stamp a default race/gender body prefix and try the item's
/// own armor prefix plus common material fallbacks where relevant.
///
/// NWN2 armor file pattern: `{body_prefix}_{armor_prefix}_{Part}{NN}.mdb`
/// e.g. `P_HHM_LE_Body01.mdb`, `P_HHM_LE_Helm05.mdb`, `P_HHM_CL_Cloak01.mdb`.
///
/// GFF `Variation` is 0-indexed (the toolset shows "Variation 0, 1, 2, ..."),
/// while mesh filenames are 1-indexed (Body01 is the first variant). The
/// engine adds 1 at render time; we do the same. `ArmorVisualType` and
/// `ModelPart1` are *not* adjusted — real data shows those match the filename
/// number directly.
fn build_armor_resrefs(
    body_prefix: &str,
    slot: Option<ArmorSlot>,
    item_armor_prefix: Option<&str>,
    variation: i32,
    model_part1: i32,
) -> Vec<Vec<String>> {
    let Some(slot) = slot else {
        return Vec::new();
    };

    let variation_nn = variation + 1;

    let candidates = match slot {
        ArmorSlot::Body => match item_armor_prefix {
            Some(pfx) => armor_resref_candidates(body_prefix, slot, variation_nn, &[pfx], false),
            None => Vec::new(),
        },
        ArmorSlot::Boots | ArmorSlot::Gloves => match item_armor_prefix {
            Some(pfx) => armor_resref_candidates(body_prefix, slot, variation_nn, &[pfx], false),
            None => Vec::new(),
        },
        ArmorSlot::Head => match item_armor_prefix {
            Some(pfx) => armor_resref_candidates(body_prefix, slot, variation_nn, &[pfx], false),
            None => armor_resref_candidates(body_prefix, slot, variation_nn, &[], true),
        },
        ArmorSlot::Cloak => {
            // Cloak uses the hardcoded `CL` armor prefix. Real data shows the
            // variant lives in `Variation` (0-indexed → +1); `ModelPart1` is
            // a fallback for items that only store it there, and it's
            // 1-indexed so we use it directly.
            let nn = if variation > 0 {
                variation_nn
            } else if model_part1 > 0 {
                model_part1
            } else {
                0
            };
            armor_resref_candidates(body_prefix, slot, nn, &["CL"], false)
        }
    };

    if candidates.is_empty() {
        Vec::new()
    } else {
        vec![candidates]
    }
}

/// Emits `{body_prefix}_{prefix}_{Part}{nn:02}` for each primary
/// prefix, optionally followed by the common material fallbacks. Fallbacks
/// are only correct for slots where the primary can't identify the material
/// (e.g. helmets, which take the wearer's chest material in-game).
fn armor_resref_candidates(
    body_prefix: &str,
    slot: ArmorSlot,
    nn: i32,
    primary_prefixes: &[&str],
    include_material_fallbacks: bool,
) -> Vec<String> {
    if nn <= 0 {
        return Vec::new();
    }
    let part = slot.part_name();
    let mut out = Vec::new();
    let mut push = |pfx: &str| {
        let s = format!("{body_prefix}_{pfx}_{part}{nn:02}");
        if !out.contains(&s) {
            out.push(s);
        }
    };
    for p in primary_prefixes {
        push(p);
    }
    if include_material_fallbacks {
        for p in FALLBACK_ARMOR_PREFIXES {
            push(p);
        }
    }
    out
}

fn normalize_weapon_prefix(prefix: &str) -> String {
    if prefix.to_uppercase().starts_with("W_") {
        prefix.to_string()
    } else {
        format!("W_{prefix}")
    }
}

/// Read the 22 `AC*` struct fields from a UTI-style GFF map into an
/// `ArmorAccessories`. Missing fields or non-struct values leave the slot
/// as `None`. Present structs contribute `{ id, tints }`; `id = 0` is
/// preserved so the original UTI can be reconstructed on write.
fn read_accessories(fields: &IndexMap<String, GffValue<'_>>) -> ArmorAccessories {
    let mut out = ArmorAccessories::default();
    for slot in AccessorySlot::all() {
        let Some(field) = fields.get(slot.uti_field()) else {
            continue;
        };
        let sub = match field {
            GffValue::StructOwned(s) => s.as_ref().clone(),
            GffValue::Struct(lazy) => lazy.force_load(),
            _ => continue,
        };
        let Some(id) = sub.get("Accessory").and_then(gff_value_to_i32) else {
            continue;
        };
        // Per-slot Tintable: each AC struct can override colours so a single
        // armor item can have (e.g.) black pauldrons and steel bracers.
        let tints = sub
            .get("Tintable")
            .and_then(|v| match v {
                GffValue::StructOwned(s) => Some(s.as_ref().clone()),
                GffValue::Struct(lazy) => Some(lazy.force_load()),
                _ => None,
            })
            .map(|t| read_tint_from_tintable(&t))
            .unwrap_or_default();
        out.set_data(*slot, Some(AccessorySlotData { id, tints }));
    }
    out
}

pub(crate) fn read_nested_armor_part(
    fields: &IndexMap<String, GffValue<'_>>,
    key: &str,
) -> Option<NestedArmorPart> {
    let part_fields = match fields.get(key)? {
        GffValue::StructOwned(s) => s.as_ref().clone(),
        GffValue::Struct(lazy) => lazy.force_load(),
        _ => return None,
    };

    let variation = part_fields
        .get("Variation")
        .and_then(gff_value_to_i32)
        .unwrap_or(0);
    if variation < 0 {
        return None;
    }
    let armor_visual_type = part_fields
        .get("ArmorVisualType")
        .and_then(gff_value_to_i32);

    Some(NestedArmorPart {
        armor_visual_type,
        variation,
        tints: match part_fields.get("ArmorTint") {
            Some(GffValue::StructOwned(s)) => {
                Some(super::appearance_helpers::read_tint_channels(s))
            }
            Some(GffValue::Struct(lazy)) => Some(super::appearance_helpers::read_tint_channels(
                &lazy.force_load(),
            )),
            _ => None,
        },
    })
}

/// Build the ordered fallback group for a nested Boots/Gloves sub-part of a
/// body-armour item. The part's own `ArmorVisualType` can differ from the
/// chest's, and the mesh may only exist for one of them (or under a neutral
/// material), so we try part-prefix first, then chest-prefix, then the
/// common material fallbacks. The loader uses the first existing file —
/// previously the entire list was merged, producing overlapping meshes.
/// Parse an accessory resref (`A_EEM_LShoulder15`) back to the skeleton
/// bone its rigid mesh should be parented to, plus the `AccessorySlot` so
/// callers can look up per-slot data (tints, etc.). Returns `None` for
/// non-accessory resrefs or unrecognized slot fragments.
pub fn resref_attach_bone_and_slot(resref: &str) -> Option<(&'static str, AccessorySlot)> {
    let rest = resref
        .strip_prefix("A_")
        .or_else(|| resref.strip_prefix("a_"))?;
    let second_underscore = rest.find('_')?;
    let after_body = &rest[second_underscore + 1..];
    let slot_len = after_body
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(after_body.len());
    if slot_len == 0 {
        return None;
    }
    let slot_str = &after_body[..slot_len];
    let slot = AccessorySlot::from_mdb_slot(slot_str)?;
    let bone = slot.attach_bone()?;
    Some((bone, slot))
}

/// MDB resref of one accessory mesh: `A_{body}_{slot}{id}` with the body
/// prefix's `P_` stripped and the id NOT zero-padded (`A_HHM_LShoulder1`),
/// which is how every accessory file on disk is named. `None` for id 0 and
/// vestigial slots.
pub fn accessory_resref(body_prefix: &str, slot: AccessorySlot, id: i32) -> Option<String> {
    if id <= 0 {
        return None;
    }
    let body = body_prefix
        .strip_prefix("P_")
        .or_else(|| body_prefix.strip_prefix("p_"))
        .unwrap_or(body_prefix);
    let mdb_slot = slot.mdb_slot()?;
    Some(format!("A_{body}_{mdb_slot}{id}"))
}

/// Build one resref group per non-zero, non-vestigial accessory slot.
/// Each group has a single candidate (no fallback list). Missing MDBs are
/// skipped at load time by `load_item_model`'s fallback handling.
fn accessory_resref_groups(body_prefix: &str, accessories: &ArmorAccessories) -> Vec<Vec<String>> {
    accessories
        .iter_renderable(body_prefix)
        .map(|a| vec![a.resref])
        .collect()
}

fn nested_part_group(
    body_prefix: &str,
    part: Option<&NestedArmorPart>,
    slot: ArmorSlot,
    game_data: &GameData,
    chest_armor_prefix: Option<&str>,
) -> Option<Vec<String>> {
    let part = part?;
    let part_prefix = part.armor_visual_type.and_then(|vt| {
        resolve_armor_prefix(game_data, vt, false)
            .into_iter()
            .next()
    });

    let primaries: Vec<&str> = [part_prefix.as_deref(), chest_armor_prefix]
        .into_iter()
        .flatten()
        .collect();
    // Nested part's `Variation` is 0-indexed like the top-level chest one.
    let candidates =
        armor_resref_candidates(body_prefix, slot, part.variation + 1, &primaries, true);
    if candidates.is_empty() {
        None
    } else {
        Some(candidates)
    }
}

impl ItemAppearanceOptions {
    fn new_empty() -> Self {
        Self {
            available_variations: Vec::new(),
            available_materials: Vec::new(),
            available_part1: Vec::new(),
            available_part2: Vec::new(),
            available_part3: Vec::new(),
            has_model: false,
            variation_field: VariationField::Variation,
            preview_pose: PreviewPose::Weapon,
            available_accessories: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only: set an accessory slot to an id with default (zero) tints.
    fn set_id(acc: &mut ArmorAccessories, slot: AccessorySlot, id: Option<i32>) {
        acc.set_data(
            slot,
            id.map(|id| AccessorySlotData {
                id,
                tints: TintChannels::default(),
            }),
        );
    }

    /// Test-only: equivalent of the old `resref_attach_bone` helper.
    fn resref_bone(resref: &str) -> Option<&'static str> {
        resref_attach_bone_and_slot(resref).map(|(b, _)| b)
    }

    #[test]
    fn accessory_resref_groups_emits_one_group_per_nonzero_rendering_slot() {
        let mut acc = ArmorAccessories::default();
        set_id(&mut acc, AccessorySlot::LtShoulder, Some(15));
        set_id(&mut acc, AccessorySlot::RtShoulder, Some(15));
        set_id(&mut acc, AccessorySlot::LtArm, Some(14));
        set_id(&mut acc, AccessorySlot::RtArm, Some(12));

        let groups = accessory_resref_groups("P_EEM", &acc);
        let flat: Vec<String> = groups.iter().flatten().cloned().collect();

        assert!(flat.contains(&"A_EEM_LShoulder15".to_string()));
        assert!(flat.contains(&"A_EEM_RShoulder15".to_string()));
        assert!(flat.contains(&"A_EEM_LUpArm14".to_string()));
        assert!(flat.contains(&"A_EEM_RUpArm12".to_string()));
        assert_eq!(
            groups.len(),
            4,
            "each accessory is its own single-candidate group"
        );
        for g in &groups {
            assert_eq!(g.len(), 1, "no fallbacks — one candidate per group");
        }
    }

    #[test]
    fn accessory_resref_groups_skips_zero_and_missing_and_vestigial() {
        let mut acc = ArmorAccessories::default();
        set_id(&mut acc, AccessorySlot::LtShoulder, Some(0));
        set_id(&mut acc, AccessorySlot::RtShoulder, None);
        set_id(&mut acc, AccessorySlot::LtAnkle, Some(5));
        set_id(&mut acc, AccessorySlot::BkHip, Some(9));
        set_id(&mut acc, AccessorySlot::LtKnee, Some(3));

        let groups = accessory_resref_groups("P_EEM", &acc);
        let flat: Vec<String> = groups.iter().flatten().cloned().collect();

        assert_eq!(flat, vec!["A_EEM_LKnee3".to_string()]);
    }

    #[test]
    fn accessory_resref_groups_strips_body_p_prefix() {
        let mut acc = ArmorAccessories::default();
        set_id(&mut acc, AccessorySlot::LtShoulder, Some(1));
        // Uppercase input (the real case — body_prefix comes from appearance.2da).
        assert_eq!(
            accessory_resref_groups("P_EEM", &acc),
            vec![vec!["A_EEM_LShoulder1".to_string()]]
        );
        // Lowercase input strips defensively; casing preserved. The resource
        // manager lookup is case-insensitive, so both resolve to the same file.
        assert_eq!(
            accessory_resref_groups("p_hhm", &acc),
            vec![vec!["A_hhm_LShoulder1".to_string()]]
        );
        // Body without P_ prefix is used as-is.
        assert_eq!(
            accessory_resref_groups("HHM", &acc),
            vec![vec!["A_HHM_LShoulder1".to_string()]]
        );
    }

    fn make_ac_struct(accessory: i32) -> GffValue<'static> {
        let mut inner = IndexMap::new();
        inner.insert("Accessory".to_string(), GffValue::Byte(accessory as u8));
        GffValue::StructOwned(Box::new(inner))
    }

    #[test]
    fn from_gff_parses_all_non_vestigial_accessory_values() {
        let mut fields: IndexMap<String, GffValue<'static>> = IndexMap::new();
        fields.insert("ACLtShoulder".into(), make_ac_struct(15));
        fields.insert("ACRtShoulder".into(), make_ac_struct(15));
        fields.insert("ACLtArm".into(), make_ac_struct(14));
        fields.insert("ACRtArm".into(), make_ac_struct(12));
        fields.insert("ACLtBracer".into(), make_ac_struct(17));
        fields.insert("ACRtBracer".into(), make_ac_struct(17));
        fields.insert("ACLtShin".into(), make_ac_struct(11));
        fields.insert("ACRtShin".into(), make_ac_struct(11));
        fields.insert("ACLtLeg".into(), make_ac_struct(14));
        fields.insert("ACRtKnee".into(), make_ac_struct(11));

        let appearance = ItemAppearance::from_gff(&fields);

        assert_eq!(
            appearance.accessories.get(AccessorySlot::LtShoulder),
            Some(15)
        );
        assert_eq!(
            appearance.accessories.get(AccessorySlot::RtShoulder),
            Some(15)
        );
        assert_eq!(appearance.accessories.get(AccessorySlot::LtArm), Some(14));
        assert_eq!(appearance.accessories.get(AccessorySlot::RtArm), Some(12));
        assert_eq!(
            appearance.accessories.get(AccessorySlot::LtBracer),
            Some(17)
        );
        assert_eq!(
            appearance.accessories.get(AccessorySlot::RtBracer),
            Some(17)
        );
        assert_eq!(appearance.accessories.get(AccessorySlot::LtShin), Some(11));
        assert_eq!(appearance.accessories.get(AccessorySlot::RtShin), Some(11));
        assert_eq!(appearance.accessories.get(AccessorySlot::LtLeg), Some(14));
        assert_eq!(appearance.accessories.get(AccessorySlot::RtKnee), Some(11));
        assert_eq!(appearance.accessories.get(AccessorySlot::LtElbow), None);
        assert_eq!(appearance.accessories.get(AccessorySlot::RtLeg), None);
    }

    #[test]
    fn from_gff_parses_vestigial_slots_for_round_trip() {
        let mut fields: IndexMap<String, GffValue<'static>> = IndexMap::new();
        fields.insert("ACLtAnkle".into(), make_ac_struct(0));
        fields.insert("ACBkHip".into(), make_ac_struct(0));
        fields.insert("ACFtHip".into(), make_ac_struct(7));

        let appearance = ItemAppearance::from_gff(&fields);
        assert_eq!(appearance.accessories.get(AccessorySlot::LtAnkle), Some(0));
        assert_eq!(appearance.accessories.get(AccessorySlot::BkHip), Some(0));
        assert_eq!(appearance.accessories.get(AccessorySlot::FtHip), Some(7));
    }

    #[test]
    fn from_gff_tolerates_malformed_accessory_fields() {
        let mut fields: IndexMap<String, GffValue<'static>> = IndexMap::new();
        fields.insert("ACLtShoulder".into(), GffValue::Byte(9));

        let appearance = ItemAppearance::from_gff(&fields);
        assert_eq!(appearance.accessories.get(AccessorySlot::LtShoulder), None);
    }

    #[test]
    fn item_appearance_default_has_empty_accessories() {
        let fields: IndexMap<String, GffValue<'_>> = IndexMap::new();
        let appearance = ItemAppearance::from_gff(&fields);
        for slot in AccessorySlot::all() {
            assert_eq!(
                appearance.accessories.get(*slot),
                None,
                "{slot:?} should be None when UTI has no AC fields"
            );
        }
    }

    #[test]
    fn armor_accessories_get_and_set_round_trip_every_slot() {
        let mut acc = ArmorAccessories::default();
        for slot in AccessorySlot::all() {
            assert_eq!(acc.get(*slot), None, "default {slot:?} should be None");
        }
        for (i, slot) in AccessorySlot::all().iter().enumerate() {
            let val = Some((i + 1) as i32);
            set_id(&mut acc, *slot, val);
            assert_eq!(acc.get(*slot), val);
        }
        set_id(&mut acc, AccessorySlot::LtShoulder, Some(99));
        assert_eq!(acc.get(AccessorySlot::LtShoulder), Some(99));
        assert_eq!(acc.get(AccessorySlot::RtShoulder), Some(2));
    }

    #[test]
    fn accessory_slot_attach_bones_match_skeleton() {
        use AccessorySlot::*;
        // Spot-check the full rendering mapping; every non-vestigial slot
        // must point at a bone that exists in P_HHM_skel.
        let cases: &[(AccessorySlot, &str)] = &[
            (LtShoulder, "LArm010"),
            (RtShoulder, "RArm110"),
            (LtArm, "LArm010"),
            (RtArm, "RArm110"),
            (LtElbow, "LArm011"),
            (RtElbow, "RArm111"),
            (LtBracer, "LArm02"),
            (RtBracer, "RArm12"),
            (LtLeg, "LLeg1"),
            (RtLeg, "RLeg1"),
            (LtKnee, "LLeg2"),
            (RtKnee, "RLeg2"),
            (LtShin, "LLeg2"),
            (RtShin, "RLeg2"),
            (LtHip, "LHip1"),
            (RtHip, "RHip1"),
        ];
        for (slot, bone) in cases {
            assert_eq!(slot.attach_bone(), Some(*bone), "bone for {slot:?}");
        }
        for vestigial in [LtAnkle, RtAnkle, LtFoot, RtFoot, BkHip, FtHip] {
            assert_eq!(vestigial.attach_bone(), None);
        }
    }

    #[test]
    fn resref_attach_bone_parses_real_accessory_resrefs() {
        assert_eq!(resref_bone("A_EEM_LShoulder15"), Some("LArm010"));
        assert_eq!(resref_bone("A_HHM_RShoulder15"), Some("RArm110"));
        assert_eq!(resref_bone("A_EEM_LUpArm14"), Some("LArm010"));
        assert_eq!(resref_bone("A_EEM_RUpArm12"), Some("RArm110"));
        assert_eq!(resref_bone("A_EEM_LBracer17"), Some("LArm02"));
        assert_eq!(resref_bone("A_EEM_RKnee11"), Some("RLeg2"));
        assert_eq!(resref_bone("A_EEM_LLowLeg11"), Some("LLeg2"));
        // Non-accessory resrefs → None.
        assert_eq!(resref_bone("P_EEM_PF_Body03"), None);
        assert_eq!(resref_bone("W_LSword01_a"), None);
        // Unknown slot fragment → None.
        assert_eq!(resref_bone("A_EEM_WeirdSlot01"), None);
    }

    #[test]
    fn accessory_slot_all_has_22_entries_and_no_duplicates() {
        use std::collections::HashSet;
        let all = AccessorySlot::all();
        assert_eq!(all.len(), 22);
        let set: HashSet<_> = all.iter().copied().collect();
        assert_eq!(
            set.len(),
            22,
            "AccessorySlot::all() must not contain duplicates"
        );
    }

    #[test]
    fn accessory_slot_uti_field_and_mdb_slot_mappings() {
        let cases: &[(AccessorySlot, &str, Option<&str>)] = &[
            (AccessorySlot::LtShoulder, "ACLtShoulder", Some("LShoulder")),
            (AccessorySlot::RtShoulder, "ACRtShoulder", Some("RShoulder")),
            (AccessorySlot::LtArm, "ACLtArm", Some("LUpArm")),
            (AccessorySlot::RtArm, "ACRtArm", Some("RUpArm")),
            (AccessorySlot::LtElbow, "ACLtElbow", Some("LElbow")),
            (AccessorySlot::RtElbow, "ACRtElbow", Some("RElbow")),
            (AccessorySlot::LtBracer, "ACLtBracer", Some("LBracer")),
            (AccessorySlot::RtBracer, "ACRtBracer", Some("RBracer")),
            (AccessorySlot::LtLeg, "ACLtLeg", Some("LUpLeg")),
            (AccessorySlot::RtLeg, "ACRtLeg", Some("RUpLeg")),
            (AccessorySlot::LtKnee, "ACLtKnee", Some("LKnee")),
            (AccessorySlot::RtKnee, "ACRtKnee", Some("RKnee")),
            (AccessorySlot::LtShin, "ACLtShin", Some("LLowLeg")),
            (AccessorySlot::RtShin, "ACRtShin", Some("RLowLeg")),
            (AccessorySlot::LtHip, "ACLtHip", Some("LHip")),
            (AccessorySlot::RtHip, "ACRtHip", Some("RHip")),
            (AccessorySlot::LtAnkle, "ACLtAnkle", None),
            (AccessorySlot::RtAnkle, "ACRtAnkle", None),
            (AccessorySlot::LtFoot, "ACLtFoot", None),
            (AccessorySlot::RtFoot, "ACRtFoot", None),
            (AccessorySlot::BkHip, "ACBkHip", None),
            (AccessorySlot::FtHip, "ACFtHip", None),
        ];
        for (slot, uti, mdb) in cases {
            assert_eq!(slot.uti_field(), *uti, "uti_field for {slot:?}");
            assert_eq!(slot.mdb_slot(), *mdb, "mdb_slot for {slot:?}");
        }
        assert_eq!(cases.len(), AccessorySlot::all().len());
    }

    #[test]
    fn classifies_nwn2_modeltype_values() {
        assert_eq!(classify_model_type("2"), ItemModelKind::ThreePartWeapon);
        assert_eq!(classify_model_type("0"), ItemModelKind::SinglePart);
        assert_eq!(classify_model_type("3"), ItemModelKind::BodyArmor);
        assert_eq!(classify_model_type(""), ItemModelKind::None);
        assert_eq!(classify_model_type("   "), ItemModelKind::None);
        assert_eq!(classify_model_type("A"), ItemModelKind::None);
    }

    fn flat(groups: &[Vec<String>]) -> Vec<String> {
        groups.iter().flatten().cloned().collect()
    }

    #[test]
    fn three_part_weapon_emits_abc_as_separate_groups_plus_single_part_fallback_group() {
        let r = build_weapon_resrefs(ItemModelKind::ThreePartWeapon, "W_LSword", 0, [1, 2, 3]);
        // Each part is its own group (they all merge). Single-part fallback
        // is a separate group tried after — only contributes a mesh if the
        // file happens to exist on disk (magicstaff case).
        assert_eq!(r.len(), 4);
        assert_eq!(r[0], vec!["W_LSword01_a".to_string()]);
        assert_eq!(r[1], vec!["W_LSword02_b".to_string()]);
        assert_eq!(r[2], vec!["W_LSword03_c".to_string()]);
        // Fallback uses ModelPart1 when Variation is 0.
        assert_eq!(r[3], vec!["W_LSword01".to_string()]);
    }

    #[test]
    fn three_part_weapon_w_prefixes_bare_label() {
        let r = build_weapon_resrefs(ItemModelKind::ThreePartWeapon, "Axe", 0, [1, 1, 1]);
        assert_eq!(r[0], vec!["W_Axe01_a".to_string()]);
        assert_eq!(r[3], vec!["W_Axe01".to_string()]);
    }

    #[test]
    fn magicstaff_single_part_file_reachable_via_fallback_group() {
        // magicstaff has modeltype=2 but the real file is `w_mstaff01.mdb`.
        let r = build_weapon_resrefs(ItemModelKind::ThreePartWeapon, "w_mstaff", 0, [1, 1, 1]);
        assert!(flat(&r).contains(&"w_mstaff01".to_string()));
    }

    #[test]
    fn three_part_weapon_zero_model_parts_skipped() {
        // A weapon with no real ModelPart values produces no groups.
        let r = build_weapon_resrefs(ItemModelKind::ThreePartWeapon, "W_LSword", 0, [0, 0, 0]);
        assert!(r.is_empty());
    }

    #[test]
    fn single_part_shield_uses_model_part1_not_variation() {
        // Real NWN2 data: shields store the variant in ModelPart1, not Variation.
        let r = build_weapon_resrefs(ItemModelKind::SinglePart, "w_she_towr", 0, [3, 0, 0]);
        assert_eq!(r, vec![vec!["w_she_towr03".to_string()]]);
    }

    #[test]
    fn single_part_falls_back_to_variation_when_model_part1_zero() {
        let r = build_weapon_resrefs(ItemModelKind::SinglePart, "w_crsbL", 1, [0, 0, 0]);
        assert_eq!(r, vec![vec!["w_crsbL01".to_string()]]);
    }

    #[test]
    fn single_part_preserves_mixed_case_label() {
        let r = build_weapon_resrefs(ItemModelKind::SinglePart, "w_she_large", 0, [3, 0, 0]);
        assert_eq!(r, vec![vec!["w_she_large03".to_string()]]);
    }

    #[test]
    fn single_part_with_no_variant_at_all_emits_nothing() {
        let r = build_weapon_resrefs(ItemModelKind::SinglePart, "W_Arrow", 0, [0, 0, 0]);
        assert!(r.is_empty());
    }

    #[test]
    fn empty_base_prefix_skips_weapon_resolution() {
        let three = build_weapon_resrefs(ItemModelKind::ThreePartWeapon, "", 1, [1, 1, 1]);
        let single = build_weapon_resrefs(ItemModelKind::SinglePart, "", 1, [0, 0, 0]);
        assert!(three.is_empty());
        assert!(single.is_empty());
    }

    #[test]
    fn body_armor_uses_armor_prefix_and_body_suffix_with_one_based_filename() {
        // GFF Variation=7 is 0-indexed → mesh is Body08 (1-indexed filename).
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Body), Some("LE"), 7, 0);
        assert_eq!(r, vec![vec!["P_HHM_LE_Body08".to_string()]]);
    }

    #[test]
    fn body_armor_variation_zero_maps_to_first_mesh() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Body), Some("LE"), 0, 0);
        assert_eq!(r, vec![vec!["P_HHM_LE_Body01".to_string()]]);
    }

    #[test]
    fn body_armor_without_resolved_prefix_emits_nothing() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Body), None, 1, 0);
        assert!(r.is_empty());
    }

    #[test]
    fn helmet_uses_item_armor_prefix_and_variation_plus_one() {
        // Helm of Darkness: ArmorVisualType=7 → PH (Half-Plate), Variation=9 → Helm10.
        // Confirmed against in-game render.
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Head), Some("PH"), 9, 0);
        assert_eq!(r, vec![vec!["P_HHM_PH_Helm10".to_string()]]);
    }

    #[test]
    fn helmet_without_armor_prefix_falls_back_to_material_list() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Head), None, 4, 0);
        assert_eq!(r.len(), 1);
        let g = &r[0];
        assert!(g.contains(&"P_HHM_LE_Helm05".to_string()));
        assert!(g.contains(&"P_HHM_CL_Helm05".to_string()));
    }

    #[test]
    fn standalone_boots_use_only_item_prefix() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Boots), Some("LE"), 3, 0);
        assert_eq!(r, vec![vec!["P_HHM_LE_Boots04".to_string()]]);
    }

    #[test]
    fn standalone_gloves_use_only_item_prefix() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Gloves), Some("CH"), 2, 0);
        assert_eq!(r, vec![vec!["P_HHM_CH_Gloves03".to_string()]]);
    }

    /// `discover_armor_variants` reports the file `…Boots01` as the value `0`,
    /// because the loader renders `variation + 1`. If these two ever disagree
    /// the gallery offers a variant that renders nothing (and an item stored
    /// with `Variation = 0` matches no tile at all).
    #[test]
    fn armor_variant_values_round_trip_to_their_filename() {
        for nn in 1..=9i32 {
            let discovered_value = nn - 1;
            let r = build_armor_resrefs(
                "P_HHM",
                Some(ArmorSlot::Boots),
                Some("LE"),
                discovered_value,
                0,
            );
            assert_eq!(r, vec![vec![format!("P_HHM_LE_Boots{nn:02}")]]);
        }
    }

    #[test]
    fn cloak_prefers_variation_over_model_part1() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Cloak), None, 2, 1);
        assert_eq!(r, vec![vec!["P_HHM_CL_Cloak03".to_string()]]);
    }

    #[test]
    fn cloak_falls_back_to_model_part1_when_variation_zero_and_mp1_set() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Cloak), None, 0, 4);
        assert_eq!(r, vec![vec!["P_HHM_CL_Cloak04".to_string()]]);
    }

    #[test]
    fn cloak_with_variation_zero_and_no_mp1_is_empty() {
        let r = build_armor_resrefs("P_HHM", Some(ArmorSlot::Cloak), None, 0, 0);
        assert!(r.is_empty());
    }

    #[test]
    fn armor_with_unknown_slot_is_empty() {
        let r = build_armor_resrefs("P_HHM", None, Some("LE"), 1, 0);
        assert!(r.is_empty());
    }

    #[test]
    fn parses_hex_and_decimal_equip_slots() {
        assert_eq!(parse_equip_slots("0x20002"), 0x20002);
        assert_eq!(parse_equip_slots("0X0001"), 0x0001);
        assert_eq!(parse_equip_slots("64"), 64);
        assert_eq!(parse_equip_slots(""), 0);
        assert_eq!(parse_equip_slots("garbage"), 0);
    }

    #[test]
    fn detects_armor_slots_from_real_baseitems_bitmasks() {
        // bitmasks copied from real baseitems.2da rows
        assert_eq!(detect_armor_slot(0x00001), Some(ArmorSlot::Head)); // helmet
        assert_eq!(detect_armor_slot(0x20002), Some(ArmorSlot::Body)); // armor (chest + creature-armor bit)
        assert_eq!(detect_armor_slot(0x00004), Some(ArmorSlot::Boots));
        assert_eq!(detect_armor_slot(0x00008), Some(ArmorSlot::Gloves));
        assert_eq!(detect_armor_slot(0x00040), Some(ArmorSlot::Cloak));
        assert_eq!(detect_armor_slot(0x00000), None);
        assert_eq!(detect_armor_slot(0x00200), None); // amulet (neck) — not armor
    }

    #[test]
    fn normalize_weapon_prefix_is_case_insensitive() {
        assert_eq!(normalize_weapon_prefix("W_Axe"), "W_Axe");
        assert_eq!(normalize_weapon_prefix("w_Lbow"), "w_Lbow");
        assert_eq!(normalize_weapon_prefix("Axe"), "W_Axe");
    }
}
