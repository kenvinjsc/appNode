// Zone read model of the cabinet being designed (pinned cabinet, or the cabinet of the selection).
import { useEffect, useState } from 'react';
import { findNode, useUi } from '../../app/uiStore';
import { Queries } from '../../core-api/queries';
import type { CoreEvent, ObjectId, SceneTree, ZonesInfo } from '../../core-api/types';
import { onCoreEvents } from '../../core-api/events';

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
  // The selected cabinet wins over a zone pinned earlier in another cabinet.
  return cabinetOf(tree, active) ?? pinned.cabinet;
}

/** Sự kiện core có làm thay đổi khoang (hoặc vị trí / tên) của tủ `cabinet` không. */
export function touchesZones(events: CoreEvent[], cabinet: ObjectId): boolean {
  return events.some((e) => {
    switch (e.type) {
      case 'ProjectLoaded':
        return true;
      case 'ZonesChanged':
        return e.cabinets.includes(cabinet);
      case 'TransformChanged':
      case 'ObjectChanged':
      case 'ObjectDeleted':
        return e.ids.includes(cabinet);
      default:
        return false;
    }
  });
}

/** Khoang của tủ: tải một lần, sau đó chỉ tải lại khi core báo khoang / vị trí của chính tủ này đổi
 * (sửa tủ khác, dời tủ khác, đổi vật liệu… không làm 2D tải lại). */
export function useZones(cabinet: ObjectId | null): ZonesInfo | null {
  const [info, setInfo] = useState<ZonesInfo | null>(null);
  const [tick, setTick] = useState(0);
  useEffect(() => {
    if (cabinet === null) return;
    return onCoreEvents((events) => {
      if (touchesZones(events, cabinet)) setTick((t) => t + 1);
    });
  }, [cabinet]);
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
  }, [cabinet, tick]);
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
