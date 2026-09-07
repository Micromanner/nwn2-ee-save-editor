import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import { buildMesh } from '../meshBuilder';
import { createMaterial, updateTintUniforms } from '../materials';
import type { MeshData } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => {
    const buffer = new ArrayBuffer(136);
    const header = new Int32Array(buffer, 0, 31);
    header[0] = 0x20534444;
    header[1] = 124;
    header[3] = 4;
    header[4] = 4;
    header[20] = 4;
    header[21] = 0x31545844;
    return buffer;
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
    expect(compactShader).toContain('diffuseColor.rgb = mix(diffuseColor.rgb, diffuseColor.rgb * tint, tintMask.a);');
    expect(shader.uniforms.tintColor2.value.toArray()).toEqual(colors.channel2);
    expect(shader.uniforms.nwn2GameStyle).toBe(material.userData.lightingUniforms.nwn2GameStyle);
    expect(shader.fragmentShader).toContain('#define RE_Direct RE_Direct_NWN2');
    expect(material.customProgramCacheKey()).toBe(`nwn2tint_${swapGB ? 'gb' : 'rgb'}_nwn2lighting_v2`);
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
    expect(uniforms.nwn2Specular.value.r).toBeCloseTo(0.405);
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
