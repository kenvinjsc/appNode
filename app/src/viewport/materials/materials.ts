// Presentation materials. Texture/colour are render-only properties: they never
// decide anything about the logical board material.
import * as THREE from 'three';

export type VisualState = 'normal' | 'hover' | 'selected' | 'active' | 'locked';

const ACCENT = new THREE.Color('#e8590c');
const cache = new Map<string, THREE.Material>();
const textures = new Map<string, THREE.Texture>();

function hash(s: string) {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h >>> 0;
}

/** Procedural wood grain (lines along V) so no asset files are needed. */
function woodTexture(color: string): THREE.Texture {
  const hit = textures.get(color);
  if (hit) return hit;
  const c = document.createElement('canvas');
  c.width = 256;
  c.height = 512;
  const g = c.getContext('2d')!;
  const base = new THREE.Color(color);
  g.fillStyle = `#${base.getHexString()}`;
  g.fillRect(0, 0, c.width, c.height);
  let seed = hash(color);
  const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296);
  for (let i = 0; i < 90; i++) {
    const x = rnd() * c.width;
    const dark = base.clone().offsetHSL(0, 0.02, -0.06 - rnd() * 0.08);
    g.strokeStyle = `rgba(${Math.round(dark.r * 255)},${Math.round(dark.g * 255)},${Math.round(dark.b * 255)},${0.25 + rnd() * 0.35})`;
    g.lineWidth = 0.6 + rnd() * 2.2;
    g.beginPath();
    let px = x;
    g.moveTo(px, 0);
    for (let y = 0; y <= c.height; y += 16) {
      px += (rnd() - 0.5) * 3 + Math.sin(y / 60 + x) * 0.8;
      g.lineTo(px, y);
    }
    g.stroke();
  }
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 4;
  textures.set(color, t);
  return t;
}

export function isGrained(materialId: string | null): boolean {
  if (!materialId) return false;
  return /OAK|WALNUT|BIRCH|PLY/.test(materialId);
}

export function surfaceMaterial(color: string, state: VisualState, grained: boolean, clip: THREE.Plane[] = []): THREE.Material {
  const key = `${color}|${state}|${grained}|${clip.length}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const m = new THREE.MeshStandardMaterial({
    color: grained ? '#ffffff' : color,
    map: grained ? woodTexture(color) : null,
    roughness: 0.72,
    metalness: 0.0,
    polygonOffset: true,
    polygonOffsetFactor: 1,
    polygonOffsetUnits: 1,
    clippingPlanes: clip,
    clipShadows: true,
  });
  if (state === 'hover') m.emissive = ACCENT.clone().multiplyScalar(0.12);
  if (state === 'selected') m.emissive = ACCENT.clone().multiplyScalar(0.22);
  if (state === 'active') m.emissive = ACCENT.clone().multiplyScalar(0.32);
  if (state === 'locked') {
    m.color.lerp(new THREE.Color('#9aa0a6'), 0.45);
  }
  cache.set(key, m);
  return m;
}

/** Kiểu bề mặt phụ kiện do core gửi (`RenderObject.look`). */
export type HardwareLook = 'GLASS' | 'MIRROR' | null;

export function hardwareMaterial(look: HardwareLook = null): THREE.Material {
  const key = `hw:${look ?? ''}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const m =
    look === 'GLASS'
      ? // Kính: trong, hơi xanh, không ghi depth để thấy đồ phía sau.
        new THREE.MeshStandardMaterial({ color: '#a5d8ff', roughness: 0.05, metalness: 0.1, transparent: true, opacity: 0.28, depthWrite: false, side: THREE.DoubleSide })
      : look === 'MIRROR'
        ? new THREE.MeshStandardMaterial({ color: '#e9eef3', roughness: 0.04, metalness: 1.0 })
        : new THREE.MeshStandardMaterial({ color: '#ffffff', roughness: 0.35, metalness: 0.7 });
  cache.set(key, m);
  return m;
}

export function roomMaterial(): THREE.Material {
  const key = 'room';
  const hit = cache.get(key);
  if (hit) return hit;
  const m = new THREE.MeshStandardMaterial({ color: '#e9e7e2', roughness: 0.95, transparent: true, opacity: 0.55, depthWrite: false, side: THREE.DoubleSide });
  cache.set(key, m);
  return m;
}

export function edgeMaterial(state: VisualState): THREE.LineBasicMaterial {
  const key = `edge|${state}`;
  const hit = cache.get(key) as THREE.LineBasicMaterial | undefined;
  if (hit) return hit;
  const color = state === 'selected' || state === 'active' ? '#e8590c' : state === 'hover' ? '#f59f63' : '#4b4f55';
  const m = new THREE.LineBasicMaterial({ color, transparent: state === 'normal' || state === 'locked', opacity: state === 'normal' ? 0.55 : state === 'locked' ? 0.35 : 1 });
  cache.set(key, m);
  return m;
}

export const faceHighlightMaterial = new THREE.MeshBasicMaterial({
  color: '#e8590c',
  transparent: true,
  opacity: 0.45,
  depthTest: true,
  polygonOffset: true,
  polygonOffsetFactor: -2,
  polygonOffsetUnits: -2,
  side: THREE.DoubleSide,
});

export const ACCENT_COLOR = ACCENT;
