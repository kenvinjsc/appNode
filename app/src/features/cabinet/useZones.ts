// Zone read model of the cabinet being designed (pinned cabinet, or the cabinet of the selection).
import { useEffect, useState } from 'react';
import { findNode, useUi } from '../../app/uiStore';
import { Queries } from '../../core-api/queries';
import type { ObjectId, SceneTree, ZonesInfo } from '../../core-api/types';

export function cabinetOf(tree: SceneTree | null, id: ObjectId | null): ObjectId | null {
  if (id === null) return null;
  let cur: ObjectId | null = id;
  while (cur !== null) {
    const f = findNode(tree, cur);
    if (!f) return null;
    if (f.node.kind === 'CABINET') return cur;
    cur = f.parent;
  }
  return null;
}

export function useCurrentCabinet(): ObjectId | null {
  const { pinned, tree, active } = useUi();
  return pinned.cabinet ?? cabinetOf(tree, active);
}

export function useZones(cabinet: ObjectId | null): ZonesInfo | null {
  const revision = useUi((s) => s.revision);
  const [info, setInfo] = useState<ZonesInfo | null>(null);
  useEffect(() => {
    let alive = true;
    if (cabinet === null) {
      setInfo(null);
      return;
    }
    Queries.zones(cabinet)
      .then((z) => alive && setInfo(z))
      .catch(() => alive && setInfo(null));
    return () => {
      alive = false;
    };
  }, [cabinet, revision]);
  return info;
}

/** Register the TAB action while a panel is shown. */
export function useTabAction(fn: () => void, deps: unknown[]) {
  useEffect(() => {
    useUi.getState().set({ tabAction: fn });
    return () => {
      if (useUi.getState().tabAction === fn) useUi.getState().set({ tabAction: null });
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}
