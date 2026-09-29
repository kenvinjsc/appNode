// Project organisation: floors (tầng) → rooms (phòng) → cabinets. Floor and room are
// cabinet attributes kept by the core; tabs here only filter the view.
import type { ObjectId, SceneTree, TreeNode } from '../../core-api/types';

const cabinets = (tree: SceneTree | null) => (tree?.roots ?? []).filter((r) => r.kind === 'CABINET');
const floorOf = (n: TreeNode) => n.floor ?? '';
const roomOf = (n: TreeNode) => n.room ?? '';

function counted(keys: string[], extra: string[]): { name: string; count: number }[] {
  const m = new Map<string, number>();
  for (const k of keys) m.set(k, (m.get(k) ?? 0) + 1);
  for (const e of extra) if (!m.has(e)) m.set(e, 0);
  return [...m.entries()].map(([name, count]) => ({ name, count }));
}

/** Floors in order of first appearance, then user-added ones. */
export function projectFloors(tree: SceneTree | null, extra: string[]) {
  return counted(cabinets(tree).map(floorOf), extra);
}

/** Rooms of a floor (null = every floor). `extra` entries are "floor/room" keys. */
export function projectRooms(tree: SceneTree | null, floor: string | null, extra: string[]) {
  const onFloor = cabinets(tree).filter((c) => floor === null || floorOf(c) === floor);
  const ex = extra.filter((k) => floor === null || k.split('/')[0] === floor).map((k) => k.slice(k.indexOf('/') + 1));
  return counted(onFloor.map(roomOf), ex);
}

export const roomLabel = (r: string) => (r === '' ? 'Chưa gán' : r);
export const extraKey = (floor: string | null, room: string) => `${floor ?? ''}/${room}`;

/** Top-level nodes shown for the floor/room tabs (null = all). */
export function roomRoots(tree: SceneTree | null, floor: string | null, room: string | null): TreeNode[] {
  if (floor === null && room === null) return tree?.roots ?? [];
  return cabinets(tree).filter((c) => (floor === null || floorOf(c) === floor) && (room === null || roomOf(c) === room));
}

/** Every id under the filtered cabinets plus the room shells (for viewport isolation). */
export function roomIds(tree: SceneTree | null, floor: string | null, room: string | null): ObjectId[] {
  const out: ObjectId[] = [];
  const walk = (n: TreeNode) => {
    out.push(n.id);
    n.children.forEach(walk);
  };
  [...(tree?.roots ?? []).filter((r) => r.kind === 'ROOM'), ...roomRoots(tree, floor, room)].forEach(walk);
  return out;
}
