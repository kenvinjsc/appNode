// Vietnamese UI vocabulary. Core returns stable keys; labels live here.

export const ROLE_LABEL: Record<string, string> = {
  Generic: 'Tấm',
  LeftSide: 'Hồi trái',
  RightSide: 'Hồi phải',
  Top: 'Nóc / giằng',
  Bottom: 'Đáy',
  Back: 'Hậu',
  Shelf: 'Kệ di động',
  ShelfFixed: 'Kệ cố định',
  BackSub: 'Hậu phụ',
  DrawerSide: 'Thành ngăn kéo',
  DrawerBack: 'Hậu / đầu hộc',
  DrawerBottom: 'Đáy ngăn kéo',
  Rail: 'Thanh chặn / thanh treo',
  Slide: 'Ray bi',
  Divider: 'Hông giữa',
  Door: 'Cánh',
  DrawerFront: 'Mặt ngăn kéo',
  Plinth: 'Chân tủ',
  Handle: 'Tay nắm',
  Hinge: 'Bản lề',
  Leg: 'Chân',
  Base: 'Tủ bếp dưới',
  Wall: 'Tủ bếp trên',
  Wardrobe: 'Tủ áo',
  OpenShelf: 'Kệ mở',
  Drawer: 'Tủ ngăn kéo',
};

export const KIND_LABEL: Record<string, string> = {
  ROOM: 'Phòng',
  CABINET: 'Tủ',
  PANEL: 'Tấm',
  HARDWARE: 'Phụ kiện',
};

export const CABINET_KINDS: { kind: import('../core-api/types').CabinetKind; label: string; icon: string }[] = [
  { kind: 'WARDROBE', label: 'Tủ áo', icon: 'cabinet' },
  { kind: 'BASE', label: 'Tủ bếp dưới', icon: 'cabinet' },
  { kind: 'WALL', label: 'Tủ bếp trên', icon: 'door' },
  { kind: 'DRAWER', label: 'Tủ ngăn kéo', icon: 'drawer' },
  { kind: 'OPEN_SHELF', label: 'Kệ mở', icon: 'shelf' },
  { kind: 'DOOR', label: 'Tủ 1 cánh', icon: 'door' },
];

export const FIELD_LABEL: Record<string, string> = {
  name: 'Tên',
  role: 'Vai trò',
  kind: 'Loại',
  catalog: 'Mã',
  width: 'Rộng',
  height: 'Cao',
  depth: 'Sâu',
  thickness: 'Dày',
  length: 'Dài',
  size: 'Kích thước',
  x: 'X',
  y: 'Y',
  z: 'Z',
  rx: 'Xoay X',
  ry: 'Xoay Y',
  rz: 'Xoay Z',
  material: 'Ván',
  grain: 'Vân gỗ',
  carcass_material: 'Thùng',
  front_material: 'Cánh / mặt',
  back_material: 'Hậu',
  edge_left: 'Cạnh trái',
  edge_right: 'Cạnh phải',
  edge_top: 'Cạnh trên',
  edge_bottom: 'Cạnh dưới',
  drills: 'Lỗ khoan',
  grooves: 'Rãnh',
  pockets: 'Hốc',
  volume: 'Thể tích',
  back_thickness: 'Dày hậu',
  plinth_height: 'Cao chân',
  door_gap: 'Khe cánh',
  top_style: 'Kiểu nóc',
  anchor_w: 'Neo rộng',
  off_front: 'Lùi mặt trước',
  off_back: 'Lùi mặt sau',
  off_left: 'Lùi trái',
  off_right: 'Lùi phải',
  off_top: 'Lùi trên',
  off_bottom: 'Lùi dưới',
  anchor_h: 'Neo cao',
  anchor_d: 'Neo sâu',
  bottom_style: 'Kiểu đáy',
  back_panel: 'Tấm hậu',
  shelves: 'Số đợt',
  doors: 'Số cánh',
  drawers: 'Số ngăn kéo',
  inner_width: 'Lọt lòng rộng',
  inner_height: 'Lọt lòng cao',
  inner_depth: 'Lọt lòng sâu',
  full_name: 'Tên tấm',
  room: 'Tên phòng',
  floor: 'Tầng',
  handles: 'Tay nắm',
  back_groove: 'Rãnh hậu',
  back_offset: 'Hậu cách sau',
  rail_width: 'Rộng giằng',
  door_thickness: 'Dày cánh',
  shelf_setback: 'Kệ lùi trước',
  links: 'Liên kết',
  tools: 'Tool đã áp',
};

export const GROUP_LABEL: Record<string, string> = {
  general: 'Chung',
  size: 'Kích thước',
  position: 'Vị trí',
  rotation: 'Góc xoay',
  material: 'Vật liệu',
  edges: 'Dán cạnh',
  manufacturing: 'Gia công',
  construction: 'Kết cấu',
  content: 'Thành phần',
  derived: 'Giá trị tính toán',
  zone_position: 'Vị trí',
  door: 'Cánh',
  drawer: 'Cấu hình chi tiết',
  link: 'Liên kết',
  stretch: 'Co giãn tấm',
  offset: 'Offset (lùi mặt)',
  constraints: 'Ràng buộc (bám mặt tấm khác)',
  relations: 'Liên kết & tool',
  edge_rule: 'Luật dán cạnh',
};

export const OPTION_LABEL: Record<string, string> = {
  INSET: 'Lọt lòng',
  OVERLAY: 'Phủ',
  RAILS: 'Thanh giằng',
  KEEP_LEFT: 'Giữ trái',
  KEEP_RIGHT: 'Giữ phải',
  KEEP_CENTER: 'Giữ giữa',
  KEEP_BOTTOM: 'Giữ dưới',
  KEEP_TOP: 'Giữ trên',
  KEEP_BACK: 'Giữ sau',
  KEEP_FRONT: 'Giữ trước',
};

export const GRAIN_LABEL: Record<string, string> = { ALONG_HEIGHT: 'Theo chiều cao', ALONG_WIDTH: 'Theo chiều rộng', NONE: 'Không vân' };

export const PURPOSE_LABEL: Record<string, string> = {
  GENERIC: 'Khoan',
  SHELF_PIN: 'Chốt đợt',
  DOWEL: 'Chốt gỗ',
  CAM_LOCK: 'Cam',
  CONNECTOR: 'Liên kết',
  HINGE_CUP: 'Khoét bản lề',
  HINGE_SCREW: 'Vít bản lề',
  HANDLE: 'Tay nắm',
};

export const EDGE_LABEL: Record<string, string> = { LEFT: 'Trái', RIGHT: 'Phải', TOP: 'Trên', BOTTOM: 'Dưới' };

export const CONTACT_LABEL: Record<string, string> = { TOUCH: 'Tiếp xúc', GAP: 'Khe hở', PENETRATE: 'Xuyên' };
export const ORIENT_LABEL: Record<string, string> = { PARALLEL: 'Song song', PERPENDICULAR: 'Vuông góc', OBLIQUE: 'Xiên' };
export const REGION_LABEL: Record<string, string> = { FACE: 'Mặt', EDGE: 'Cạnh', END: 'Đầu' };

/** Role label from either `LeftSide` or `LEFT_SIDE` spelling. */
export function roleLabel(role: string): string {
  const pascal = role.includes('_') || role === role.toUpperCase() ? role.toLowerCase().replace(/(^|_)([a-z])/g, (_, __, c: string) => c.toUpperCase()) : role;
  return ROLE_LABEL[pascal] ?? role;
}

export function objectLabel(kind: string, role: string | null): string {
  if (role && ROLE_LABEL[role]) return ROLE_LABEL[role];
  return KIND_LABEL[kind] ?? kind;
}

export function fmt(n: number | null | undefined, digits = 1): string {
  if (n === null || n === undefined || Number.isNaN(n)) return '–';
  const r = Math.round(n * 10 ** digits) / 10 ** digits;
  return r.toLocaleString('vi-VN', { maximumFractionDigits: digits, useGrouping: false });
}
