import * as THREE from 'three';
import { createMaterial, tintChannelsToColors, type TintColors } from './materials';
import { buildMesh, buildSkeleton } from './meshBuilder';
import type { ModelData } from './types';

// Shared offscreen thumbnail renderer: ONE WebGL context for every thumbnail
// in the app (browser contexts are limited to ~8-16), rendering one frame per
// model to a detached canvas and returning a PNG data URL. No composer, GTAO,
// PMREM, or shadows - invisible at thumbnail size and expensive per frame.
// Model-agnostic on purpose: items today, character heads/hair and portraits
// can reuse it later.

const THUMBNAIL_SIZE = 224;

interface RendererState {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
}

let state: RendererState | null = null;

function initRenderer(): RendererState {
  const canvas = document.createElement('canvas');
  canvas.width = THUMBNAIL_SIZE;
  canvas.height = THUMBNAIL_SIZE;

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    preserveDrawingBuffer: true,
  });
  renderer.setSize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, false);
  renderer.setPixelRatio(1);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 0.8;

  // On context loss the renderer is unusable; drop the singleton so the next
  // job rebuilds it from scratch.
  canvas.addEventListener(
    'webglcontextlost',
    (e) => {
      e.preventDefault();
      state = null;
    },
    { once: true },
  );

  const scene = new THREE.Scene();
  const sun = new THREE.DirectionalLight(0xffe0b2, 1.5);
  sun.position.set(250, 500, -400);
  scene.add(sun);
  const back = new THREE.DirectionalLight(0xffffff, 1.5);
  back.position.set(-250, 500, 400);
  scene.add(back);
  const hemi = new THREE.HemisphereLight(0xffffff, 0xffffff, 0.6);
  scene.add(hemi);

  const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 10000);

  return { renderer, scene, camera };
}

function frameCamera(
  camera: THREE.PerspectiveCamera,
  target: THREE.Object3D,
  isSkinnedArmor: boolean,
): void {
  const box = new THREE.Box3().setFromObject(target);
  const center = box.getCenter(new THREE.Vector3());
  const size = box.getSize(new THREE.Vector3());
  const distance = Math.max(size.x, size.y, size.z) * 1.6;

  if (isSkinnedArmor) {
    camera.position.set(center.x, center.y + distance * 0.1, center.z - distance);
  } else {
    camera.position.set(center.x + distance, center.y + distance * 0.25, center.z - distance * 0.6);
  }
  camera.lookAt(center);
}

function disposeGroup(group: THREE.Group): void {
  group.traverse((obj) => {
    if (!(obj instanceof THREE.Mesh || obj instanceof THREE.SkinnedMesh)) return;
    obj.geometry.dispose();
    const mat = obj.material as THREE.MeshPhysicalMaterial;
    mat.map?.dispose();
    mat.normalMap?.dispose();
    mat.emissiveMap?.dispose();
    const uniforms = (mat.userData as Record<string, unknown>).tintUniforms as
      | { tintMap?: { value: THREE.Texture } }
      | undefined;
    uniforms?.tintMap?.value?.dispose();
    mat.dispose();
  });
}

/// Renders one model to a PNG data URL. Returns '' when the model has no
/// meshes or the WebGL context is lost. Callers must serialize invocations
/// (the render queue in useModelThumbnail) - the scene/canvas are shared.
export async function renderModelThumbnail(
  modelData: ModelData,
  tints: TintColors,
): Promise<string> {
  if (modelData.meshes.length === 0) return '';

  const { renderer, scene, camera } = (state ??= initRenderer());

  const group = new THREE.Group();
  let skeleton: THREE.Skeleton | undefined;
  let rootBone: THREE.Bone | undefined;
  if (modelData.skeleton) {
    const built = buildSkeleton(modelData.skeleton);
    skeleton = built.skeleton;
    rootBone = built.rootBone;
    group.add(rootBone);
  }

  try {
    const visible = modelData.meshes.filter((m) => !/_L\d+$/i.test(m.name));
    const materials = await Promise.all(
      visible.map((m) =>
        createMaterial(
          m.material,
          m.override_tints ? tintChannelsToColors(m.override_tints) : tints,
          true,
          null,
        ),
      ),
    );
    for (let i = 0; i < visible.length; i++) {
      group.add(buildMesh(visible[i], materials[i], skeleton, rootBone));
    }

    // Same posing conventions as ItemViewer3D: yaw flat weapons off-axis,
    // face skinned armor bodies straight on.
    const isSkinnedArmor = modelData.skeleton != null;
    group.rotation.y = isSkinnedArmor ? 0 : Math.PI / 5;

    scene.add(group);
    frameCamera(camera, group, isSkinnedArmor);
    renderer.render(scene, camera);
    if (renderer.getContext().isContextLost()) return '';
    return renderer.domElement.toDataURL('image/png');
  } finally {
    scene.remove(group);
    disposeGroup(group);
    // Frees the boneTexture the renderer allocates for skinned meshes -
    // without this every armor thumbnail leaks one GPU texture.
    skeleton?.dispose();
  }
}
