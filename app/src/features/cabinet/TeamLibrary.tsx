// Thư viện nhóm (D31): nguồn thư viện "Công ty" (thư mục chung chứa library.json), chỉ đọc / ghi,
// trùng tên giữ bản máy hoặc dùng bản nguồn; đẩy chuẩn xưởng / template / mẫu lên nguồn.
import { useEffect, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands, type LibrarySourcesInfo } from '../../core-api/commands';
import { Icon } from '../../shared/icons';

export function TeamLibrary() {
  const { revision } = useUi();
  const [info, setInfo] = useState<LibrarySourcesInfo | null>(null);
  const [name, setName] = useState('Công ty');
  const [path, setPath] = useState('');
  const [readonly, setReadonly] = useState(false);
  const load = () =>
    Commands.getLibrarySources()
      .then(setInfo)
      .catch(() => setInfo(null));
  useEffect(() => {
    void load();
  }, [revision]);
  if (!info) return null;
  const save = (sources: LibrarySourcesInfo['sources'], conflict = info.conflict) =>
    void Commands.setLibrarySources(sources.map(({ name, path, readonly }) => ({ name, path, readonly })), conflict)
      .then(setInfo)
      .catch(() => undefined);
  return (
    <div className="team-lib">
      {info.sources.map((s, i) => (
        <div className="tpl-item" key={s.name}>
          <span title={s.path}>
            {s.name} {s.readonly ? '(chỉ đọc)' : ''}
          </span>
          <small>{s.items} mục</small>
          <button className="icon-btn danger" title="Bỏ nguồn" onClick={() => save(info.sources.filter((_, k) => k !== i))}>
            <Icon name="x" size={12} />
          </button>
        </div>
      ))}
      {info.errors.map((e) => (
        <p key={e} className="muted small warn">
          Không đọc được: {e}
        </p>
      ))}
      <label className="struct-row">
        <span>Tên nguồn</span>
        <input className="field" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.stopPropagation()} />
      </label>
      <label className="struct-row">
        <span>Thư mục chung</span>
        <input className="field" placeholder="\\\\may-chu\\thu-vien hoặc /mnt/chung" value={path} onChange={(e) => setPath(e.target.value)} onKeyDown={(e) => e.stopPropagation()} />
      </label>
      <label className="struct-row">
        <span>Chỉ đọc (thợ)</span>
        <input type="checkbox" checked={readonly} onChange={(e) => setReadonly(e.target.checked)} />
      </label>
      <button className="btn" disabled={!name.trim() || !path.trim()} onClick={() => save([...info.sources.filter((s) => s.name !== name.trim()), { name: name.trim(), path: path.trim(), readonly, items: 0 }])}>
        Thêm nguồn
      </button>
      <label className="struct-row">
        <span>Trùng tên</span>
        <select className="field" value={info.conflict} onChange={(e) => save(info.sources, e.target.value as LibrarySourcesInfo['conflict'])}>
          <option value="KEEP_LOCAL">Giữ bản máy</option>
          <option value="USE_REMOTE">Dùng bản nguồn</option>
        </select>
      </label>
      <button className="btn" onClick={() => void Commands.reloadLibrary().then(setInfo).catch(() => undefined)}>
        Nạp lại từ nguồn
      </button>
      <p className="muted small">Đẩy chuẩn xưởng lên nguồn: chuột phải tủ → bảng kết cấu → Chuẩn xưởng → chọn → "Đẩy lên nhóm".</p>
    </div>
  );
}
