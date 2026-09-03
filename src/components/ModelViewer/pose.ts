import * as THREE from 'three';
import type { PreviewPose } from '@/lib/bindings';

// Shared posing rules for the item preview and the thumbnail gallery. Both
// render the same model, so the rotation and the camera placement live here
// rather than being duplicated (and drifting) in each renderer.
//
// Item MDBs are authored in their attachment orientation:
// - weapons already stand up; they are only yawed off-axis so flat blades
//   don't turn into razor-thin silhouettes under grazing light.
// - shields lie flat with their face pointing up, so they get pitched upright.
// - armor is a skinned full-body form viewed from a fixed front profile.

export function applyPoseRotation(object: THREE.Object3D, pose: PreviewPose): void {
  switch (pose) {
    case 'shield':
      object.rotation.set(-Math.PI / 2, 0, 0);
      break;
    case 'weapon':
      object.rotation.set(0, Math.PI / 5, 0);
      break;
    case 'armor':
      object.rotation.set(0, 0, 0);
      break;
  }
}

/// Camera position for a posed model. Weapons get a three-quarter view; armor
/// and shields are viewed straight on from -Z with a slight upward tilt.
export function poseCameraPosition(
  pose: PreviewPose,
  center: THREE.Vector3,
  distance: number,
): THREE.Vector3 {
  if (pose === 'weapon') {
    return new THREE.Vector3(
      center.x + distance,
      center.y + distance * 0.25,
      center.z - distance * 0.6,
    );
  }
  return new THREE.Vector3(center.x, center.y + distance * 0.1, center.z - distance);
}

/// Fallback for callers with no backend pose hint: skinned models are armor,
/// everything else is treated as a weapon.
export function poseFromModel(hasSkeleton: boolean): PreviewPose {
  return hasSkeleton ? 'armor' : 'weapon';
}
