// Three.js viewport. Presentation only: meshes come from the core, placement
// comes from the core's world matrices, nothing here computes CAD geometry.
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { ObjectId, RenderBatch, RenderObject, ZonesInfo } from '../../core-api/types';
import { GeometryCache } from './geometryCache';
import { edgeMaterial, faceHighlightMaterial, hardwareMaterial, isGrained, roomMaterial, surfaceMaterial, type VisualState } from '../materials/materials';

export type StandardView = 'front' | 'back' | 'left' | 'right' | 'top' | 'iso';

interface Entry {
  ro: RenderObject;
  mesh: THREE.Mesh | null;
  edges: THREE.LineSegments | null;
  /** For instanced hardware. */
  instance: { key: string; index: number } | null;
  matrix: THREE.Matrix4;
}

export interface PickResult {
  id: ObjectId;
  faceId: number | null;
  edgeId: number | null;
  point: THREE.Vector3;
  normal: THREE.Vector3 | null;
}

class InstanceGroup {
  mesh: THREE.InstancedMesh;
  ids: ObjectId[] = [];
  constructor(geometry: THREE.BufferGeometry, capacity: number) {
    this.mesh = new THREE.InstancedMesh(geometry, hardwareMaterial(), capacity);
    this.mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.mesh.count = 0;
    this.mesh.frustumCulled = false;
  }
}

export class ViewportEngine {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly content = new THREE.Group();
  readonly overlay = new THREE.Group();
  readonly persp: THREE.PerspectiveCamera;
  readonly ortho: THREE.OrthographicCamera;
  camera: THREE.Camera;
  readonly controls: OrbitControls;
  readonly geometries = new GeometryCache();
  readonly entries = new Map<ObjectId, Entry>();
  private instances = new Map<string, InstanceGroup>();
  private raycaster = new THREE.Raycaster();
  private needsRender = true;
  private disposed = false;
  private grid: THREE.GridHelper;
  private faceHighlight: THREE.Mesh | null = null;
  private edgeHighlight: THREE.LineSegments | null = null;
  readonly clipPlane = new THREE.Plane(new THREE.Vector3(0, -1, 0), 1000);
  private clipping = false;
  private states = new Map<ObjectId, VisualState>();
  private isolation: Set<ObjectId> | null = null;
  readonly zoneGroup = new THREE.Group();
  private zoneMeshes: THREE.Mesh[] = [];
  onFrame: (() => void) | null = null;

  constructor(private container: HTMLElement) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.05;
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    container.appendChild(this.renderer.domElement);

    this.scene.background = new THREE.Color('#e9ecef');
    const w = container.clientWidth || 800;
    const h = container.clientHeight || 600;
    this.persp = new THREE.PerspectiveCamera(40, w / h, 10, 200000);
    this.persp.position.set(3200, 2400, 4200);
    this.ortho = new THREE.OrthographicCamera(-w, w, h, -h, -100000, 100000);
    this.camera = this.persp;

    this.controls = new OrbitControls(this.persp, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.12;
    this.controls.target.set(500, 900, 300);
    this.controls.mouseButtons = { LEFT: THREE.MOUSE.ROTATE, MIDDLE: THREE.MOUSE.DOLLY, RIGHT: THREE.MOUSE.PAN };
    this.controls.addEventListener('change', () => this.requestRender());

    // Lights
    this.scene.add(new THREE.HemisphereLight('#ffffff', '#b8b2a8', 1.1));
    const sun = new THREE.DirectionalLight('#ffffff', 1.6);
    sun.position.set(2500, 5000, 4000);
    sun.castShadow = true;
    sun.shadow.mapSize.set(2048, 2048);
    const sc = sun.shadow.camera as THREE.OrthographicCamera;
    sc.left = -5000;
    sc.right = 5000;
    sc.top = 5000;
    sc.bottom = -5000;
    sc.far = 20000;
    sun.shadow.bias = -0.0005;
    this.scene.add(sun);
    const fill = new THREE.DirectionalLight('#dfe8ff', 0.5);
    fill.position.set(-3000, 2000, -1500);
    this.scene.add(fill);

    // Ground, grid, axes
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(40000, 40000), new THREE.ShadowMaterial({ opacity: 0.12 }));
    ground.rotation.x = -Math.PI / 2;
    ground.position.y = -0.5;
    ground.receiveShadow = true;
    ground.name = 'ground';
    this.scene.add(ground);
    this.grid = new THREE.GridHelper(20000, 200, '#c5cad0', '#d9dde1');
    (this.grid.material as THREE.Material).transparent = true;
    (this.grid.material as THREE.Material).opacity = 0.8;
    this.scene.add(this.grid);
    const axes = new THREE.AxesHelper(400);
    axes.position.y = 0.5;
    this.scene.add(axes);

    this.scene.add(this.content);
    this.scene.add(this.overlay);
    this.scene.add(this.zoneGroup);

    const ro = new ResizeObserver(() => this.resize());
    ro.observe(container);
    this.resize();

    const loop = () => {
      if (this.disposed) return;
      requestAnimationFrame(loop);
      const moved = this.controls.update();
      if (moved || this.needsRender) {
        this.needsRender = false;
        this.renderer.render(this.scene, this.camera);
        this.onFrame?.();
      }
    };
    loop();
  }

  dispose() {
    this.disposed = true;
    this.controls.dispose();
    this.renderer.dispose();
    this.renderer.domElement.remove();
  }

  requestRender() {
    this.needsRender = true;
  }

  resize() {
    const w = this.container.clientWidth || 1;
    const h = this.container.clientHeight || 1;
    this.renderer.setSize(w, h, false);
    this.renderer.domElement.style.width = '100%';
    this.renderer.domElement.style.height = '100%';
    this.persp.aspect = w / h;
    this.persp.updateProjectionMatrix();
    this.updateOrthoFrustum();
    this.requestRender();
  }

  private updateOrthoFrustum() {
    const w = this.container.clientWidth || 1;
    const h = this.container.clientHeight || 1;
    const dist = this.persp.position.distanceTo(this.controls.target);
    const halfH = dist * Math.tan(THREE.MathUtils.degToRad(this.persp.fov / 2));
    this.ortho.left = (-halfH * w) / h;
    this.ortho.right = (halfH * w) / h;
    this.ortho.top = halfH;
    this.ortho.bottom = -halfH;
    this.ortho.updateProjectionMatrix();
  }

  setProjection(p: 'perspective' | 'orthographic') {
    if (p === 'orthographic') {
      this.updateOrthoFrustum();
      this.ortho.position.copy(this.persp.position);
      this.ortho.quaternion.copy(this.persp.quaternion);
      this.ortho.zoom = 1;
      this.ortho.updateProjectionMatrix();
      this.camera = this.ortho;
      this.controls.object = this.ortho;
    } else {
      this.persp.position.copy(this.ortho.position);
      this.camera = this.persp;
      this.controls.object = this.persp;
    }
    this.controls.update();
    this.requestRender();
  }

  // ------------------------------------------------------------- scene sync

  /** Apply a batch from the core: new meshes + object placements/states. */
  applyBatch(batch: RenderBatch) {
    for (const [key, m] of Object.entries(batch.meshes)) this.geometries.put(key, m);
    const touchedInstances = new Set<string>();
    for (const ro of batch.objects) {
      const old = this.entries.get(ro.id);
      const matrix = new THREE.Matrix4().fromArray(ro.matrix);
      if (old && old.ro.geometry_key === ro.geometry_key && old.ro.kind === ro.kind) {
        old.ro = ro;
        old.matrix = matrix;
        if (old.mesh) this.placeMesh(old);
        if (old.instance) touchedInstances.add(old.instance.key);
        continue;
      }
      if (old) this.removeEntry(ro.id, touchedInstances);
      const entry = this.createEntry(ro, matrix, touchedInstances);
      if (entry) this.entries.set(ro.id, entry);
    }
    for (const k of touchedInstances) this.syncInstances(k);
    this.refreshStates();
    this.requestRender();
  }

  private createEntry(ro: RenderObject, matrix: THREE.Matrix4, touched: Set<string>): Entry | null {
    const g = this.geometries.get(ro.geometry_key);
    if (!g) return null;
    this.geometries.retain(ro.geometry_key);
    if (ro.kind === 'HARDWARE') {
      let group = this.instances.get(ro.geometry_key);
      if (!group) {
        group = new InstanceGroup(g.surface, 64);
        group.mesh.userData.instanceKey = ro.geometry_key;
        this.instances.set(ro.geometry_key, group);
        this.content.add(group.mesh);
      }
      group.ids.push(ro.id);
      touched.add(ro.geometry_key);
      return { ro, mesh: null, edges: null, instance: { key: ro.geometry_key, index: group.ids.length - 1 }, matrix };
    }
    const mat = ro.kind === 'ROOM' ? roomMaterial() : surfaceMaterial(ro.color, 'normal', isGrained(ro.material_id), this.clipping ? [this.clipPlane] : []);
    const mesh = new THREE.Mesh(g.surface, mat);
    mesh.matrixAutoUpdate = false;
    mesh.userData.id = ro.id;
    mesh.castShadow = ro.kind === 'PANEL';
    mesh.receiveShadow = true;
    let edges: THREE.LineSegments | null = null;
    if (ro.kind === 'PANEL') {
      edges = new THREE.LineSegments(g.edges, edgeMaterial('normal'));
      edges.matrixAutoUpdate = false;
      edges.userData.id = ro.id;
      edges.raycast = () => undefined;
      this.content.add(edges);
    } else {
      mesh.renderOrder = -1;
    }
    this.content.add(mesh);
    const entry: Entry = { ro, mesh, edges, instance: null, matrix };
    this.placeMesh(entry);
    return entry;
  }

  private placeMesh(e: Entry) {
    if (!e.mesh) return;
    e.mesh.matrix.copy(e.matrix);
    e.mesh.matrixWorldNeedsUpdate = true;
    const vis = e.ro.visible && (!this.isolation || this.isolation.has(e.ro.id));
    e.mesh.visible = vis;
    if (e.edges) {
      e.edges.matrix.copy(e.matrix);
      e.edges.matrixWorldNeedsUpdate = true;
      e.edges.visible = vis;
    }
  }

  private syncInstances(key: string): void {
    const group = this.instances.get(key);
    if (!group) return;
    const ids = group.ids.filter((id) => this.entries.get(id)?.instance?.key === key || !this.entries.has(id));
    group.ids = ids.filter((id) => this.entries.has(id));
    if (group.ids.length > group.mesh.instanceMatrix.count) {
      const g = this.geometries.get(key)!;
      this.content.remove(group.mesh);
      group.mesh.dispose();
      const bigger = new InstanceGroup(g.surface, group.ids.length * 2);
      bigger.ids = group.ids;
      bigger.mesh.userData.instanceKey = key;
      this.instances.set(key, bigger);
      this.content.add(bigger.mesh);
      return this.syncInstances(key);
    }
    const zero = new THREE.Matrix4().makeScale(0, 0, 0);
    const color = new THREE.Color();
    group.ids.forEach((id, i) => {
      const e = this.entries.get(id)!;
      e.instance = { key, index: i };
      group.mesh.setMatrixAt(i, e.ro.visible && (!this.isolation || this.isolation.has(id)) ? e.matrix : zero);
      const st = this.states.get(id) ?? 'normal';
      color.set(st === 'selected' || st === 'active' ? '#f08c4a' : st === 'hover' ? '#c9a58c' : '#9aa1a8');
      group.mesh.setColorAt(i, color);
    });
    group.mesh.count = group.ids.length;
    group.mesh.instanceMatrix.needsUpdate = true;
    if (group.mesh.instanceColor) group.mesh.instanceColor.needsUpdate = true;
    group.mesh.computeBoundingSphere();
  }

  private removeEntry(id: ObjectId, touched: Set<string>) {
    const e = this.entries.get(id);
    if (!e) return;
    if (e.mesh) this.content.remove(e.mesh);
    if (e.edges) this.content.remove(e.edges);
    if (e.instance) touched.add(e.instance.key);
    this.geometries.release(e.ro.geometry_key);
    this.entries.delete(id);
  }

  remove(ids: ObjectId[]) {
    const touched = new Set<string>();
    for (const id of ids) this.removeEntry(id, touched);
    for (const k of touched) this.syncInstances(k);
    this.geometries.collect();
    this.requestRender();
  }

  clear() {
    this.remove(Array.from(this.entries.keys()));
    for (const g of this.instances.values()) this.content.remove(g.mesh);
    this.instances.clear();
  }

  // ------------------------------------------------------------ visual state

  setStates(states: Map<ObjectId, VisualState>) {
    this.states = states;
    this.refreshStates();
  }

  private refreshStates() {
    const clip = this.clipping ? [this.clipPlane] : [];
    const touched = new Set<string>();
    for (const e of this.entries.values()) {
      const st: VisualState = this.states.get(e.ro.id) ?? (e.ro.locked ? 'locked' : 'normal');
      if (e.mesh && e.ro.kind === 'PANEL') {
        e.mesh.material = surfaceMaterial(e.ro.color, st, isGrained(e.ro.material_id), clip);
        if (e.edges) e.edges.material = edgeMaterial(st);
      }
      if (e.instance) touched.add(e.instance.key);
    }
    for (const k of touched) this.syncInstances(k);
    this.requestRender();
  }

  setClipping(on: boolean, height?: number) {
    this.clipping = on;
    this.renderer.localClippingEnabled = on;
    if (height !== undefined) this.clipPlane.constant = height;
    this.refreshStates();
  }

  /** Show only these objects (cô lập tủ); null shows everything. */
  setIsolation(ids: ObjectId[] | null) {
    this.isolation = ids ? new Set(ids) : null;
    const touched = new Set<string>();
    for (const e of this.entries.values()) {
      this.placeMesh(e);
      if (e.instance) touched.add(e.instance.key);
    }
    for (const k of touched) this.syncInstances(k);
    this.requestRender();
  }

  /** Draw the cabinet's leaf zones (vùng lọt lòng) for pinning. */
  setZones(info: ZonesInfo | null, pinned: number[], hover: number | null) {
    for (const m of this.zoneMeshes) {
      this.zoneGroup.remove(m);
      m.geometry.dispose();
    }
    this.zoneMeshes = [];
    if (info) {
      const cab = new THREE.Matrix4().fromArray(info.matrix);
      for (const z of info.zones) {
        const isPinned = pinned.includes(z.id);
        if (!z.leaf && !isPinned) continue;
        const geo = new THREE.BoxGeometry(z.size[0], z.size[1], z.size[2]);
        const color = isPinned ? '#e8590c' : hover === z.id ? '#f59f63' : '#4dabf7';
        // Drawn through the fronts (depthTest off) so closed cabinets can still be pinned.
        const mat = new THREE.MeshBasicMaterial({ color, transparent: true, opacity: isPinned ? 0.3 : hover === z.id ? 0.22 : 0.06, depthWrite: false, depthTest: false });
        const m = new THREE.Mesh(geo, mat);
        m.matrixAutoUpdate = false;
        m.matrix.copy(cab).multiply(new THREE.Matrix4().makeTranslation(z.min[0] + z.size[0] / 2, z.min[1] + z.size[1] / 2, z.min[2] + z.size[2] / 2));
        m.userData.zoneId = z.id;
        m.userData.volume = z.size[0] * z.size[1] * z.size[2];
        m.renderOrder = 5;
        const edges = new THREE.LineSegments(new THREE.EdgesGeometry(geo), new THREE.LineBasicMaterial({ color, transparent: true, depthTest: false, opacity: isPinned || hover === z.id ? 0.95 : 0.4 }));
        edges.renderOrder = 6;
        edges.raycast = () => undefined;
        m.add(edges);
        this.zoneGroup.add(m);
        this.zoneMeshes.push(m);
      }
    }
    this.requestRender();
  }

  /** Smallest zone under the cursor. */
  pickZone(clientX: number, clientY: number): number | null {
    if (!this.zoneMeshes.length) return null;
    this.raycaster.setFromCamera(this.ndc(clientX, clientY), this.camera);
    const hits = this.raycaster.intersectObjects(this.zoneMeshes, false);
    if (!hits.length) return null;
    hits.sort((a, b) => a.object.userData.volume - b.object.userData.volume);
    return hits[0].object.userData.zoneId as number;
  }

  setGridVisible(v: boolean) {
    this.grid.visible = v;
    this.requestRender();
  }

  /** Preview matrices during a drag (visual only, never sent to the core). */
  previewMatrices(ids: ObjectId[], transform: THREE.Matrix4 | null) {
    const touched = new Set<string>();
    for (const id of ids) {
      const e = this.entries.get(id);
      if (!e) continue;
      const m = transform ? transform.clone().multiply(e.matrix) : e.matrix;
      if (e.mesh) {
        e.mesh.matrix.copy(m);
        e.mesh.matrixWorldNeedsUpdate = true;
      }
      if (e.edges) {
        e.edges.matrix.copy(m);
        e.edges.matrixWorldNeedsUpdate = true;
      }
      if (e.instance) {
        const g = this.instances.get(e.instance.key);
        g?.mesh.setMatrixAt(e.instance.index, m);
        touched.add(e.instance.key);
      }
    }
    for (const k of touched) {
      const g = this.instances.get(k)!;
      g.mesh.instanceMatrix.needsUpdate = true;
    }
    this.requestRender();
  }

  highlightFace(id: ObjectId | null, faceId: number | null) {
    if (this.faceHighlight) {
      this.overlay.remove(this.faceHighlight);
      this.faceHighlight.geometry.dispose();
      this.faceHighlight = null;
    }
    if (id !== null && faceId !== null) {
      const e = this.entries.get(id);
      const geo = e && this.geometries.faceGeometry(e.ro.geometry_key, faceId);
      if (e && geo) {
        this.faceHighlight = new THREE.Mesh(geo, faceHighlightMaterial);
        this.faceHighlight.matrixAutoUpdate = false;
        this.faceHighlight.matrix.copy(e.matrix);
        this.faceHighlight.raycast = () => undefined;
        this.overlay.add(this.faceHighlight);
      }
    }
    this.requestRender();
  }

  highlightEdge(id: ObjectId | null, edgeId: number | null) {
    if (this.edgeHighlight) {
      this.overlay.remove(this.edgeHighlight);
      this.edgeHighlight.geometry.dispose();
      this.edgeHighlight = null;
    }
    if (id !== null && edgeId !== null) {
      const e = this.entries.get(id);
      const geo = e && this.geometries.edgeGeometry(e.ro.geometry_key, edgeId);
      if (e && geo) {
        this.edgeHighlight = new THREE.LineSegments(geo, new THREE.LineBasicMaterial({ color: '#e8590c', depthTest: false }));
        this.edgeHighlight.matrixAutoUpdate = false;
        this.edgeHighlight.matrix.copy(e.matrix);
        this.edgeHighlight.renderOrder = 10;
        this.overlay.add(this.edgeHighlight);
      }
    }
    this.requestRender();
  }

  // ----------------------------------------------------------------- picking

  private ndc(clientX: number, clientY: number) {
    const r = this.renderer.domElement.getBoundingClientRect();
    return new THREE.Vector2(((clientX - r.left) / r.width) * 2 - 1, -((clientY - r.top) / r.height) * 2 + 1);
  }

  pick(clientX: number, clientY: number, opts: { includeRoom?: boolean; edges?: boolean } = {}): PickResult | null {
    this.raycaster.setFromCamera(this.ndc(clientX, clientY), this.camera);
    const targets: THREE.Object3D[] = [];
    for (const e of this.entries.values()) {
      if (e.mesh && e.mesh.visible && (opts.includeRoom || e.ro.kind !== 'ROOM')) targets.push(e.mesh);
    }
    for (const g of this.instances.values()) targets.push(g.mesh);
    const hits = this.raycaster.intersectObjects(targets, false);
    for (const h of hits) {
      if (this.clipping && h.point.y > this.clipPlane.constant) continue;
      let id: ObjectId | undefined = h.object.userData.id;
      if (id === undefined && h.instanceId !== undefined) {
        const key = h.object.userData.instanceKey as string;
        id = this.instances.get(key)?.ids[h.instanceId];
      }
      if (id === undefined) continue;
      const e = this.entries.get(id)!;
      const g = this.geometries.get(e.ro.geometry_key);
      const faceId = g && h.faceIndex !== undefined && h.faceIndex !== null ? g.faceIds[h.faceIndex] ?? null : null;
      const normal = h.face ? h.face.normal.clone().transformDirection(e.matrix) : null;
      let edgeId: number | null = null;
      if (opts.edges && g && e.edges) {
        edgeId = this.nearestEdge(e, h.point);
      }
      return { id, faceId, edgeId, point: h.point.clone(), normal };
    }
    return null;
  }

  private nearestEdge(e: Entry, p: THREE.Vector3): number | null {
    const g = this.geometries.get(e.ro.geometry_key)!;
    const pos = g.edges.getAttribute('position') as THREE.BufferAttribute;
    const a = new THREE.Vector3();
    const b = new THREE.Vector3();
    const line = new THREE.Line3();
    const q = new THREE.Vector3();
    let best = Infinity;
    let id: number | null = null;
    for (let s = 0; s < g.edgeIds.length; s++) {
      a.fromBufferAttribute(pos, s * 2).applyMatrix4(e.matrix);
      b.fromBufferAttribute(pos, s * 2 + 1).applyMatrix4(e.matrix);
      line.set(a, b);
      line.closestPointToPoint(p, true, q);
      const d = q.distanceTo(p);
      if (d < best) {
        best = d;
        id = g.edgeIds[s];
      }
    }
    // Only if the click is close to the edge in world terms (scaled by distance).
    const tol = this.camera === this.persp ? this.persp.position.distanceTo(p) * 0.012 : 15;
    return best < tol ? id : null;
  }

  /** Point on the ground plane (y = 0) under the cursor. */
  groundPoint(clientX: number, clientY: number): THREE.Vector3 | null {
    this.raycaster.setFromCamera(this.ndc(clientX, clientY), this.camera);
    const p = new THREE.Vector3();
    return this.raycaster.ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0, 1, 0), 0), p) ? p : null;
  }

  /** Ids whose screen-space bbox centre lies inside the rectangle (client px). */
  boxSelect(x0: number, y0: number, x1: number, y1: number): ObjectId[] {
    const r = this.renderer.domElement.getBoundingClientRect();
    const [minX, maxX] = [Math.min(x0, x1), Math.max(x0, x1)];
    const [minY, maxY] = [Math.min(y0, y1), Math.max(y0, y1)];
    const out: ObjectId[] = [];
    const v = new THREE.Vector3();
    for (const e of this.entries.values()) {
      if (!e.ro.visible || e.ro.kind === 'ROOM') continue;
      const box = this.worldBox(e.ro.id);
      if (!box) continue;
      box.getCenter(v).project(this.camera);
      const sx = r.left + ((v.x + 1) / 2) * r.width;
      const sy = r.top + ((1 - v.y) / 2) * r.height;
      if (sx >= minX && sx <= maxX && sy >= minY && sy <= maxY && v.z < 1) out.push(e.ro.id);
    }
    return out;
  }

  // --------------------------------------------------------------- bounds

  worldBox(id: ObjectId): THREE.Box3 | null {
    const e = this.entries.get(id);
    if (!e) return null;
    const g = this.geometries.get(e.ro.geometry_key);
    if (!g || !g.surface.boundingBox) return null;
    return g.surface.boundingBox.clone().applyMatrix4(e.matrix);
  }

  boxOf(ids: ObjectId[]): THREE.Box3 {
    const b = new THREE.Box3();
    for (const id of ids) {
      const w = this.worldBox(id);
      if (w) b.union(w);
    }
    return b;
  }

  allBox(): THREE.Box3 {
    const b = new THREE.Box3();
    for (const e of this.entries.values()) {
      if (!e.ro.visible) continue;
      const w = this.worldBox(e.ro.id);
      if (w) b.union(w);
    }
    return b;
  }

  // ---------------------------------------------------------------- camera

  fitBox(box: THREE.Box3, view?: StandardView) {
    if (box.isEmpty()) return;
    const center = box.getCenter(new THREE.Vector3());
    const size = box.getSize(new THREE.Vector3());
    const radius = Math.max(size.length() / 2, 200);
    const dir = view ? this.viewDir(view) : this.persp.position.clone().sub(this.controls.target).normalize();
    const dist = radius / Math.sin(THREE.MathUtils.degToRad(this.persp.fov / 2)) * 1.05;
    this.controls.target.copy(center);
    this.persp.position.copy(center).addScaledVector(dir, dist);
    this.persp.up.set(0, 1, 0);
    if (view === 'top') this.persp.up.set(0, 0, -1);
    this.persp.lookAt(center);
    if (this.camera === this.ortho) {
      this.ortho.position.copy(this.persp.position);
      this.ortho.quaternion.copy(this.persp.quaternion);
      this.ortho.up.copy(this.persp.up);
      this.updateOrthoFrustum();
      this.ortho.zoom = 1;
      this.ortho.updateProjectionMatrix();
    }
    this.controls.update();
    this.requestRender();
  }

  private viewDir(v: StandardView): THREE.Vector3 {
    switch (v) {
      case 'front':
        return new THREE.Vector3(0, 0, 1);
      case 'back':
        return new THREE.Vector3(0, 0, -1);
      case 'left':
        return new THREE.Vector3(-1, 0, 0);
      case 'right':
        return new THREE.Vector3(1, 0, 0);
      case 'top':
        return new THREE.Vector3(0, 1, 0.0001).normalize();
      default:
        return new THREE.Vector3(0.62, 0.45, 0.85).normalize();
    }
  }

  setView(v: StandardView, box?: THREE.Box3) {
    this.fitBox(box && !box.isEmpty() ? box : this.allBox(), v);
  }

  /** World → client pixel coordinates (for HTML overlays). */
  toScreen(p: THREE.Vector3): { x: number; y: number; visible: boolean } {
    const v = p.clone().project(this.camera);
    const w = this.container.clientWidth;
    const h = this.container.clientHeight;
    return { x: ((v.x + 1) / 2) * w, y: ((1 - v.y) / 2) * h, visible: v.z > -1 && v.z < 1 };
  }

  get dom() {
    return this.renderer.domElement;
  }
}
