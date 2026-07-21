use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::parsers::gr2::Gr2Parser;
use crate::parsers::mdb::MdbParser;
use crate::parsers::mdb::types::{Material, RigidMeshPacket, SkinMeshPacket};
use crate::services::resource_manager::ResourceManager;

fn guess_body_skeleton(prefix: &str) -> Option<&'static str> {
    match prefix {
        "P_AAM_" | "P_ASM_" | "P_EDM_" | "P_EEM_" | "P_EHM_" | "P_ELM_" | "P_ERM_" | "P_ESM_"
        | "P_EWM_" | "P_HAM_" | "P_HEM_" | "P_HFM_" | "P_HHM_" | "P_HIM_" | "P_HPM_" | "P_HTM_"
        | "P_HWM_" => Some("P_HHM_skel"),

        "P_AAF_" | "P_ASF_" | "P_EDF_" | "P_EEF_" | "P_EHF_" | "P_ELF_" | "P_ERF_" | "P_ESF_"
        | "P_EWF_" | "P_HAF_" | "P_HEF_" | "P_HFF_" | "P_HHF_" | "P_HIF_" | "P_HPF_" | "P_HTF_"
        | "P_HWF_" => Some("P_HHF_skel"),

        "P_DDM_" | "P_DGM_" | "P_DUM_" => Some("P_DDM_skel"),
        "P_DDF_" | "P_DGF_" | "P_DUF_" => Some("P_DDF_skel"),

        "P_GGM_" | "P_GSM_" => Some("P_GGM_skel"),
        "P_GGF_" | "P_GSF_" => Some("P_GGF_skel"),

        "P_OOM_" | "P_OGM_" => Some("P_OOM_skel"),
        "P_OOF_" | "P_OGF_" => Some("P_OOF_skel"),

        _ => None,
    }
}

fn guess_cloak_skeleton(prefix: &str) -> Option<&'static str> {
    match prefix {
        "P_DDM_" => Some("P_DDMcapewing_skel"),
        "P_DDF_" => Some("P_DDFcapewing_skel"),
        "P_EEM_" | "P_HHM_" => Some("P_HHMcapewing_skel"),
        "P_EEF_" | "P_HHF_" => Some("P_HHFcapewing_skel"),
        "P_GGM_" => Some("P_GGMcapewing_skel"),
        "P_GGF_" => Some("P_GGFcapewing_skel"),
        "P_OOM_" => Some("P_OOMcapewing_skel"),
        "P_OOF_" => Some("P_OOFcapewing_skel"),
        _ => None,
    }
}

fn guess_tail_skeleton(prefix: &str) -> Option<&'static str> {
    match prefix {
        "P_DDM_" => Some("P_DDMtail_skel"),
        "P_DDF_" => Some("P_DDFtail_skel"),
        "P_EEM_" | "P_HHM_" | "P_HTM_" => Some("P_HHMtail_skel"),
        "P_EEF_" | "P_HHF_" | "P_HTF_" => Some("P_HHFtail_skel"),
        "P_GGM_" => Some("P_GGMtail_skel"),
        "P_GGF_" => Some("P_GGFtail_skel"),
        "P_OOM_" => Some("P_OOMtail_skel"),
        "P_OOF_" => Some("P_OOFtail_skel"),
        _ => None,
    }
}

fn resolve_skeleton_name(mesh_name: &str, stored_skeleton: &str) -> String {
    let upper = mesh_name.to_uppercase();
    let prefix = if upper.len() >= 6 { &upper[..6] } else { "" };

    if (upper.contains("_CLOAK") || upper.contains("_WINGS"))
        && let Some(skel) = guess_cloak_skeleton(prefix)
    {
        return skel.to_string();
    }

    if upper.contains("_TAIL")
        && let Some(skel) = guess_tail_skeleton(prefix)
    {
        return skel.to_string();
    }

    if let Some(skel) = guess_body_skeleton(prefix) {
        return skel.to_string();
    }

    stored_skeleton.to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelData {
    pub meshes: Vec<MeshData>,
    pub hooks: Vec<HookData>,
    pub hair: Vec<HairData>,
    pub helm: Vec<HelmData>,
    pub skeleton: Option<SkeletonData>,
    pub animations: Vec<AnimationData>,
    #[serde(default)]
    pub attached_parts: Vec<AttachedPart>,
    #[serde(default)]
    pub secondary_skeletons: Vec<NamedSkeleton>,
}

/// A model segment rendered with its own skeleton (e.g. creature wings/tails)
/// and optionally parented to a body bone at `attach_bone`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedPart {
    pub name: String,
    pub meshes: Vec<MeshData>,
    pub skeleton: Option<SkeletonData>,
    pub animations: Vec<AnimationData>,
    pub attach_bone: Option<String>,
}

/// Secondary skeleton attached to the same animated model as the primary body
/// skeleton. Cape bones live here so they can be animated by the shared body
/// idle/fidget tracks without being part of the body bone palette.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedSkeleton {
    pub name: String,
    pub skeleton: SkeletonData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationData {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<TrackData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackData {
    pub bone_name: String,
    pub times: Vec<f32>,
    pub positions: Option<Vec<f32>>,
    pub rotations: Option<Vec<f32>>,
    pub scales: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    pub name: String,
    pub mesh_type: String,
    pub part: String,
    pub tint_group: String,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub tangents: Vec<f32>,
    pub indices: Vec<u16>,
    pub bone_weights: Option<Vec<f32>>,
    pub bone_indices: Option<Vec<u8>>,
    pub material: MaterialData,
    #[serde(default)]
    pub skeleton_ref: Option<String>,
    /// Skeleton bone this mesh should be parented to at render time.
    /// Used for rigid accessory meshes (pauldrons, bracers, greaves) that
    /// have no skin weights and instead ride a specific body bone.
    #[serde(default)]
    pub attach_bone: Option<String>,
    /// Per-mesh tint channels that override the top-level item tint.
    /// Set for armor accessories so each slot (pauldron, bracer, greave)
    /// can carry its own colour independently from the chest's tint.
    #[serde(default)]
    pub override_tints: Option<crate::character::appearance_helpers::TintChannels>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialData {
    pub diffuse_map: String,
    pub normal_map: String,
    pub tint_map: String,
    pub glow_map: String,
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletonData {
    pub bones: Vec<BoneData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneData {
    pub name: String,
    pub parent_index: i32,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub inverse_world_4x4: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookData {
    pub name: String,
    pub position: [f32; 3],
    pub orientation: [[f32; 3]; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HairData {
    pub name: String,
    pub shortening_behavior: u32,
    pub position: [f32; 3],
    pub orientation: [[f32; 3]; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmData {
    pub name: String,
    pub hiding_behavior: u32,
    pub position: [f32; 3],
    pub orientation: [[f32; 3]; 3],
}

/// Resolve and load the cape skeleton for a character body resref
/// (e.g. `"P_HHM_NK_Body01"`). Returns `None` if the prefix has no cape
/// skeleton (non-humanoid races) or the GR2 is missing.
pub fn load_cape_skeleton_for_body(
    rm: &ResourceManager,
    body_resref: &str,
) -> Option<(String, SkeletonData)> {
    let upper = body_resref.to_uppercase();
    if upper.len() < 6 {
        return None;
    }
    let cape_name = guess_cloak_skeleton(&upper[..6])?;
    let skel = load_skeleton(rm, cape_name)?;
    Some((cape_name.to_string(), skel))
}

pub fn head_has_fhair_meshes(resource_manager: &ResourceManager, head_resref: &str) -> bool {
    let Ok(mdb_data) = resource_manager.get_resource_bytes(head_resref, "mdb") else {
        return false;
    };
    let Ok(mdb) = MdbParser::parse(&mdb_data) else {
        return false;
    };
    mdb.skin_meshes
        .iter()
        .any(|m| m.name.to_lowercase().contains("_fhair"))
        || mdb
            .rigid_meshes
            .iter()
            .any(|m| m.name.to_lowercase().contains("_fhair"))
}

pub fn load_model(
    resource_manager: &ResourceManager,
    resref: &str,
    part: &str,
    tint_group: &str,
) -> Result<ModelData, String> {
    let (mut data, mdb) = parse_mdb(resource_manager, resref, part, tint_group)?;

    let skeleton_name = mdb
        .skin_meshes
        .first()
        .map(|sm| resolve_skeleton_name(&sm.name, &sm.skeleton_name));

    if let Some(ref skel_name) = skeleton_name {
        debug!("Resolving skeleton '{}' for model", skel_name);
        data.skeleton = load_skeleton(resource_manager, skel_name);
        if let Some(ref skel) = data.skeleton {
            let palettes = build_bone_palettes(skel);
            remap_mesh_bone_indices(&mut data.meshes, &palettes, skel);
            data.animations = load_idle_animations(resource_manager, skel_name);
        }
    }

    Ok(data)
}

pub fn load_model_with_skeleton(
    resource_manager: &ResourceManager,
    resref: &str,
    skeleton_resref: &str,
    part: &str,
    tint_group: &str,
) -> Result<ModelData, String> {
    let (mut data, mdb) = parse_mdb(resource_manager, resref, part, tint_group)?;

    if !mdb.skin_meshes.is_empty() {
        data.skeleton = load_skeleton(resource_manager, skeleton_resref);
        if let Some(ref skel) = data.skeleton {
            let palettes = build_bone_palettes(skel);
            remap_mesh_bone_indices(&mut data.meshes, &palettes, skel);
            data.animations = load_idle_animations(resource_manager, skeleton_resref);
        }
    }

    Ok(data)
}

pub fn load_meshes_with_existing_skeleton(
    resource_manager: &ResourceManager,
    resref: &str,
    part: &str,
    tint_group: &str,
    skeleton: &SkeletonData,
    palettes: &BonePalettes,
) -> Result<Vec<MeshData>, String> {
    let (mut data, _mdb) = parse_mdb(resource_manager, resref, part, tint_group)?;
    remap_mesh_bone_indices(&mut data.meshes, palettes, skeleton);
    Ok(data.meshes)
}

/// Load a cloak MDB, routing it to the cape skeleton when the MDB declares a
/// `*capewing_skel` AND a cape skeleton is provided. Otherwise the cloak is
/// rigged against the body skeleton (unique full-body cloaks).
///
/// Returns the meshes and, when the cape path was taken, the cape skeleton
/// (consumed from the input) wrapped as a `NamedSkeleton` for the caller to
/// register as a secondary skeleton. Parses the MDB exactly once.
pub fn load_cloak(
    rm: &ResourceManager,
    resref: &str,
    body_skeleton: &SkeletonData,
    body_palettes: &BonePalettes,
    cape: Option<(String, SkeletonData)>,
) -> Result<(Vec<MeshData>, Option<NamedSkeleton>), String> {
    let (mut data, mdb) = parse_mdb(rm, resref, "cloak", "cloak")?;

    let wants_cape = mdb
        .skin_meshes
        .first()
        .map(|sm| sm.skeleton_name.to_lowercase().contains("capewing"))
        .unwrap_or(false);

    if wants_cape && let Some((_cape_name, cape_skel)) = cape {
        let palette = build_cape_bone_palette(&cape_skel);
        remap_cloak_bone_indices(&mut data.meshes, &palette, cape_skel.bones.len());
        return Ok((
            data.meshes,
            Some(NamedSkeleton {
                name: "cape".to_string(),
                skeleton: cape_skel,
            }),
        ));
    }

    remap_mesh_bone_indices(&mut data.meshes, body_palettes, body_skeleton);
    Ok((data.meshes, None))
}

fn remap_secondary_bone_indices(
    meshes: &mut [MeshData],
    palette: &[usize],
    bone_count: usize,
    skeleton_ref: &str,
) {
    for mesh in meshes.iter_mut() {
        if let Some(ref mut indices) = mesh.bone_indices {
            for idx in indices.iter_mut() {
                let i = *idx as usize;
                let skel_idx = if i < palette.len() { palette[i] } else { i };
                *idx = skel_idx as u8;
            }
        }
        if mesh.mesh_type == "skin" {
            mesh.skeleton_ref = Some(skeleton_ref.to_string());
        }
    }

    for mesh in meshes.iter() {
        if let Some(ref indices) = mesh.bone_indices {
            let max = indices.iter().copied().max().unwrap_or(0) as usize;
            if max >= bone_count {
                warn!(
                    "Secondary mesh '{}' has bone index {} >= skeleton bone count {}",
                    mesh.name, max, bone_count
                );
            }
        }
    }
}

fn remap_cloak_bone_indices(meshes: &mut [MeshData], cape_palette: &[usize], bone_count: usize) {
    remap_secondary_bone_indices(meshes, cape_palette, bone_count, "cape");
}

/// Load a synced tail/wing MDB (base-anim set == body's) and deliver it as a
/// secondary skeleton, exactly like a cape. The part's own skeleton is loaded,
/// its skin meshes are remapped into that skeleton's bone space and tagged with
/// `skeleton_ref`/`tint_group` = `part_name` so the viewer binds them and the
/// shared body idle/fidget tracks animate them.
pub fn load_synced_part(
    rm: &ResourceManager,
    resref: &str,
    skeleton_resref: &str,
    part_name: &str,
) -> Result<(Vec<MeshData>, NamedSkeleton), String> {
    let (mut data, _mdb) = parse_mdb(rm, resref, part_name, part_name)?;

    let skeleton = load_skeleton(rm, skeleton_resref)
        .ok_or_else(|| format!("Synced part skeleton not found: {skeleton_resref}"))?;

    let palette = build_cape_bone_palette(&skeleton);
    remap_secondary_bone_indices(&mut data.meshes, &palette, skeleton.bones.len(), part_name);

    Ok((
        data.meshes,
        NamedSkeleton {
            name: part_name.to_string(),
            skeleton,
        },
    ))
}

fn parse_mdb(
    resource_manager: &ResourceManager,
    resref: &str,
    part: &str,
    tint_group: &str,
) -> Result<(ModelData, crate::parsers::mdb::types::MdbFile), String> {
    let mdb_data = resource_manager
        .get_resource_bytes(resref, "mdb")
        .map_err(|e| format!("Failed to load MDB {resref}: {e}"))?;

    let mdb =
        MdbParser::parse(&mdb_data).map_err(|e| format!("Failed to parse MDB {resref}: {e}"))?;

    let mut meshes = Vec::new();
    for rm in &mdb.rigid_meshes {
        meshes.push(flatten_rigid_mesh(rm, part, tint_group));
    }
    for sm in &mdb.skin_meshes {
        meshes.push(flatten_skin_mesh(sm, part, tint_group));
    }

    let hooks = mdb
        .hooks
        .iter()
        .map(|h| HookData {
            name: h.name.clone(),
            position: h.position,
            orientation: h.orientation,
        })
        .collect();

    let hair = mdb
        .hair
        .iter()
        .map(|h| HairData {
            name: h.name.clone(),
            shortening_behavior: h.shortening_behavior as u32,
            position: h.position,
            orientation: h.orientation,
        })
        .collect();

    let helm = mdb
        .helm
        .iter()
        .map(|h| HelmData {
            name: h.name.clone(),
            hiding_behavior: h.hiding_behavior as u32,
            position: h.position,
            orientation: h.orientation,
        })
        .collect();

    let data = ModelData {
        meshes,
        hooks,
        hair,
        helm,
        skeleton: None,
        animations: Vec::new(),
        attached_parts: Vec::new(),
        secondary_skeletons: Vec::new(),
    };

    Ok((data, mdb))
}

pub fn load_skeleton(
    resource_manager: &ResourceManager,
    skeleton_name: &str,
) -> Option<SkeletonData> {
    match resource_manager.get_resource_bytes(skeleton_name, "gr2") {
        Ok(gr2_data) => match Gr2Parser::parse(&gr2_data) {
            Ok(skel) => {
                debug!(
                    "Loaded skeleton '{}' with {} bones",
                    skel.name,
                    skel.bones.len()
                );
                Some(SkeletonData {
                    bones: skel
                        .bones
                        .iter()
                        .map(|b| {
                            let m = &b.inverse_world_4x4;
                            // GR2 stores inverse_world_4x4 in row-vector convention,
                            // row-major: translation in last row (m[12..14]), last column
                            // is [0,0,0,1]. Convert to Three.js column-major with
                            // coordinate swizzle (NWN2 Y-fwd/Z-up → Three.js Y-up/Z-back).
                            // Steps: transpose (row-vec → col-vec), then C·M·C⁻¹, then
                            // row-major → column-major. Positions already in cm (no ×100).
                            let iw = vec![
                                m[0], m[2], -m[1], m[3], m[8], m[10], -m[9], m[11], -m[4], -m[6],
                                m[5], -m[7], m[12], m[14], -m[13], m[15],
                            ];
                            BoneData {
                                name: b.name.clone(),
                                parent_index: b.parent_index,
                                position: b.transform.position,
                                rotation: b.transform.rotation,
                                scale: b.transform.scale,
                                inverse_world_4x4: iw,
                            }
                        })
                        .collect(),
                })
            }
            Err(e) => {
                warn!("Failed to parse GR2 {}: {}", skeleton_name, e);
                None
            }
        },
        Err(e) => {
            warn!("Skeleton not found {}: {}", skeleton_name, e);
            None
        }
    }
}

/// A tail/wing part is "synced" when its `NWN2_BaseAnims` set matches the body's
/// own animation set — the body idle clips already carry its bone tracks, so it
/// rides the body mixer like the cape. Otherwise it uses its own creature idle set.
pub fn is_synced_base_anims(body_skeleton_resref: &str, base_anims: Option<&str>) -> bool {
    let Some(base) = base_anims else {
        return false;
    };
    let body_prefix = body_skeleton_resref
        .strip_suffix("_skel")
        .or_else(|| body_skeleton_resref.strip_suffix("_Skel"))
        .unwrap_or(body_skeleton_resref);
    body_prefix.eq_ignore_ascii_case(base)
}

pub fn load_idle_animations(
    resource_manager: &ResourceManager,
    skeleton_name: &str,
) -> Vec<AnimationData> {
    let prefix = skeleton_name
        .strip_suffix("_skel")
        .or_else(|| skeleton_name.strip_suffix("_Skel"))
        .unwrap_or(skeleton_name);
    load_idle_animations_for_prefix(resource_manager, prefix)
}

pub fn load_idle_animations_for_prefix(
    resource_manager: &ResourceManager,
    anim_prefix: &str,
) -> Vec<AnimationData> {
    let idle_suffixes = ["_idle", "_idlefidgetnervous"];

    let mut animations = Vec::new();
    for suffix in &idle_suffixes {
        let resref = format!("{anim_prefix}{suffix}");
        let bytes = match resource_manager.get_resource_bytes(&resref, "gr2") {
            Ok(b) => {
                info!("Found {resref}.gr2 ({} bytes)", b.len());
                b
            }
            Err(e) => {
                info!("Idle animation not found: {resref}.gr2: {e}");
                continue;
            }
        };

        match Gr2Parser::parse_animations(&bytes) {
            Ok(gr2_anims) => {
                info!("Parsed {resref}: {} animations", gr2_anims.len());
                for anim in &gr2_anims {
                    let tag = format!("{resref} [{}]", anim.name);
                    let mut converted = convert_animation(anim);
                    converted.name = tag;
                    animations.push(converted);
                }
            }
            Err(e) => {
                info!("Failed to parse {resref}: {e}");
            }
        }
    }

    animations
}

fn convert_animation(anim: &crate::parsers::gr2::Gr2Animation) -> AnimationData {
    let tracks = anim
        .tracks
        .iter()
        .filter(|t| {
            !t.position_keys.is_empty() || !t.rotation_keys.is_empty() || !t.scale_keys.is_empty()
        })
        .map(|track| {
            let primary_times: Vec<f32> = if !track.rotation_keys.is_empty() {
                track.rotation_keys.iter().map(|(t, _)| *t).collect()
            } else if !track.position_keys.is_empty() {
                track.position_keys.iter().map(|(t, _)| *t).collect()
            } else {
                track.scale_keys.iter().map(|(t, _)| *t).collect()
            };

            let positions = if track.position_keys.is_empty() {
                None
            } else {
                Some(
                    track
                        .position_keys
                        .iter()
                        .flat_map(|(_, v)| v.iter().copied())
                        .collect(),
                )
            };

            let rotations = if track.rotation_keys.is_empty() {
                None
            } else {
                Some(
                    track
                        .rotation_keys
                        .iter()
                        .flat_map(|(_, v)| v.iter().copied())
                        .collect(),
                )
            };

            let scales = if track.scale_keys.is_empty() {
                None
            } else {
                Some(
                    track
                        .scale_keys
                        .iter()
                        .flat_map(|(_, v)| v.iter().copied())
                        .collect(),
                )
            };

            TrackData {
                bone_name: track.bone_name.clone(),
                times: primary_times,
                positions,
                rotations,
                scales,
            }
        })
        .collect();

    AnimationData {
        name: anim.name.clone(),
        duration: anim.duration,
        tracks,
    }
}

fn convert_material(m: &Material) -> MaterialData {
    MaterialData {
        diffuse_map: m.diffuse_map_name.clone(),
        normal_map: m.normal_map_name.clone(),
        tint_map: m.tint_map_name.clone(),
        glow_map: m.glow_map_name.clone(),
        flags: m.flags,
    }
}

fn flatten_rigid_mesh(rm: &RigidMeshPacket, part: &str, tint_group: &str) -> MeshData {
    let vc = rm.vertices.len();
    let mut positions = Vec::with_capacity(vc * 3);
    let mut normals = Vec::with_capacity(vc * 3);
    let mut uvs = Vec::with_capacity(vc * 2);
    let mut tangents = Vec::with_capacity(vc * 3);

    for v in &rm.vertices {
        positions.extend_from_slice(&v.position);
        normals.extend_from_slice(&v.normal);
        tangents.extend_from_slice(&v.tangent);
        uvs.push(v.uvw[0]);
        uvs.push(v.uvw[1]);
    }

    MeshData {
        name: rm.name.clone(),
        mesh_type: "rigid".to_string(),
        part: part.to_string(),
        tint_group: tint_group.to_string(),
        positions,
        normals,
        uvs,
        tangents,
        indices: rm.faces.iter().flat_map(|f| f.indices).collect(),
        bone_weights: None,
        bone_indices: None,
        material: convert_material(&rm.material),
        skeleton_ref: None,
        attach_bone: None,
        override_tints: None,
    }
}

/// Bone palettes for MDB skinning index remapping.
/// Matches nwn2mdk's process_fbx_bones (export_info.cpp):
/// iterate skeleton array order, skip ap_*, separate f_*, Ribcage last.
pub struct BonePalettes {
    body: Vec<usize>,
    face: Vec<usize>,
}

pub fn build_bone_palettes(skeleton: &SkeletonData) -> BonePalettes {
    let mut body = Vec::new();
    let mut face = Vec::new();
    let mut ribcage: Option<usize> = None;

    for (skel_idx, bone) in skeleton.bones.iter().enumerate() {
        if bone.name.starts_with("ap_") {
            // attachment points — not used for skinning
        } else if bone.name.starts_with("f_") {
            face.push(skel_idx);
        } else if bone.name == "Ribcage" {
            ribcage = Some(skel_idx);
        } else {
            body.push(skel_idx);
        }
    }

    if let Some(rc) = ribcage {
        body.push(rc);
    }

    BonePalettes { body, face }
}

/// Palette for secondary skeletons (cape, tail). Unlike the body palette,
/// there is no face/Ribcage special casing — only `ap_*` attachment points
/// are skipped. Order matches skeleton bone order.
pub fn build_cape_bone_palette(skeleton: &SkeletonData) -> Vec<usize> {
    skeleton
        .bones
        .iter()
        .enumerate()
        .filter(|(_, b)| !b.name.starts_with("ap_"))
        .map(|(i, _)| i)
        .collect()
}

/// Remap MDB bone_indices from palette space to full-skeleton space.
/// Cutscene meshes (heads) use face palette for low indices, body palette for the rest.
fn remap_mesh_bone_indices(
    meshes: &mut [MeshData],
    palettes: &BonePalettes,
    skeleton: &SkeletonData,
) {
    use crate::parsers::mdb::types::material_flags::CUTSCENE_MESH;

    // Phase 1: standard palette remap
    for mesh in meshes.iter_mut() {
        let Some(ref mut indices) = mesh.bone_indices else {
            continue;
        };
        let is_cutscene = mesh.material.flags & CUTSCENE_MESH != 0;
        for idx in indices.iter_mut() {
            let i = *idx as usize;
            let skel_idx = if is_cutscene && i < palettes.face.len() {
                palettes.face[i]
            } else if i < palettes.body.len() {
                palettes.body[i]
            } else {
                i // fallback: pass through
            };
            *idx = skel_idx as u8;
        }
    }

    // Phase 2: detect and fix eye↔lid palette mismatch.
    // Some race MDBs were authored with a skeleton version where
    // eye rotation bones had different palette positions than the
    // current GR2 skeleton. Detect by checking if an "Eye" mesh
    // ended up bound to eyelid bones (should be eyeL/eyeR).
    fix_eye_lid_mismatch(meshes, skeleton);
}

fn fix_eye_lid_mismatch(meshes: &mut [MeshData], skeleton: &SkeletonData) {
    let find = |name: &str| -> Option<usize> { skeleton.bones.iter().position(|b| b.name == name) };

    let (Some(eye_l), Some(eye_r), Some(lid_l), Some(lid_r)) =
        (find("eyeL"), find("eyeR"), find("eyeLlid"), find("eyeRlid"))
    else {
        return;
    };

    let needs_swap = meshes.iter().any(|mesh| {
        let lower = mesh.name.to_ascii_lowercase();
        if !lower.contains("eye") || lower.contains("lid") {
            return false;
        }
        let Some(ref indices) = mesh.bone_indices else {
            return false;
        };
        indices
            .iter()
            .any(|&idx| idx as usize == lid_l || idx as usize == lid_r)
    });

    if !needs_swap {
        return;
    }

    debug!(
        "Eye/lid mismatch detected: swapping eyeL(skel[{}])↔eyeLlid(skel[{}]), eyeR(skel[{}])↔eyeRlid(skel[{}])",
        eye_l, lid_l, eye_r, lid_r
    );

    // Apply symmetric swap to ALL meshes so both eye and head meshes are fixed
    for mesh in meshes.iter_mut() {
        let Some(ref mut indices) = mesh.bone_indices else {
            continue;
        };
        for idx in indices.iter_mut() {
            let i = *idx as usize;
            if i == eye_l {
                *idx = lid_l as u8;
            } else if i == lid_l {
                *idx = eye_l as u8;
            } else if i == eye_r {
                *idx = lid_r as u8;
            } else if i == lid_r {
                *idx = eye_r as u8;
            }
        }
    }
}

fn flatten_skin_mesh(sm: &SkinMeshPacket, part: &str, tint_group: &str) -> MeshData {
    let vc = sm.vertices.len();
    let mut positions = Vec::with_capacity(vc * 3);
    let mut normals = Vec::with_capacity(vc * 3);
    let mut uvs = Vec::with_capacity(vc * 2);
    let mut tangents = Vec::with_capacity(vc * 3);
    let mut bone_weights = Vec::with_capacity(vc * 4);
    let mut bone_indices = Vec::with_capacity(vc * 4);

    for v in &sm.vertices {
        positions.extend_from_slice(&v.position);
        normals.extend_from_slice(&v.normal);
        tangents.extend_from_slice(&v.tangent);
        uvs.push(v.uvw[0]);
        uvs.push(v.uvw[1]);
        bone_weights.extend_from_slice(&v.bone_weights);
        bone_indices.extend_from_slice(&v.bone_indices);
    }

    MeshData {
        name: sm.name.clone(),
        mesh_type: "skin".to_string(),
        part: part.to_string(),
        tint_group: tint_group.to_string(),
        positions,
        normals,
        uvs,
        tangents,
        indices: sm.faces.iter().flat_map(|f| f.indices).collect(),
        bone_weights: Some(bone_weights),
        bone_indices: Some(bone_indices),
        material: convert_material(&sm.material),
        skeleton_ref: None,
        attach_bone: None,
        override_tints: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(name: &str, parent: i32) -> BoneData {
        BoneData {
            name: name.to_string(),
            parent_index: parent,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            inverse_world_4x4: vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    #[test]
    fn test_build_cape_bone_palette_skips_ap_prefix() {
        let skel = SkeletonData {
            bones: vec![
                bone("P_HHMcapewing_skel", -1),
                bone("ap_tail", 0),
                bone("MCape1", 0),
                bone("MCape2", 2),
                bone("ap_wings", 0),
                bone("LCape1", 0),
            ],
        };

        let palette = build_cape_bone_palette(&skel);
        assert_eq!(palette, vec![0, 2, 3, 5]);
    }

    #[test]
    fn test_build_cape_bone_palette_empty_skeleton() {
        let skel = SkeletonData { bones: Vec::new() };
        assert!(build_cape_bone_palette(&skel).is_empty());
    }

    #[test]
    fn test_build_cape_bone_palette_only_ap_bones() {
        let skel = SkeletonData {
            bones: vec![bone("ap_wings", -1), bone("ap_tail", -1)],
        };
        assert!(build_cape_bone_palette(&skel).is_empty());
    }
}
