import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import { localFromWorld } from './TransformGizmo';

describe('localFromWorld', () => {
  it('matches the core rotation convention (R = Rx*Ry*Rz == Euler XYZ)', () => {
    const q = new THREE.Quaternion().setFromEuler(new THREE.Euler(0, THREE.MathUtils.degToRad(90), 0, 'XYZ'));
    const world = new THREE.Matrix4().compose(new THREE.Vector3(100, 0, 560), q, new THREE.Vector3(1, 1, 1));
    const t = localFromWorld(world, new THREE.Matrix4());
    expect(t.translation).toEqual([100, 0, 560]);
    expect(t.rotation_deg).toEqual([0, 90, 0]);
  });

  it('removes the parent transform', () => {
    const parent = new THREE.Matrix4().makeTranslation(1000, 0, 0);
    const world = new THREE.Matrix4().makeTranslation(1018, 100, 0);
    expect(localFromWorld(world, parent).translation).toEqual([18, 100, 0]);
  });
});
