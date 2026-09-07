import * as THREE from 'three';
import { DDSLoader } from 'three/examples/jsm/loaders/DDSLoader.js';
import { invoke } from '@tauri-apps/api/core';

const ddsLoader = new DDSLoader();

export interface TintColors {
  channel1: [number, number, number];
  channel2: [number, number, number];
  channel3: [number, number, number];
}

export interface TintChannelsRaw {
  channel1: { r: number; g: number; b: number; a: number };
  channel2: { r: number; g: number; b: number; a: number };
  channel3: { r: number; g: number; b: number; a: number };
}

export function tintChannelsToColors(tc: TintChannelsRaw): TintColors {
  return {
    channel1: [tc.channel1.r / 255, tc.channel1.g / 255, tc.channel1.b / 255],
    channel2: [tc.channel2.r / 255, tc.channel2.g / 255, tc.channel2.b / 255],
    channel3: [tc.channel3.r / 255, tc.channel3.g / 255, tc.channel3.b / 255],
  };
}

// Parsed-DDS cache: fetching + parsing is the expensive part and variants of
// one item mostly share textures, so cache the CPU-side parse result (null =
// known failure) and build a fresh, independently-disposable CompressedTexture
// per consumer. Never cache live GPU textures here - consumers dispose them.
type ParsedDds = ReturnType<DDSLoader['parse']>;

const DDS_CACHE_LIMIT = 300;
const DDS_CACHE_BYTES_LIMIT = 128 * 1024 * 1024;
const ddsCache = new Map<string, ParsedDds | null>();
const pendingDds = new Map<string, Promise<ParsedDds | null>>();
let ddsCacheBytes = 0;

function ddsBytes(parsed: ParsedDds | null | undefined): number {
  return parsed?.mipmaps.reduce((total, mip) => total + mip.data.byteLength, 0) ?? 0;
}

function ddsCachePut(name: string, parsed: ParsedDds | null): void {
  if (ddsCache.has(name)) {
    ddsCacheBytes -= ddsBytes(ddsCache.get(name));
    ddsCache.delete(name);
  }
  const bytes = ddsBytes(parsed);
  if (bytes > DDS_CACHE_BYTES_LIMIT) return;
  while (ddsCache.size >= DDS_CACHE_LIMIT || ddsCacheBytes + bytes > DDS_CACHE_BYTES_LIMIT) {
    const oldest = ddsCache.keys().next().value;
    if (oldest === undefined) break;
    ddsCacheBytes -= ddsBytes(ddsCache.get(oldest));
    ddsCache.delete(oldest);
  }
  ddsCache.set(name, parsed);
  ddsCacheBytes += bytes;
}

function ddsCacheGet(name: string): ParsedDds | null | undefined {
  const value = ddsCache.get(name);
  if (value !== undefined) {
    ddsCache.delete(name);
    ddsCache.set(name, value);
  }
  return value;
}

function buildTexture(dds: ParsedDds): THREE.CompressedTexture {
  const texture = new THREE.CompressedTexture(
    dds.mipmaps,
    dds.width,
    dds.height,
    dds.format as THREE.CompressedPixelFormat,
  );
  texture.needsUpdate = true;
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  return texture;
}

/// maxSize asks the backend to drop mip levels above that dimension (EE
/// upscales are 2048px; thumbnails need far less), cached separately from
/// the full-size texture.
export async function loadDDSTexture(
  textureName: string,
  maxSize?: number,
): Promise<THREE.CompressedTexture | null> {
  if (!textureName) return null;
  const parsed = await fetchParsedDds(textureName, maxSize);
  return parsed ? buildTexture(parsed) : null;
}

/// Fetches and parses a DDS into the cache without building a texture, so a
/// later loadDDSTexture for the same name and size is a cache hit.
export async function warmDDSCache(textureName: string, maxSize?: number): Promise<void> {
  if (!textureName) return;
  await fetchParsedDds(textureName, maxSize);
}

async function fetchParsedDds(textureName: string, maxSize?: number): Promise<ParsedDds | null> {
  const key = maxSize ? `${textureName}@${maxSize}` : textureName;
  const cached = ddsCacheGet(key);
  if (cached !== undefined) return cached;

  const pending = pendingDds.get(key);
  if (pending) return pending;

  const request = (async () => {
    try {
      const buffer: ArrayBuffer = await invoke('get_texture_bytes', { name: textureName, maxSize });
      const parsed = ddsLoader.parse(buffer, true);
      if (!parsed.width || !parsed.height || !parsed.format || parsed.mipmaps.length === 0) {
        throw new Error(`Unsupported or invalid DDS: ${textureName}`);
      }
      ddsCachePut(key, parsed);
      return parsed;
    } catch (err) {
      console.warn(`[texture] Failed to load '${textureName}':`, err);
      ddsCachePut(key, null);
      return null;
    } finally {
      pendingDds.delete(key);
    }
  })();

  pendingDds.set(key, request);
  return request;
}

function injectTintShader(
  mat: THREE.MeshStandardMaterial,
  tintMapTexture: THREE.CompressedTexture,
  tintColors: TintColors,
  swapGB: boolean,
): void {
  const uniforms = {
    tintMap: { value: tintMapTexture },
    tintColor1: { value: new THREE.Vector3(...tintColors.channel1) },
    tintColor2: { value: new THREE.Vector3(...tintColors.channel2) },
    tintColor3: { value: new THREE.Vector3(...tintColors.channel3) },
  };

  (mat.userData as Record<string, unknown>).tintUniforms = uniforms;

  // swapGB converts viewer palettes back to raw shader RGB order.
  const ch2 = swapGB ? 'b' : 'g';
  const ch3 = swapGB ? 'g' : 'b';

  // three.js does NOT include onBeforeCompile changes in its shader-program
  // cache key. Without this, two tint materials that differ ONLY in swapGB
  // (e.g. the swap=false hair mesh and the swap=true head/face mesh, both with
  // DXT5 masks) collide on the same key and the second silently reuses the
  // first's compiled program — so the face renders with the hair's swap and
  // the eyebrows get driven by the eye channel. Keying on swapGB keeps the two
  // channel orderings in separate programs.
  mat.customProgramCacheKey = () => `nwn2tint_${swapGB ? 'gb' : 'rgb'}`;

  mat.onBeforeCompile = (shader) => {
    shader.uniforms.tintMap = uniforms.tintMap;
    shader.uniforms.tintColor1 = uniforms.tintColor1;
    shader.uniforms.tintColor2 = uniforms.tintColor2;
    shader.uniforms.tintColor3 = uniforms.tintColor3;

    shader.fragmentShader = shader.fragmentShader.replace(
      '#include <common>',
      `#include <common>
uniform sampler2D tintMap;
uniform vec3 tintColor1;
uniform vec3 tintColor2;
uniform vec3 tintColor3;
`,
    );

    shader.fragmentShader = shader.fragmentShader.replace(
      '#include <color_fragment>',
      `#include <color_fragment>
{
  #if defined(USE_MAP)
    vec2 tintUv = vMapUv;
  #elif defined(USE_NORMALMAP)
    vec2 tintUv = vNormalMapUv;
  #else
    vec2 tintUv = vec2(0.0);
  #endif
  vec4 tintMask = texture2D(tintMap, tintUv);
  // EE LightingPS: weighted sum, with alpha blending against untinted albedo.
  vec3 tint = tintColor1 * tintMask.r
            + tintColor2 * tintMask.${ch2}
            + tintColor3 * tintMask.${ch3};
  diffuseColor.rgb = mix(diffuseColor.rgb, diffuseColor.rgb * tint, tintMask.a);
}
`,
    );
  };
}

export function updateTintUniforms(
  scene: THREE.Scene,
  tintGroup: string,
  tintColors: TintColors,
): void {
  scene.traverse((obj) => {
    if (!(obj instanceof THREE.Mesh || obj instanceof THREE.SkinnedMesh)) return;
    if (obj.userData.tintGroup !== tintGroup) return;
    if (obj.userData.hasOverrideTints) return;

    const mat = obj.material as THREE.MeshStandardMaterial;
    const uniforms = (mat.userData as Record<string, unknown>).tintUniforms as
      | Record<string, { value: THREE.Vector3 }>
      | undefined;
    if (!uniforms) return;

    uniforms.tintColor1.value.set(...tintColors.channel1);
    uniforms.tintColor2.value.set(...tintColors.channel2);
    uniforms.tintColor3.value.set(...tintColors.channel3);
  });
}

// Calibration constants for MDB specular mapping. Values are tuned against
// in-game reference screenshots — edit ONLY this block during calibration.
export const SPECULAR = {
  INTENSITY_SCALE: 18.0,
  // three.js dielectric F0 is 0.04 * specularIntensity; NWN2's phong steel
  // needs far more than the physical intensity=1 ceiling.
  INTENSITY_MAX: 24.0,
  ROUGHNESS_MIN: 0.05,
  ROUGHNESS_MAX: 1.0,
  ENV_MAP_INTENSITY: 6.0,
  // 0x08 "mirroring" renders as an albedo-tinted metal reflection (gold trim
  // reflects gold, steel reflects silver).
  ENV_MAP_METALNESS: 0.85,
  // The engine multiplies the diffuse map by the MDB diffuse_color (plate
  // uses values down to 0.14). A/B toggle for calibration.
  DIFFUSE_COLOR_ENABLED: true,
};

export type ShadingMode = 'physical' | 'game';
export const DEFAULT_SHADING_MODE: ShadingMode = 'game';

function injectGameLighting(mat: THREE.MeshPhysicalMaterial, specular: THREE.Color, power: number): void {
  const uniforms = {
    nwn2GameStyle: { value: 0 },
    nwn2Specular: { value: specular },
    nwn2SpecularPower: { value: power },
  };
  mat.userData.lightingUniforms = uniforms;
  mat.onBeforeRender = (_renderer, scene) => {
    uniforms.nwn2GameStyle.value = scene.userData.shadingMode === 'game' ? 1 : 0;
  };
  const tintCompile = mat.onBeforeCompile;
  const tintKey = mat.customProgramCacheKey();
  mat.customProgramCacheKey = () => `${tintKey}_nwn2lighting_v2`;
  mat.onBeforeCompile = (shader, renderer) => {
    tintCompile.call(mat, shader, renderer);
    Object.assign(shader.uniforms, uniforms);
    shader.fragmentShader = shader.fragmentShader.replace(
      '#include <normalmap_pars_fragment>',
      `#include <normalmap_pars_fragment>
uniform float nwn2GameStyle;
uniform vec3 nwn2Specular;
uniform float nwn2SpecularPower;

void RE_Direct_NWN2(const in IncidentLight light, const in vec3 position,
    const in vec3 normal, const in vec3 viewDir, const in vec3 clearcoatNormal,
    const in PhysicalMaterial material, inout ReflectedLight reflectedLight) {
  if (nwn2GameStyle < 0.5) {
    RE_Direct_Physical(light, position, normal, viewDir, clearcoatNormal, material, reflectedLight);
    return;
  }
  vec3 halfVector = light.direction + viewDir;
  halfVector *= inversesqrt(max(dot(halfVector, halfVector), 1e-8));
  float highlight = pow(saturate(dot(normal, halfVector)), nwn2SpecularPower);
  float gloss = 1.0;
  #ifdef USE_NORMALMAP
    gloss = texture2D(normalMap, vNormalMapUv).a;
  #endif
  // Convert the physical preview rig's irradiance to unnormalized game light units.
  vec3 lightColor = light.color * RECIPROCAL_PI;
  reflectedLight.directDiffuse += lightColor * saturate(dot(normal, light.direction)) * material.diffuseColor;
  reflectedLight.directSpecular += lightColor * nwn2Specular * highlight * gloss;
}

void RE_IndirectDiffuse_NWN2(const in vec3 irradiance, const in vec3 position,
    const in vec3 normal, const in vec3 viewDir, const in vec3 clearcoatNormal,
    const in PhysicalMaterial material, inout ReflectedLight reflectedLight) {
  if (nwn2GameStyle < 0.5) {
    RE_IndirectDiffuse_Physical(irradiance, position, normal, viewDir, clearcoatNormal, material, reflectedLight);
    return;
  }
  reflectedLight.indirectDiffuse += (irradiance * RECIPROCAL_PI) * material.diffuseColor;
}

void RE_IndirectSpecular_NWN2(const in vec3 radiance, const in vec3 irradiance,
    const in vec3 clearcoatRadiance, const in vec3 position, const in vec3 normal,
    const in vec3 viewDir, const in vec3 clearcoatNormal,
    const in PhysicalMaterial material, inout ReflectedLight reflectedLight) {
  if (nwn2GameStyle < 0.5) {
    RE_IndirectSpecular_Physical(radiance, irradiance, clearcoatRadiance, position, normal, viewDir, clearcoatNormal, material, reflectedLight);
    return;
  }
  // Keep the existing 0x08 reflection approximation separate from diffuse.
  #ifdef USE_ENVMAP
    ReflectedLight environment = ReflectedLight(vec3(0.0), vec3(0.0), vec3(0.0), vec3(0.0));
    RE_IndirectSpecular_Physical(radiance, irradiance, clearcoatRadiance, position, normal, viewDir, clearcoatNormal, material, environment);
    reflectedLight.indirectSpecular += environment.indirectSpecular;
  #endif
}

#undef RE_Direct
#undef RE_IndirectDiffuse
#undef RE_IndirectSpecular
#define RE_Direct RE_Direct_NWN2
#define RE_IndirectDiffuse RE_IndirectDiffuse_NWN2
#define RE_IndirectSpecular RE_IndirectSpecular_NWN2
`,
    );
  };
}

// Blinn-Phong exponent → GGX roughness. Non-finite/absent power falls back
// to 1, which maps to ~0.82 — close to the viewer's previous flat default.
function powerToRoughness(power: number | undefined): number {
  const p = power !== undefined && Number.isFinite(power) && power > 0 ? power : 1;
  return THREE.MathUtils.clamp(
    Math.sqrt(2 / (p + 2)),
    SPECULAR.ROUGHNESS_MIN,
    SPECULAR.ROUGHNESS_MAX,
  );
}

export async function createMaterial(
  materialData: {
    diffuse_map: string;
    normal_map: string;
    tint_map?: string;
    glow_map: string;
    flags: number;
    diffuse_color?: [number, number, number];
    specular_color?: [number, number, number];
    specular_level?: number;
    specular_power?: number;
  },
  tintColors?: TintColors,
  swapGB: boolean = false,
  envMap: THREE.Texture | null = null,
  textureMaxSize?: number,
): Promise<THREE.MeshPhysicalMaterial> {
  const rawLevel = materialData.specular_level;
  const specLevel = rawLevel !== undefined && Number.isFinite(rawLevel) ? Math.max(rawLevel, 0) : 0;
  const mat = new THREE.MeshPhysicalMaterial({
    side: THREE.DoubleSide,
    roughness: powerToRoughness(materialData.specular_power),
    metalness: 0.0,
    specularColor: new THREE.Color(...(materialData.specular_color ?? [1, 1, 1])),
    specularIntensity: THREE.MathUtils.clamp(
      specLevel * SPECULAR.INTENSITY_SCALE,
      0,
      SPECULAR.INTENSITY_MAX,
    ),
  });
  if (SPECULAR.DIFFUSE_COLOR_ENABLED && materialData.diffuse_color) {
    mat.color = new THREE.Color(...materialData.diffuse_color);
  }

  const needGlow = !!(materialData.flags & 0x20);

  const [diffuse, tintTex, normal, glow] = await Promise.all([
    loadDDSTexture(materialData.diffuse_map, textureMaxSize),
    materialData.tint_map
      ? loadDDSTexture(materialData.tint_map, textureMaxSize)
      : Promise.resolve(null),
    loadDDSTexture(materialData.normal_map, textureMaxSize),
    needGlow ? loadDDSTexture(materialData.glow_map, textureMaxSize) : Promise.resolve(null),
  ]);

  if (diffuse) {
    mat.map = diffuse;
  }

  if (tintTex && tintColors) {
    if (!diffuse) {
      const white = new THREE.DataTexture(new Uint8Array([255, 255, 255, 255]), 1, 1);
      white.needsUpdate = true;
      mat.map = white;
    }
    injectTintShader(mat, tintTex, tintColors, swapGB);
  } else if (!diffuse && tintTex) {
    mat.map = tintTex;
  }

  if (normal) {
    mat.normalMap = normal;
    // NWN2 stores the gloss mask in the normal map's alpha channel (verified
    // against P_HHF_PF_Body01_n: alpha is the worn-steel pattern). three.js
    // multiplies this texture's alpha against specularIntensity.
    mat.specularIntensityMap = normal;
  }

  if (materialData.flags & 0x01) {
    mat.alphaTest = 0.5;
  }

  if (materialData.flags & 0x06) {
    mat.transparent = true;
    mat.depthWrite = false;
    if (materialData.flags & 0x04) mat.blending = THREE.AdditiveBlending;
  }

  if (glow) {
    mat.emissiveMap = glow;
    mat.emissive = new THREE.Color(1, 1, 1);
  }

  if (envMap && materialData.flags & 0x08) {
    mat.envMap = envMap;
    mat.envMapIntensity = SPECULAR.ENV_MAP_INTENSITY;
    mat.metalness = SPECULAR.ENV_MAP_METALNESS;
  }

  // CPU-side MDB-to-light preprocessing is unresolved; use authored values
  // for the comparison, without the physical path's plate calibration.
  const power = materialData.specular_power;
  injectGameLighting(
    mat,
    new THREE.Color(...(materialData.specular_color ?? [1, 1, 1])).multiplyScalar(specLevel),
    power !== undefined && Number.isFinite(power) && power > 0 ? power : 1,
  );

  return mat;
}
