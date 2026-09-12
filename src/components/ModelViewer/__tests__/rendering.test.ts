import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import { buildMesh } from '../meshBuilder';
import { SPECULAR, createMaterial, ddsMeanAlpha, updateTintUniforms } from '../materials';
import type { MeshData } from '../types';

// One 4x4 block: DXT1 (8 data bytes) or DXT5 with a flat alpha (16 data bytes,
// a0 = a1 = alpha, all indices 0).
function makeDds(fourCC: 'DXT1' | 'DXT5', alpha = 255): ArrayBuffer {
  const blockBytes = fourCC === 'DXT1' ? 8 : 16;
  const buffer = new ArrayBuffer(128 + blockBytes);
  const header = new Int32Array(buffer, 0, 31);
  header[0] = 0x20534444;
  header[1] = 124;
  header[3] = 4;
  header[4] = 4;
  header[20] = 4;
  header[21] = fourCC === 'DXT1' ? 0x31545844 : 0x35545844;
  if (fourCC === 'DXT5') {
    const block = new Uint8Array(buffer, 128, 16);
    block[0] = alpha;
    block[1] = alpha;
  }
  return buffer;
}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (_cmd: string, args: { name: string }) => {
    if (args.name === 'matte_normal') return makeDds('DXT5', 0);
    if (args.name === 'glossy_normal') return makeDds('DXT5', 255);
    return makeDds('DXT1');
  }),
}));

const materialData = {
  diffuse_map: '', normal_map: '', tint_map: '', glow_map: '', flags: 0,
  diffuse_color: [1, 1, 1] as [number, number, number],
  specular_color: [1, 1, 1] as [number, number, number],
  specular_level: 0, specular_power: 1,
};

const meshData: MeshData = {
  name: 'accessory', mesh_type: 'rigid', part: 'item', tint_group: 'item',
  positions: [1, 2, 3], normals: [0, 0, 1], uvs: [0, 0],
  tangents: [0, 1, 0, -1], indices: [], bone_weights: null, bone_indices: null,
  material: materialData,
};

describe('MDB rendering', () => {
  it.each([false, true])('uses the EE weighted-sum tint equation (swapped=%s)', async (swapGB) => {
    const colors = { channel1: [0.2, 0.3, 0.4], channel2: [0.5, 0.6, 0.7], channel3: [0.8, 0.9, 1] } as const;
    const material = await createMaterial(
      { ...materialData, diffuse_map: 'test_diffuse', tint_map: 'test_mask' },
      { channel1: [...colors.channel1], channel2: [...colors.channel2], channel3: [...colors.channel3] },
      swapGB,
    );
    const shader = {
      uniforms: {},
      vertexShader: THREE.ShaderLib.physical.vertexShader,
      fragmentShader: THREE.ShaderLib.physical.fragmentShader,
    } as Parameters<typeof material.onBeforeCompile>[0];
    material.onBeforeCompile(shader, {} as THREE.WebGLRenderer);
    const compactShader = shader.fragmentShader.replace(/\s+/g, ' ');
    expect(compactShader).toContain(`vec3 tint = tintColor1 * tintMask.r + tintColor2 * tintMask.${swapGB ? 'b' : 'g'} + tintColor3 * tintMask.${swapGB ? 'g' : 'b'};`);
    expect(compactShader).toContain('vec3 factor = mix(vec3(1.0), tint, tintMask.a);');
    expect(compactShader).toContain('diffuseColor.rgb *= factorLinear;');
    expect(shader.uniforms.tintColor2.value.toArray()).toEqual(colors.channel2);
    expect(shader.uniforms.nwn2GameStyle).toBe(material.userData.lightingUniforms.nwn2GameStyle);
    expect(shader.fragmentShader).toContain('#define RE_Direct RE_Direct_NWN2');
    expect(material.customProgramCacheKey()).toBe(`nwn2tint_srgb_${swapGB ? 'gb' : 'rgb'}_nwn2lighting_v2`);
    material.map?.dispose();
    shader.uniforms.tintMap.value.dispose();
    material.dispose();
  });

  it('rotates authored tangents with the mesh and retains mirrored handedness', () => {
    const material = new THREE.MeshPhysicalMaterial();
    const mesh = buildMesh(meshData, material) as THREE.Mesh;
    expect(Array.from(mesh.geometry.getAttribute('tangent').array)).toEqual([0, 0, -1, -1]);
    expect(Array.from(mesh.geometry.getAttribute('normal').array)).toEqual([0, 1, -0]);
    mesh.geometry.dispose();
    material.dispose();
  });

  it('switches material response without changing the calibrated physical values', async () => {
    const material = await createMaterial({
      ...materialData, normal_map: 'hair_normal', flags: 1,
      specular_color: [0.9, 0.9, 0.9], specular_level: 0.45, specular_power: 10,
    });
    const scene = new THREE.Scene();
    const mesh = new THREE.Mesh(new THREE.PlaneGeometry(), material);
    const render = () => material.onBeforeRender(
      {} as THREE.WebGLRenderer, scene, new THREE.PerspectiveCamera(), mesh.geometry, mesh,
      {} as THREE.Group,
    );
    const uniforms = material.userData.lightingUniforms;
    expect(material.specularIntensity).toBeCloseTo(8.1);
    expect(material.specularIntensityMap).toBe(material.normalMap);
    expect(uniforms.nwn2Specular.value.r).toBeCloseTo(0.405 * SPECULAR.GAME_SPECULAR_SCALE);
    expect(uniforms.nwn2SpecularPower.value).toBe(10);
    render();
    expect(uniforms.nwn2GameStyle.value).toBe(0);
    const version = material.version;
    scene.userData.shadingMode = 'game';
    render();
    expect(uniforms.nwn2GameStyle.value).toBe(1);
    expect(material.version).toBe(version);
    expect(material.specularIntensity).toBeCloseTo(8.1);
    expect(material.envMap).toBeNull();
    expect(material.transparent).toBe(false);
    scene.userData.shadingMode = 'physical';
    render();
    expect(uniforms.nwn2GameStyle.value).toBe(0);
    material.normalMap?.dispose();
    mesh.geometry.dispose();
    material.dispose();
  });

  it('keeps accessory colors while the primary item changes color', () => {
    const scene = new THREE.Scene();
    const palette = { channel1: { r: 20, g: 40, b: 60, a: 255 }, channel2: { r: 70, g: 80, b: 90, a: 255 }, channel3: { r: 100, g: 110, b: 120, a: 255 } };
    for (const override_tints of [null, palette]) {
      const material = new THREE.MeshPhysicalMaterial();
      material.userData.tintUniforms = {
        tintColor1: { value: new THREE.Vector3(0.2, 0.3, 0.4) },
        tintColor2: { value: new THREE.Vector3(0.5, 0.6, 0.7) },
        tintColor3: { value: new THREE.Vector3(0.8, 0.9, 1) },
      };
      scene.add(buildMesh({ ...meshData, override_tints }, material));
    }
    updateTintUniforms(scene, 'item', { channel1: [1, 0, 1], channel2: [1, 1, 1], channel3: [1, 1, 1] });
    const materials = scene.children.map(obj => (obj as THREE.Mesh).material as THREE.MeshPhysicalMaterial);
    expect(materials[0].userData.tintUniforms.tintColor1.value.toArray()).toEqual([1, 0, 1]);
    expect(materials[1].userData.tintUniforms.tintColor1.value.toArray()).toEqual([0.2, 0.3, 0.4]);
    scene.children.forEach(obj => (obj as THREE.Mesh).geometry.dispose());
    materials.forEach(material => material.dispose());
  });

  it('reads the DXT5 gloss mean from the alpha block in both palette modes', () => {
    const block = (a0: number, a1: number, indices: number[]) => {
      const data = new Uint8Array(16);
      data[0] = a0;
      data[1] = a1;
      let bits = 0n;
      indices.forEach((idx, i) => { bits |= BigInt(idx) << BigInt(3 * i); });
      for (let i = 0; i < 6; i++) data[2 + i] = Number((bits >> BigInt(8 * i)) & 0xffn);
      return { mipmaps: [{ data, width: 4, height: 4 }], format: THREE.RGBA_S3TC_DXT5_Format };
    };
    // a0 > a1: 8-entry interpolated palette; index 1 = a1, index 7 = (a0 + 6 a1) / 7.
    expect(ddsMeanAlpha(block(255, 0, new Array(16).fill(1)))).toBeCloseTo(0);
    expect(ddsMeanAlpha(block(255, 0, new Array(16).fill(0)))).toBeCloseTo(1);
    expect(ddsMeanAlpha(block(255, 0, new Array(16).fill(7)))).toBeCloseTo(1 / 7);
    // a0 <= a1: 6-entry palette plus explicit 0 and 255.
    expect(ddsMeanAlpha(block(0, 0, new Array(16).fill(7)))).toBeCloseTo(1);
    expect(ddsMeanAlpha(block(0, 0, new Array(16).fill(6)))).toBeCloseTo(0);
    expect(ddsMeanAlpha(block(100, 200, new Array(16).fill(1)))).toBeCloseTo(200 / 255);
    expect(ddsMeanAlpha({ mipmaps: [{ data: new Uint8Array(8), width: 4, height: 4 }], format: THREE.RGB_S3TC_DXT1_Format })).toBe(1);
  });

  it('gates the 0x08 environment path on the normal-map gloss', async () => {
    const envMap = new THREE.Texture();
    const matte = await createMaterial({ ...materialData, normal_map: 'matte_normal', flags: 0x08 }, undefined, false, envMap);
    expect(matte.envMap).toBeNull();
    expect(matte.metalness).toBe(0);
    const glossy = await createMaterial({ ...materialData, normal_map: 'glossy_normal', flags: 0x08 }, undefined, false, envMap);
    expect(glossy.envMap).toBe(envMap);
    expect(glossy.metalness).toBeCloseTo(0.85);
    expect(glossy.envMapIntensity).toBeCloseTo(6);
    const unflagged = await createMaterial({ ...materialData, normal_map: 'glossy_normal', flags: 0 }, undefined, false, envMap);
    expect(unflagged.envMap).toBeNull();
    [matte, glossy, unflagged].forEach(material => material.dispose());
  });

  it('keeps cutouts in the depth-writing pass and honors blend flags', async () => {
    const cutout = await createMaterial({ ...materialData, flags: 1 });
    expect(cutout.alphaTest).toBe(0.5);
    expect(cutout.transparent).toBe(false);
    expect(cutout.depthWrite).toBe(true);
    const blend = await createMaterial({ ...materialData, flags: 2 });
    expect(blend.transparent).toBe(true);
    expect(blend.depthWrite).toBe(false);
    const additive = await createMaterial({ ...materialData, flags: 4 });
    expect(additive.blending).toBe(THREE.AdditiveBlending);
    [cutout, blend, additive].forEach(material => material.dispose());
  });
});
