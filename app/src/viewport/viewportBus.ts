// Imperative access to the live viewport (commands from ribbon/shortcuts).
import * as THREE from 'three';
import { create } from 'zustand';
import type { ObjectId, SceneTree } from '../core-api/types';
import type { StandardView, ViewportEngine } from './renderer/ViewportEngine';

let engine: ViewportEngine | null = null;

export const useSceneRevision = create<{ rev: number; bump: () => void }>((set, get) => ({
  rev: 0,
  bump: () => set({ rev: get().rev + 1 }),
}));

export function setEngine(e: ViewportEngine | null) {
  engine = e;
}

export function getEngine() {
  return engine;
}

/** All descendant ids (inclusive) of the given ids, from the tree read model. */
export function expandSubtrees(tree: SceneTree | null, ids: ObjectId[]): ObjectId[] {
  if (!tree) return ids;
  const out = new Set<ObjectId>();
  const want = new Set(ids);
  const walk = (n: SceneTree['roots'][number], inside: boolean) => {
    const on = inside || want.has(n.id);
    if (on) out.add(n.id);
    for (const c of n.children) walk(c, on);
  };
  for (const r of tree.roots) walk(r, false);
  for (const id of ids) out.add(id);
  return Array.from(out);
}

export const View = {
  set(v: StandardView, ids?: ObjectId[], tree?: SceneTree | null) {
    if (!engine) return;
    const box = ids && ids.length ? engine.boxOf(expandSubtrees(tree ?? null, ids)) : undefined;
    engine.setView(v, box);
  },
  fit(ids: ObjectId[], tree: SceneTree | null) {
    if (!engine) return;
    const box = ids.length ? engine.boxOf(expandSubtrees(tree, ids)) : engine.allBox();
    engine.fitBox(box.isEmpty() ? engine.allBox() : box);
  },
  fitAll() {
    engine?.fitBox(engine.allBox());
  },
  projection(p: 'perspective' | 'orthographic') {
    engine?.setProjection(p);
  },
  screenshot(): string | null {
    return engine ? engine.dom.toDataURL('image/png') : null;
  },
  box(ids: ObjectId[], tree: SceneTree | null): THREE.Box3 | null {
    if (!engine) return null;
    const b = engine.boxOf(expandSubtrees(tree, ids));
    return b.isEmpty() ? null : b;
  },
};
