export interface MaterialData {
  diffuse_map: string;
  normal_map: string;
  tint_map: string;
  glow_map: string;
  flags: number;
  diffuse_color: [number, number, number];
  specular_color: [number, number, number];
  specular_level: number;
  specular_power: number;
}

export interface MeshData {
  name: string;
  mesh_type: string;
  part: string;
  tint_group: string;
  positions: number[];
  normals: number[];
  uvs: number[];
  tangents: number[];
  indices: number[];
  bone_weights: number[] | null;
  bone_indices: number[] | null;
  material: MaterialData;
  skeleton_ref?: string | null;
  /// Skeleton bone to parent this mesh under (rigid armor accessories).
  attach_bone?: string | null;
  /// Per-mesh tint channels overriding the item's top-level tint. Set for
  /// armor accessories so each slot can carry its own colour.
  override_tints?: {
    channel1: { r: number; g: number; b: number; a: number };
    channel2: { r: number; g: number; b: number; a: number };
    channel3: { r: number; g: number; b: number; a: number };
  } | null;
}

export interface BoneData {
  name: string;
  parent_index: number;
  position: [number, number, number];
  rotation: [number, number, number, number];
  scale: [number, number, number];
  inverse_world_4x4: number[];
}

export interface HookData {
  name: string;
  position: [number, number, number];
  orientation: [[number, number, number], [number, number, number], [number, number, number]];
}

export interface HairData {
  name: string;
  shortening_behavior: number;
  position: [number, number, number];
  orientation: [[number, number, number], [number, number, number], [number, number, number]];
}

export interface HelmData {
  name: string;
  hiding_behavior: number;
  position: [number, number, number];
  orientation: [[number, number, number], [number, number, number], [number, number, number]];
}

export interface TrackData {
  bone_name: string;
  times: number[];
  positions: number[] | null;
  rotations: number[] | null;
  scales: number[] | null;
}

export interface AnimationData {
  name: string;
  duration: number;
  tracks: TrackData[];
}

export interface AttachedPart {
  name: string;
  meshes: MeshData[];
  skeleton: { bones: BoneData[] } | null;
  animations: AnimationData[];
  attach_bone: string | null;
}

export interface NamedSkeleton {
  name: string;
  skeleton: { bones: BoneData[] };
}

export interface ModelData {
  meshes: MeshData[];
  hooks: HookData[];
  hair: HairData[];
  helm: HelmData[];
  skeleton: { bones: BoneData[] } | null;
  animations: AnimationData[];
  attached_parts?: AttachedPart[];
  secondary_skeletons?: NamedSkeleton[];
}
