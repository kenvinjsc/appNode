// Mẫu dựng sẵn (D30): lưới mẫu theo phòng; chọn mẫu → sửa W / H / D + tham số → Chèn.
// Core dựng mẫu (insert_product) trong một bước undo; UI chỉ gửi kích thước và tham số.
import { useEffect, useState } from 'react';
import { findNode, useUi } from '../../app/uiStore';
import { Commands, type ProductInfo } from '../../core-api/commands';
import { Icon } from '../../shared/icons';

export function TemplateGallery() {
  const { gallery, set } = useUi();
  const [list, setList] = useState<ProductInfo[]>([]);
  const [room, setRoom] = useState('');
  const [key, setKey] = useState('');
  const [size, setSize] = useState<[number, number, number]>([0, 0, 0]);
  const [params, setParams] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!gallery) return;
    Commands.getProducts()
      .then((r) => {
        setList(r.products);
        setRoom((cur) => cur || r.products[0]?.room || '');
      })
      .catch(() => setList([]));
  }, [gallery]);
  const cur = list.find((p) => p.key === key);
  useEffect(() => {
    if (!cur) return;
    setSize(cur.size);
    setParams(Object.fromEntries(cur.params.map((p) => [p.key, p.default])));
  }, [cur]);
  if (!gallery) return null;
  const rooms = [...new Set(list.map((p) => p.room))];
  const close = () => set({ gallery: false });
  const insert = () => {
    if (!cur) return;
    const s = useUi.getState();
    const after = s.selection.find((id) => findNode(s.tree, id)?.node.kind === 'CABINET');
    void Commands.insertProduct(cur.key, after, { width: size[0], height: size[1], depth: size[2] }, params)
      .then((r) => {
        s.select([r.id]);
        close();
      })
      .catch(() => undefined);
  };
  const dim = (i: 0 | 1 | 2, label: string) => (
    <label className="struct-row">
      <span>{label}</span>
      <input
        className="field"
        type="number"
        value={size[i]}
        onChange={(e) => {
          const v = [...size] as [number, number, number];
          v[i] = Number(e.target.value);
          setSize(v);
        }}
        onKeyDown={(e) => e.stopPropagation()}
      />
    </label>
  );
  return (
    <div className="modal-back" onPointerDown={close}>
      <div className="modal gallery" onPointerDown={(e) => e.stopPropagation()}>
        <div className="struct-head">
          <Icon name="sample" size={15} />
          <b>Mẫu dựng sẵn</b>
          <div className="seg">
            {rooms.map((r) => (
              <button key={r} className={r === room ? 'active' : ''} onClick={() => setRoom(r)}>
                {r}
              </button>
            ))}
          </div>
          <div className="spacer" />
          <button className="icon-btn" title="Đóng" onClick={close}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="gallery-body">
          <div className="gallery-grid">
            {list
              .filter((p) => p.room === room)
              .map((p) => (
                <button key={p.key} className={`gallery-card ${p.key === key ? 'on' : ''}`} onClick={() => setKey(p.key)} onDoubleClick={insert}>
                  <b>{p.name}</b>
                  <span>{p.summary}</span>
                </button>
              ))}
          </div>
          <div className="gallery-form">
            {cur ? (
              <>
                <b>{cur.name}</b>
                {dim(0, 'Rộng')}
                {dim(1, 'Cao')}
                {dim(2, 'Sâu')}
                {cur.params.map((p) => (
                  <label key={p.key} className="struct-row">
                    <span>{p.label}</span>
                    {p.kind === 'bool' ? (
                      <input type="checkbox" checked={params[p.key] === 'on'} onChange={(e) => setParams({ ...params, [p.key]: e.target.checked ? 'on' : 'off' })} />
                    ) : p.kind === 'select' ? (
                      <select className="field" value={params[p.key]} onChange={(e) => setParams({ ...params, [p.key]: e.target.value })}>
                        {p.options.map((o) => (
                          <option key={o.value} value={o.value}>
                            {o.label}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <input className="field" value={params[p.key] ?? ''} onChange={(e) => setParams({ ...params, [p.key]: e.target.value })} onKeyDown={(e) => e.stopPropagation()} />
                    )}
                  </label>
                ))}
                <button className="btn primary" onClick={insert}>
                  Chèn mẫu
                </button>
                <div className="muted small">Chèn cạnh tủ đang chọn (cùng phòng); một lần Hoàn tác bỏ cả mẫu.</div>
              </>
            ) : (
              <div className="muted small">Chọn một mẫu bên trái.</div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
