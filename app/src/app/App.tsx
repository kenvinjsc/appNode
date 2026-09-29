import { useEffect } from 'react';
import { setErrorNotifier } from '../core-api/commands';
import { onHistoryStatus } from '../core-api/events';
import { Queries } from '../core-api/queries';
import { CncWorkspace } from '../features/cnc/CncWorkspace';
import { Drawing2D } from '../features/drawing2d/Drawing2D';
import { ManufacturingWorkspace } from '../features/manufacturing/ManufacturingWorkspace';
import { MaterialBrowser } from '../features/materials/MaterialBrowser';
import { NestingWorkspace } from '../features/nesting/NestingWorkspace';
import { PropertiesPanel } from '../features/properties/PropertiesPanel';
import { PartsReport } from '../features/report/PartsReport';
import { SceneTree } from '../features/scene-tree/SceneTree';
import { Icon } from '../shared/icons';
import { Viewport } from '../viewport/Viewport';
import { Actions } from './actions';
import { ContextMenu, Jobs, StatusBar, TitleBar, Toasts, WorkspaceRail } from './Chrome';
import { Ribbon } from './Ribbon';
import { ShortcutSheet, useShortcutLayer } from './Shortcuts';
import { useUi } from './uiStore';

let booted = false;

export function App() {
  const { workspace, panels, drawer, set } = useUi();
  useShortcutLayer();

  useEffect(() => {
    setErrorNotifier((m) => useUi.getState().toast({ kind: 'error', title: m.title, detail: m.detail }));
    const off = onHistoryStatus((s) => useUi.getState().set({ canUndo: s.canUndo, canRedo: s.canRedo, revision: s.revision }));
    Queries.materials().then((materials) => set({ materials })).catch(() =>
      useUi.getState().toast({ kind: 'error', title: 'Không kết nối được lõi CAD.', detail: 'Hãy chạy aic-dev-server (npm run core) hoặc mở ứng dụng desktop.' }),
    );
    // First start: open the sample project so the workspace is not empty.
    if (booted) return off;
    booted = true;
    Queries.sceneTree()
      .then((t) => {
        if (t.roots.length === 0) void Actions.sampleProject();
      })
      .catch(() => undefined);
    return off;
  }, [set]);

  return (
    <div className="app">
      <TitleBar />
      <Ribbon />
      <div className="main">
        <WorkspaceRail />
        {/* The design viewport stays mounted so GPU caches survive workspace switches. */}
        <div className="ws ws-design" style={{ display: workspace === 'design' ? 'flex' : 'none' }}>
          {panels.tree && <SceneTree />}
          <div className="center">
            <div className="panel view3d">
              <div className="panel-header">
                <Icon name="box3d" size={16} />
                <span>3D · Phối cảnh</span>
              </div>
              <Viewport />
            </div>
            {panels.drawing && <Drawing2D onClose={() => set({ panels: { ...panels, drawing: false } })} />}
          </div>
          {panels.properties && <PropertiesPanel />}
        </div>
        {workspace === 'manufacturing' && <ManufacturingWorkspace />}
        {workspace === 'nesting' && <NestingWorkspace />}
        {workspace === 'cnc' && <CncWorkspace />}
        {drawer && (
          <div className="drawer">
            {drawer === 'materials' && <MaterialBrowser />}
            {drawer === 'report' && <PartsReport />}
            {drawer === 'shortcuts' && <ShortcutSheet />}
          </div>
        )}
      </div>
      <StatusBar />
      <ContextMenu />
      <Toasts />
      <Jobs />
    </div>
  );
}
