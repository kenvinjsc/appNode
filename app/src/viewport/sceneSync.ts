// Incremental scene synchronisation driven by core events. Only the objects
// named in an event are re-queried; meshes already on the GPU (by geometry
// key) are never re-sent.
import { onCoreEvents } from '../core-api/events';
import { Queries } from '../core-api/queries';
import type { CoreEvent, ObjectId } from '../core-api/types';
import { useUi } from '../app/uiStore';
import { getEngine, useSceneRevision } from './viewportBus';

export async function refreshTree() {
  const tree = await Queries.sceneTree();
  useUi.getState().set({ tree });
}

export async function syncAll() {
  const engine = getEngine();
  if (!engine) return;
  const batch = await Queries.renderObjects(null, engine.geometries.keys());
  engine.clear();
  engine.applyBatch(batch);
  useSceneRevision.getState().bump();
}

async function syncIds(ids: ObjectId[]) {
  const engine = getEngine();
  if (!engine || ids.length === 0) return;
  const batch = await Queries.renderObjects(ids, engine.geometries.keys());
  engine.applyBatch(batch);
  useSceneRevision.getState().bump();
}

export function handleEvents(events: CoreEvent[]) {
  const engine = getEngine();
  const dirty = new Set<ObjectId>();
  let tree = false;
  for (const ev of events) {
    switch (ev.type) {
      case 'ProjectLoaded':
        useUi.getState().clearSelection();
        void syncAll();
        void refreshTree();
        return;
      case 'ObjectDeleted':
        engine?.remove(ev.ids);
        useSceneRevision.getState().bump();
        break;
      case 'SelectionInvalidated': {
        const s = useUi.getState();
        const gone = new Set(ev.ids);
        const next = s.selection.filter((i) => !gone.has(i));
        if (next.length !== s.selection.length) s.select(next);
        break;
      }
      case 'SceneTreeChanged':
        tree = true;
        break;
      case 'ObjectCreated':
      case 'ObjectChanged':
      case 'GeometryChanged':
      case 'TransformChanged':
        ev.ids.forEach((i) => dirty.add(i));
        break;
    }
  }
  if (dirty.size) void syncIds(Array.from(dirty));
  if (tree || dirty.size) void refreshTree();
}

let installed = false;
export function installSceneSync() {
  if (installed) return;
  installed = true;
  onCoreEvents(handleEvents);
}
