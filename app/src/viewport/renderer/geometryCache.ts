// Geometry cache keyed by the core's geometry_key. Identical parts share one
// BufferGeometry; a key only ever maps to one immutable mesh.
import * as THREE from 'three';
import type { MeshData } from '../../core-api/types';

export interface CachedGeometry {
  surface: THREE.BufferGeometry;
  edges: THREE.BufferGeometry;
  /** triangle index → CAD face id */
  faceIds: Uint32Array;
  /** segment index → CAD edge id */
  edgeIds: Uint32Array;
  refs: number;
}

/** Box-projected UVs (presentation only) so wood textures follow the panel axes. */
function boxUv(pos: Float32Array, nrm: Float32Array): Float32Array {
  const uv = new Float32Array((pos.length / 3) * 2);
  for (let i = 0; i < pos.length / 3; i++) {
    const nx = Math.abs(nrm[i * 3]);
    const ny = Math.abs(nrm[i * 3 + 1]);
    const nz = Math.abs(nrm[i * 3 + 2]);
    const x = pos[i * 3];
    const y = pos[i * 3 + 1];
    const z = pos[i * 3 + 2];
    let u: number;
    let v: number;
    if (nz >= nx && nz >= ny) {
      u = x;
      v = y;
    } else if (nx >= ny) {
      u = z;
      v = y;
    } else {
      u = x;
      v = z;
    }
    uv[i * 2] = u / 600;
    uv[i * 2 + 1] = v / 1200;
  }
  return uv;
}

export class GeometryCache {
  private map = new Map<string, CachedGeometry>();

  has(key: string) {
    return this.map.has(key);
  }

  keys(): string[] {
    return Array.from(this.map.keys());
  }

  get(key: string) {
    return this.map.get(key);
  }

  put(key: string, m: MeshData) {
    if (this.map.has(key)) return;
    const surface = new THREE.BufferGeometry();
    const pos = new Float32Array(m.positions);
    const nrm = new Float32Array(m.normals);
    surface.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    surface.setAttribute('normal', new THREE.BufferAttribute(nrm, 3));
    surface.setAttribute('uv', new THREE.BufferAttribute(boxUv(pos, nrm), 2));
    surface.setIndex(new THREE.BufferAttribute(new Uint32Array(m.indices), 1));
    surface.computeBoundingBox();
    surface.computeBoundingSphere();
    const edges = new THREE.BufferGeometry();
    edges.setAttribute('position', new THREE.BufferAttribute(new Float32Array(m.edges), 3));
    edges.computeBoundingSphere();
    this.map.set(key, { surface, edges, faceIds: new Uint32Array(m.face_ids), edgeIds: new Uint32Array(m.edge_ids), refs: 0 });
  }

  retain(key: string) {
    const g = this.map.get(key);
    if (g) g.refs++;
  }

  release(key: string) {
    const g = this.map.get(key);
    if (!g) return;
    g.refs--;
  }

  /** Free geometries no object uses any more. */
  collect() {
    for (const [k, g] of this.map) {
      if (g.refs <= 0) {
        g.surface.dispose();
        g.edges.dispose();
        this.map.delete(k);
      }
    }
  }

  /** Triangles of one CAD face as a standalone geometry (face highlight). */
  faceGeometry(key: string, faceId: number): THREE.BufferGeometry | null {
    const g = this.map.get(key);
    if (!g) return null;
    const idx = g.surface.getIndex()!;
    const pos = g.surface.getAttribute('position') as THREE.BufferAttribute;
    const out: number[] = [];
    for (let t = 0; t < g.faceIds.length; t++) {
      if (g.faceIds[t] !== faceId) continue;
      for (let k = 0; k < 3; k++) {
        const vi = idx.getX(t * 3 + k);
        out.push(pos.getX(vi), pos.getY(vi), pos.getZ(vi));
      }
    }
    if (!out.length) return null;
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(out), 3));
    return geo;
  }

  edgeGeometry(key: string, edgeId: number): THREE.BufferGeometry | null {
    const g = this.map.get(key);
    if (!g) return null;
    const pos = g.edges.getAttribute('position') as THREE.BufferAttribute;
    const out: number[] = [];
    for (let s = 0; s < g.edgeIds.length; s++) {
      if (g.edgeIds[s] !== edgeId) continue;
      for (let k = 0; k < 2; k++) out.push(pos.getX(s * 2 + k), pos.getY(s * 2 + k), pos.getZ(s * 2 + k));
    }
    if (!out.length) return null;
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(out), 3));
    return geo;
  }
}
