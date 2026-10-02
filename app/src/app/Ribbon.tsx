// Contextual ribbon: tabs group tools so they are never all shown at once.
import { useEffect, useState } from 'react';
import { CABINET_KINDS } from '../shared/i18n';
import { Icon } from '../shared/icons';
import { View } from '../viewport/viewportBus';
import { Actions } from './actions';
import { openSplitTool } from '../features/cabinet/SplitDialog';
import { findNode, useUi, type RibbonTab } from './uiStore';
import { Commands } from '../core-api/commands';

function Btn({ icon, label, onClick, active, disabled, title, accent }: { icon: string; label: string; onClick?: () => void; active?: boolean; disabled?: boolean; title?: string; accent?: boolean }) {
  return (
    <button className={`rb-btn ${active ? 'active' : ''} ${accent ? 'accent' : ''}`} onClick={onClick} disabled={disabled} title={title ?? label}>
      <Icon name={icon} size={20} />
      <span>{label}</span>
    </button>
  );
}

function Group({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="rb-group">
      <div className="rb-items">{children}</div>
      <div className="rb-label">{label}</div>
    </div>
  );
}

function CabinetMenu() {
  const [open, setOpen] = useState(false);
  const { tool, setTool } = useUi();
  const [products, setProducts] = useState<{ key: string; name: string; summary: string }[]>([]);
  useEffect(() => {
    if (open && !products.length)
      Commands.getProducts()
        .then((r) => setProducts(r.products.filter((p) => p.room === 'Bếp')))
        .catch(() => undefined);
  }, [open, products.length]);
  const after = () => {
    const s = useUi.getState();
    return s.selection.find((id) => findNode(s.tree, id)?.node.kind === 'CABINET');
  };
  return (
    <div className="rb-drop" onMouseLeave={() => setOpen(false)}>
      <Btn icon="cabinet" label="Tủ ▾" accent active={tool.type === 'place-cabinet'} onClick={() => setOpen(!open)} title="Tạo tủ: chọn loại rồi nhấp vào vị trí" />
      {open && (
        <div className="menu">
          {CABINET_KINDS.map((k) => (
            <button
              key={k.kind}
              onClick={() => {
                setTool({ type: 'place-cabinet', kind: k.kind });
                setOpen(false);
              }}
            >
              <Icon name={k.icon} size={16} /> {k.label}
            </button>
          ))}
          {(['LEFT', 'RIGHT'] as const).map((hand) => (
            <button
              key={hand}
              onClick={() => {
                setOpen(false);
                const s = useUi.getState();
                const after = s.selection.find((id) => findNode(s.tree, id)?.node.kind === 'CABINET');
                void Commands.createCorner(hand, after)
                  .then((r) => s.select([r.id]))
                  .catch(() => undefined);
              }}
            >
              <Icon name="cabinet" size={16} /> Tủ góc L mù ({hand === 'LEFT' ? 'góc trái' : 'góc phải'})
            </button>
          ))}
          {[false, true].map((wall) => (
            <button
              key={`dg${wall}`}
              onClick={() => {
                setOpen(false);
                const s = useUi.getState();
                const after = s.selection.find((id) => findNode(s.tree, id)?.node.kind === 'CABINET');
                void Commands.createDiagonalCorner(wall, after)
                  .then((r) => s.select([r.id]))
                  .catch(() => undefined);
              }}
            >
              <Icon name="cabinet" size={16} /> Tủ góc chéo ({wall ? 'bếp trên' : 'bếp dưới'})
            </button>
          ))}
          <button
            onClick={() => {
              setOpen(false);
              useUi.getState().set({ gallery: true });
            }}
          >
            <Icon name="sample" size={16} /> Mẫu dựng sẵn (tủ áo, giường, kệ TV…)
          </button>
          <div className="menu-sep">Sản phẩm khác</div>
          {FURNITURE.map((f) => (
            <button
              key={f.label}
              onClick={() => {
                setOpen(false);
                const s = useUi.getState();
                const sel = s.selection.map((id) => findNode(s.tree, id)?.node).find((n) => n?.kind === 'CABINET');
                void Commands.createFurniture(f.kind, f.size, f.options, sel?.room ?? undefined)
                  .then((r) => s.select([r.id]))
                  .catch(() => undefined);
              }}
            >
              <Icon name="cabinet" size={16} /> {f.label}
            </button>
          ))}
          {products.length > 0 && <div className="menu-sep">Mẫu bếp dựng sẵn</div>}
          {products.map((p) => (
            <button
              key={p.key}
              title={p.summary}
              onClick={() => {
                setOpen(false);
                void Commands.insertProduct(p.key, after())
                  .then((r) => useUi.getState().select([r.id]))
                  .catch(() => undefined);
              }}
            >
              <Icon name="sample" size={16} /> {p.name}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** Sản phẩm ngoài tủ hộp (core dựng theo `create_furniture`). */
const FURNITURE: { label: string; kind: string; size: { width?: number; height?: number; depth?: number }; options?: Record<string, string> }[] = [
  { label: 'Giường 1600 × 2000', kind: 'BED', size: { width: 1600, depth: 2000, height: 1000 } },
  { label: 'Giường hộc kéo 1600', kind: 'BED', size: { width: 1600, depth: 2000, height: 1000 }, options: { bed_frame_h: '450', bed_storage: 'DRAWERS_2_SIDES' } },
  { label: 'Bàn học 1200 (hộc phải, kệ trên)', kind: 'DESK', size: { width: 1200, height: 750, depth: 600 }, options: { desk_hutch_h: '600', desk_hutch_shelves: '2' } },
  { label: 'Bàn làm việc 1400', kind: 'DESK', size: { width: 1400, height: 750, depth: 700 }, options: { desk_support_left: 'LEG', desk_support_right: 'LEG', desk_keyboard_tray: 'on' } },
  { label: 'Bàn trang điểm 1000', kind: 'DESK', size: { width: 1000, height: 750, depth: 450 }, options: { desk_support_left: 'PANEL', desk_unit_drawers: '2', desk_mirror_w: '600', desk_cable_d: '0', desk_modesty: 'off' } },
  { label: 'Bàn đảo 1800 (mở 2 mặt)', kind: 'ISLAND', size: { width: 1800, height: 900, depth: 900 } },
  { label: 'Vách TV 3000 × 2700', kind: 'CLADDING', size: { width: 3000, height: 2700 }, options: { cl_cols: '/5' } },
  { label: 'Vách lam 1200 × 2700', kind: 'CLADDING', size: { width: 1200, height: 2700 }, options: { cl_boards: 'off', cl_frame: 'on', cl_batten: 'on' } },
];

export function Ribbon() {
  const { ribbonTab, set, tool, setTool, canUndo, canRedo, selection, selectionMode, showRelations, showDimensions, snap, projection, section, panels, tree, drawer } = useUi();
  const tabs: [RibbonTab, string][] = [
    ['file', 'Tệp'],
    ['home', 'Trang đầu'],
    ['design', 'Thiết kế'],
    ['edit', 'Chỉnh sửa'],
    ['material', 'Vật liệu'],
    ['manufacturing', 'Sản xuất (CNC)'],
    ['view', 'Xem'],
  ];
  const has = selection.length > 0;
  const project = (
    <Group label="Dự án">
      <Btn icon="new" label="Mới" onClick={() => void Actions.newProject()} />
      <Btn icon="open" label="Mở" onClick={() => Actions.open()} title="Mở (Ctrl+O)" />
      <Btn icon="save" label="Lưu" onClick={() => void Actions.save()} title="Lưu (Ctrl+S)" />
    </Group>
  );
  const editTools = (
    <Group label="Chỉnh sửa">
      <Btn icon="select" label="Chọn" active={tool.type === 'select'} onClick={() => setTool({ type: 'select' })} title="Chọn (Q)" />
      <Btn icon="move" label="Di chuyển" active={tool.type === 'move'} onClick={() => setTool({ type: 'move' })} title="Di chuyển (M)" />
      <Btn icon="rotate" label="Xoay" active={tool.type === 'rotate'} onClick={() => setTool({ type: 'rotate' })} title="Xoay (R)" />
      <Btn icon="fit" label="Phóng vừa" onClick={() => View.fit(selection, tree)} title="Phóng vừa (F)" />
    </Group>
  );
  const components = (
    <Group label="Thành phần">
      <CabinetMenu />
      <Btn icon="shelf" label="Đợt" onClick={() => void Actions.bumpCabinet('shelves', 1)} title="Thêm đợt vào tủ đang chọn" />
      <Btn icon="door" label="Cánh" onClick={() => void Actions.bumpCabinet('doors', 1)} title="Thêm cánh" />
      <Btn icon="drawer" label="Ngăn kéo" onClick={() => void Actions.bumpCabinet('drawers', 1)} title="Thêm ngăn kéo" />
      <Btn icon="divider" label="Vách ngăn" onClick={() => void Actions.addDivider()} />
      <Btn icon="grid" label="Chia khoang" active={useUi.getState().splitTool !== null} onClick={() => openSplitTool()} title="Chia khoang theo công thức: bấm vào khoang trong 3D / 2D (K)" />
    </Group>
  );
  const toolsGroup = (
    <Group label="Công cụ">
      <Btn icon="duplicate" label="Sao chép" disabled={!has} onClick={() => void Actions.duplicate()} title="Sao chép (Ctrl+D)" />
      <Btn icon="trash" label="Xóa" disabled={!has} onClick={() => void Actions.delete()} title="Xóa (Delete)" />
      <Btn icon="relations" label="Liên kết" active={showRelations} onClick={() => set({ showRelations: !showRelations })} title="Quan hệ lắp ghép (Shift+R)" />
    </Group>
  );
  const production = (
    <Group label="Sản xuất">
      <Btn icon="drill" label="Gia công" onClick={() => set({ workspace: 'manufacturing' })} />
      <Btn icon="nesting" label="Xếp tấm" onClick={() => set({ workspace: 'nesting' })} />
      <Btn icon="cnc" label="Xuất CNC" accent onClick={() => set({ workspace: 'cnc' })} />
      <Btn icon="report" label="Báo cáo" active={drawer === 'report'} onClick={() => set({ drawer: drawer === 'report' ? null : 'report' })} />
    </Group>
  );
  return (
    <div className="ribbon">
      <div className="rb-tabs">
        {tabs.map(([k, l]) => (
          <button key={k} className={ribbonTab === k ? 'active' : ''} onClick={() => set({ ribbonTab: k })}>
            {l}
          </button>
        ))}
      </div>
      <div className="rb-body">
        {ribbonTab === 'file' && (
          <>
            {project}
            <Group label="Mẫu">
              <Btn icon="sample" label="Dự án mẫu" onClick={() => void Actions.sampleProject()} />
              <Btn icon="download" label="Ảnh chụp" onClick={() => {
                const url = View.screenshot();
                if (!url) return;
                const a = document.createElement('a');
                a.href = url;
                a.download = 'aic-viewport.png';
                a.click();
              }} />
            </Group>
            <Group label="Lịch sử">
              <Btn icon="undo" label="Hoàn tác" disabled={!canUndo} onClick={() => void Actions.undo()} />
              <Btn icon="redo" label="Làm lại" disabled={!canRedo} onClick={() => void Actions.redo()} />
            </Group>
          </>
        )}
        {ribbonTab === 'home' && (
          <>
            {project}
            {editTools}
            {components}
            {toolsGroup}
            {production}
          </>
        )}
        {ribbonTab === 'design' && (
          <>
            <Group label="Tạo">
              <Btn icon="room" label="Phòng" onClick={() => void Actions.addRoom()} />
              <CabinetMenu />
              <Btn icon="panel" label="Tấm rời" onClick={() => void Actions.addFreePanel()} />
            </Group>
            {components}
            <Group label="Phụ kiện">
              <Btn icon="hardware" label="Phụ kiện" onClick={() => set({ drawer: 'materials' })} title="Tay nắm, bản lề và thanh treo được sinh tự động theo cánh/tủ" />
            </Group>
          </>
        )}
        {ribbonTab === 'edit' && (
          <>
            {editTools}
            <Group label="Chế độ chọn">
              <Btn icon="object" label="Đối tượng" active={selectionMode === 'object'} onClick={() => set({ selectionMode: 'object' })} />
              <Btn icon="face" label="Mặt" active={selectionMode === 'face'} onClick={() => set({ selectionMode: 'face' })} />
              <Btn icon="edge" label="Cạnh" active={selectionMode === 'edge'} onClick={() => set({ selectionMode: 'edge' })} />
              <Btn icon="boxSelect" label="Quét chọn" title="Giữ Shift và kéo trong khung nhìn" onClick={() => useUi.getState().toast({ kind: 'info', title: 'Quét chọn', detail: 'Giữ Shift và kéo chuột trong khung nhìn 3D.' })} />
            </Group>
            {toolsGroup}
            <Group label="Căn chỉnh">
              <Btn icon="align" label="Trái" disabled={selection.length < 2} title="Căn mép trái" onClick={() => void Commands.alignObjects(selection, 'LEFT').catch(() => undefined)} />
              <Btn icon="align" label="Phải" disabled={selection.length < 2} title="Căn mép phải" onClick={() => void Commands.alignObjects(selection, 'RIGHT').catch(() => undefined)} />
              <Btn icon="align" label="Trên" disabled={selection.length < 2} title="Căn đỉnh (tủ trên)" onClick={() => void Commands.alignObjects(selection, 'TOP').catch(() => undefined)} />
              <Btn icon="align" label="Dưới" disabled={selection.length < 2} title="Căn đáy" onClick={() => void Commands.alignObjects(selection, 'BOTTOM').catch(() => undefined)} />
              <Btn icon="align" label="Căn trước" disabled={selection.length < 2} title="Căn mặt trước" onClick={() => void Commands.alignObjects(selection, 'FRONT').catch(() => undefined)} />
              <Btn icon="dimension" label="Chia đều" disabled={selection.length < 3} title="Phân bố đều theo chiều ngang" onClick={() => void Commands.distributeObjects(selection, 'X').catch(() => undefined)} />
              <Btn icon="rotate" label="Xoay 90°" disabled={!has} title="Xoay 90° quanh tâm" onClick={() => void Commands.rotateObjects(selection, 90).catch(() => undefined)} />
              <Btn icon="rotate" label="−90°" disabled={!has} title="Xoay −90° quanh tâm" onClick={() => void Commands.rotateObjects(selection, -90).catch(() => undefined)} />
              <Btn icon="room" label="Sát tường" disabled={!has} title="Đặt sát tường gần nhất" onClick={() => void Commands.snapToWall(selection).catch(() => undefined)} />
            </Group>
            <Group label="Trạng thái">
              <Btn icon="eyeOff" label="Ẩn/hiện" disabled={!has} onClick={() => void Actions.toggleHidden()} />
              <Btn icon="lock" label="Khóa" disabled={!has} onClick={() => void Actions.toggleLocked()} />
              <Btn icon="magnet" label="Bắt dính" active={snap} onClick={() => set({ snap: !snap })} />
            </Group>
          </>
        )}
        {ribbonTab === 'material' && (
          <Group label="Vật liệu">
            <Btn icon="material" label="Thư viện" active={drawer === 'materials'} onClick={() => set({ drawer: drawer === 'materials' ? null : 'materials' })} />
            <Btn icon="edgeband" label="Dán cạnh" disabled={!has} onClick={() => set({ propertiesTab: 'material' })} />
          </Group>
        )}
        {ribbonTab === 'manufacturing' && (
          <>
            <Group label="Gia công">
              <Btn icon="drill" label="Khoan" onClick={() => set({ workspace: 'manufacturing' })} />
              <Btn icon="pocket" label="Hốc" onClick={() => set({ workspace: 'manufacturing' })} />
              <Btn icon="groove" label="Rãnh" onClick={() => set({ workspace: 'manufacturing' })} />
              <Btn icon="edgeband" label="Dán cạnh" onClick={() => set({ propertiesTab: 'material' })} />
              <Btn icon="inspect" label="Kiểm tra" active={showRelations} onClick={() => set({ showRelations: !showRelations })} />
            </Group>
            {production}
          </>
        )}
        {ribbonTab === 'view' && (
          <>
            <Group label="Hướng nhìn">
              <Btn icon="front" label="Trước" onClick={() => View.set('front', selection, tree)} title="1" />
              <Btn icon="front" label="Sau" onClick={() => View.set('back', selection, tree)} title="2" />
              <Btn icon="side" label="Trái" onClick={() => View.set('left', selection, tree)} title="3" />
              <Btn icon="side" label="Phải" onClick={() => View.set('right', selection, tree)} title="4" />
              <Btn icon="top" label="Trên" onClick={() => View.set('top', selection, tree)} title="5" />
              <Btn icon="view" label="Phối cảnh" onClick={() => View.set('iso', selection, tree)} title="0" />
            </Group>
            <Group label="Hiển thị">
              <Btn icon="perspective" label={projection === 'perspective' ? 'Phối cảnh' : 'Trực giao'} onClick={() => set({ projection: projection === 'perspective' ? 'orthographic' : 'perspective' })} />
              <Btn icon="fit" label="Vừa tất cả" onClick={() => View.fitAll()} />
              <Btn icon="section" label="Mặt cắt" active={section} onClick={() => set({ section: !section })} />
              <Btn icon="dimension" label="Kích thước" active={showDimensions} onClick={() => set({ showDimensions: !showDimensions })} />
              <Btn icon="relations" label="Quan hệ" active={showRelations} onClick={() => set({ showRelations: !showRelations })} />
            </Group>
            <Group label="Bảng">
              <Btn icon="layers" label="Cây" active={panels.tree} onClick={() => set({ panels: { ...panels, tree: !panels.tree } })} />
              <Btn icon="drawing" label="Bản vẽ 2D" active={panels.drawing} onClick={() => set({ panels: { ...panels, drawing: !panels.drawing } })} />
              <Btn icon="settings" label="Thuộc tính" active={panels.properties} onClick={() => set({ panels: { ...panels, properties: !panels.properties } })} />
              <Btn icon="keyboard" label="Phím tắt" onClick={() => set({ drawer: 'shortcuts' })} />
            </Group>
          </>
        )}
      </div>
    </div>
  );
}
