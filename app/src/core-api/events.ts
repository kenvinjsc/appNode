// Core events fan-out. Subscribers update incrementally (never a full reload
// except on ProjectLoaded).
import type { CoreEvent, Response } from './types';

type Listener = (events: CoreEvent[]) => void;
const listeners = new Set<Listener>();

export function onCoreEvents(l: Listener): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function emitCoreEvents(events: CoreEvent[]) {
  if (events.length === 0) return;
  for (const l of listeners) l(events);
}

type StatusListener = (s: { canUndo: boolean; canRedo: boolean; revision: number }) => void;
const statusListeners = new Set<StatusListener>();

export function onHistoryStatus(l: StatusListener): () => void {
  statusListeners.add(l);
  return () => statusListeners.delete(l);
}

export function emitStatus(r: Response<unknown>) {
  for (const l of statusListeners) l({ canUndo: r.can_undo, canRedo: r.can_redo, revision: r.revision });
}
