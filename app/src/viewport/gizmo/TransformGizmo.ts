// Move/rotate gizmo. While dragging, only a *preview* matrix is shown. On
// mouse-up a single SetTransform command goes to the core, which answers with
// the new scene state (TransformChanged events).
import * as THREE from 'three';
import { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import type { ObjectId, SnapHint, Transform3D, Vec3 } from '../../core-api/types';
import { transformOf, type TransformInfo } from '../../core-api/queries';
import type { ViewportEngine } from '../renderer/ViewportEngine';

export interface GizmoCallbacks {
  snap: (id: ObjectId, delta: Vec3) => Promise<{ delta: Vec3; hints: SnapHint[] } | null>;
  commit: (id: ObjectId, t: Transform3D) => void;
  hints: (h: SnapHint[] | null, delta: Vec3 | null) => void;
  dragging: (on: boolean) => void;
}

/** Local transform from a world matrix given the parent's world matrix. */
export function localFromWorld(world: THREE.Matrix4, parentWorld: THREE.Matrix4): Transform3D {
  const local = parentWorld.clone().invert().multiply(world);
  const p = new THREE.Vector3();
  const q = new THREE.Quaternion();
  const s = new THREE.Vector3();
  local.decompose(p, q, s);
  // Core rotation is R = Rx * Ry * Rz == three.js Euler order 'XYZ'.
  const e = new THREE.Euler().setFromQuaternion(q, 'XYZ');
  const r = (v: number) => Math.round(v * 1000) / 1000;
  const d = (v: number) => {
    const x = r(THREE.MathUtils.radToDeg(v));
    return Object.is(x, -0) ? 0 : x;
  };
  return { translation: [r(p.x), r(p.y), r(p.z)], rotation_deg: [d(e.x), d(e.y), d(e.z)] };
}

export class TransformGizmo {
  readonly controls: TransformControls;
  private proxy = new THREE.Object3D();
  private target: {
    id: ObjectId;
    preview: ObjectId[];
    info: TransformInfo;
    startProxy: THREE.Matrix4;
    startWorld: THREE.Matrix4;
    parentWorld: THREE.Matrix4;
  } | null = null;
  private snapped: Vec3 | null = null;
  private snapSeq = 0;
  snapEnabled = true;

  constructor(private engine: ViewportEngine, private cb: GizmoCallbacks) {
    this.controls = new TransformControls(engine.camera, engine.dom);
    this.controls.setSize(0.9);
    this.controls.setSpace('world');
    engine.scene.add(this.proxy);
    engine.scene.add(this.controls.getHelper());
    this.controls.addEventListener('dragging-changed', (e) => {
      const on = Boolean((e as unknown as { value: boolean }).value);
      engine.controls.enabled = !on;
      cb.dragging(on);
      if (!on) this.finish();
    });
    this.controls.addEventListener('objectChange', () => this.onChange());
    this.controls.addEventListener('change', () => engine.requestRender());
  }

  setCamera(c: THREE.Camera) {
    this.controls.camera = c;
  }

  async attach(id: ObjectId, preview: ObjectId[], mode: 'translate' | 'rotate', pivot: THREE.Vector3) {
    const info = await transformOf(id);
    this.proxy.position.copy(pivot);
    this.proxy.quaternion.identity();
    this.proxy.updateMatrixWorld(true);
    this.target = {
      id,
      preview,
      info,
      startProxy: this.proxy.matrixWorld.clone(),
      startWorld: new THREE.Matrix4().fromArray(info.world_matrix),
      parentWorld: new THREE.Matrix4().fromArray(info.parent_matrix),
    };
    this.controls.setMode(mode);
    this.controls.showY = true;
    if (mode === 'rotate') {
      // Furniture rotates about the vertical axis in practice; keep X/Z available but Y first.
      this.controls.showX = true;
      this.controls.showZ = true;
    }
    this.controls.attach(this.proxy);
    this.engine.requestRender();
  }

  detach() {
    this.controls.detach();
    this.target = null;
    this.cb.hints(null, null);
    this.engine.requestRender();
  }

  get attachedId() {
    return this.target?.id ?? null;
  }

  private deltaMatrix(): THREE.Matrix4 {
    const t = this.target!;
    this.proxy.updateMatrixWorld(true);
    return this.proxy.matrixWorld.clone().multiply(t.startProxy.clone().invert());
  }

  private onChange() {
    const t = this.target;
    if (!t) return;
    if (this.controls.mode === 'translate') {
      const start = new THREE.Vector3().setFromMatrixPosition(t.startProxy);
      const raw = this.proxy.position.clone().sub(start);
      const delta: Vec3 = [raw.x, raw.y, raw.z];
      this.snapped = null;
      if (this.snapEnabled) {
        const seq = ++this.snapSeq;
        void this.cb.snap(t.id, delta).then((r) => {
          if (!r || seq !== this.snapSeq || !this.target) return;
          this.snapped = r.delta;
          const m = new THREE.Matrix4().makeTranslation(r.delta[0], r.delta[1], r.delta[2]);
          this.engine.previewMatrices(t.preview, m);
          this.cb.hints(r.hints, r.delta);
        });
      } else {
        this.cb.hints([], delta);
      }
      this.engine.previewMatrices(t.preview, this.deltaMatrix());
    } else {
      // Snap rotation to 15° steps for predictable furniture placement.
      const e = new THREE.Euler().setFromQuaternion(this.proxy.quaternion, 'XYZ');
      const step = THREE.MathUtils.degToRad(15);
      e.set(Math.round(e.x / step) * step, Math.round(e.y / step) * step, Math.round(e.z / step) * step);
      this.proxy.quaternion.setFromEuler(e);
      this.engine.previewMatrices(t.preview, this.deltaMatrix());
    }
  }

  private finish() {
    const t = this.target;
    if (!t) return;
    let m = this.deltaMatrix();
    if (this.controls.mode === 'translate' && this.snapped) {
      m = new THREE.Matrix4().makeTranslation(this.snapped[0], this.snapped[1], this.snapped[2]);
    }
    const isIdentity = m.equals(new THREE.Matrix4());
    this.engine.previewMatrices(t.preview, null);
    this.cb.hints(null, null);
    if (!isIdentity) {
      const newWorld = m.multiply(t.startWorld);
      this.cb.commit(t.id, localFromWorld(newWorld, t.parentWorld));
    }
    this.snapped = null;
  }
}
