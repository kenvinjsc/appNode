//! Bản vẽ in (D29): core chiếu layout các tủ thành primitive vector trên giấy (mm, gốc
//! trên-trái, y xuống): mặt đứng theo từng hướng tường, mặt bằng, chi tiết tủ (trước / bên),
//! chuỗi kích thước, ký hiệu chiều mở cánh, khung tên. UI chỉ vẽ SVG và in.

use crate::Engine;
use aic_domain::layout::PartKind;
use aic_domain::{EdgeSide, ObjectId, PanelRole};
use aic_math::{Obb, Transform3D};
use aic_project::CoreError;
use serde_json::{json, Value};

/// Tỷ lệ chuẩn (1:n), thử từ lớn đến nhỏ.
const SCALES: [f64; 7] = [5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0];
const MARGIN: f64 = 10.0;
const TITLE_H: f64 = 24.0;
/// Chỗ cho kích thước + tiêu đề quanh mỗi hình (mm giấy).
const PAD: [f64; 4] = [22.0, 10.0, 22.0, 12.0]; // trái, phải, dưới, trên

#[derive(Clone, Copy)]
enum Side {
    Below,
    Left,
    Above,
}

#[derive(Clone)]
struct Dim {
    a: f64,
    b: f64,
    /// Toạ độ đường gốc (v với Below / Above, u với Left).
    at: f64,
    side: Side,
    level: u32,
}

#[derive(Clone, Default)]
struct View {
    title: String,
    /// Hình chữ nhật (u0, v0, u1, v1, lớp) — model mm, v hướng lên.
    rects: Vec<(f64, f64, f64, f64, &'static str)>,
    lines: Vec<(f64, f64, f64, f64, &'static str)>,
    /// Chữ tại (u, v), cỡ mm giấy.
    texts: Vec<(f64, f64, String, f64)>,
    dims: Vec<Dim>,
    min: [f64; 2],
    max: [f64; 2],
}

impl View {
    fn new(title: String) -> Self {
        Self { title, min: [f64::MAX; 2], max: [f64::MIN; 2], ..Default::default() }
    }
    fn grow(&mut self, u: f64, v: f64) {
        self.min = [self.min[0].min(u), self.min[1].min(v)];
        self.max = [self.max[0].max(u), self.max[1].max(v)];
    }
    fn rect(&mut self, u0: f64, v0: f64, u1: f64, v1: f64, cls: &'static str) {
        self.grow(u0, v0);
        self.grow(u1, v1);
        self.rects.push((u0, v0, u1, v1, cls));
    }
    fn size(&self) -> [f64; 2] {
        [(self.max[0] - self.min[0]).max(1.0), (self.max[1] - self.min[1]).max(1.0)]
    }
    fn empty(&self) -> bool {
        self.rects.is_empty()
    }
}

/// Paper transform of one placed view.
struct Place {
    ox: f64,
    oy: f64,
    s: f64,
    umin: f64,
    vmax: f64,
}

impl Place {
    fn p(&self, u: f64, v: f64) -> (f64, f64) {
        (r(self.ox + (u - self.umin) / self.s), r(self.oy + (self.vmax - v) / self.s))
    }
}

fn r(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn fmt(v: f64) -> String {
    let v = (v * 10.0).round() / 10.0;
    if (v - v.round()).abs() < 1e-9 { format!("{}", v.round() as i64) } else { format!("{v:.1}").replace('.', ",") }
}

impl Engine {
    /// `get_drawing_sheet`: trang A3 / A4 có khung tên cho một phòng (hoặc các tủ `ids`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn drawing_sheet(&self, ids: &[ObjectId], room: Option<&str>, floor: Option<&str>, paper: &str, portrait: bool, views: &[String], hide_fronts: bool, drawer: &str, date: &str) -> Result<Value, CoreError> {
        let mut cabs: Vec<(ObjectId, aic_domain::Cabinet)> = self
            .doc
            .objects
            .iter()
            .filter_map(|(id, o)| o.as_cabinet().map(|c| (*id, c.clone())))
            .filter(|(id, c)| {
                if !ids.is_empty() {
                    return ids.contains(id);
                }
                room.is_none_or(|r| c.room == r) && floor.is_none_or(|f| c.floor == f)
            })
            .filter(|(id, _)| self.doc.scene.is_effectively_visible(*id))
            .collect();
        if cabs.is_empty() {
            return Err(crate::zones::bad("scope", "no cabinet"));
        }
        cabs.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        let want = |k: &str| views.is_empty() && k != "DETAIL" || views.iter().any(|v| v.eq_ignore_ascii_case(k));
        let mut list: Vec<View> = Vec::new();
        if want("ELEVATION") {
            list.extend(self.elevations(&cabs));
        }
        if want("PLAN") {
            list.push(self.plan(&cabs));
        }
        if want("DETAIL") {
            for (id, c) in &cabs {
                list.extend(self.details(*id, c, hide_fronts));
            }
        }
        list.retain(|v| !v.empty());
        let (pw, ph) = match (paper.to_ascii_uppercase().as_str(), portrait) {
            ("A4", false) => (297.0, 210.0),
            ("A4", true) => (210.0, 297.0),
            (_, false) => (420.0, 297.0),
            (_, true) => (297.0, 420.0),
        };
        let (aw, ah) = (pw - 2.0 * MARGIN, ph - 2.0 * MARGIN - TITLE_H);
        let fits = |v: &View, s: f64| {
            let [w, h] = v.size();
            w / s + PAD[0] + PAD[1] <= aw && h / s + PAD[2] + PAD[3] <= ah
        };
        // Xếp kệ: hàng ngang, hết chỗ thì trang mới.
        let pack = |scale: f64| -> Vec<Vec<(usize, f64, f64)>> {
            let mut sheets: Vec<Vec<(usize, f64, f64)>> = vec![Vec::new()];
            let (mut x, mut y, mut row_h) = (0.0, 0.0, 0.0_f64);
            for (i, v) in list.iter().enumerate() {
                let [w, h] = v.size();
                let (bw, bh) = (w / scale + PAD[0] + PAD[1], h / scale + PAD[2] + PAD[3]);
                if x > 0.0 && x + bw > aw {
                    x = 0.0;
                    y += row_h;
                    row_h = 0.0;
                }
                if y > 0.0 && y + bh > ah {
                    sheets.push(Vec::new());
                    x = 0.0;
                    y = 0.0;
                    row_h = 0.0;
                }
                sheets.last_mut().unwrap().push((i, MARGIN + x, MARGIN + y));
                x += bw;
                row_h = row_h.max(bh);
            }
            sheets
        };
        // Tỷ lệ chung: lớn nhất xếp được tất cả vào một trang; không được thì lớn nhất mà
        // từng hình vừa một trang (nhiều trang).
        let single = SCALES.iter().copied().find(|s| list.iter().all(|v| fits(v, *s)) && pack(*s).len() == 1);
        let scale = single.or_else(|| SCALES.iter().copied().find(|s| list.iter().all(|v| fits(v, *s)))).unwrap_or(*SCALES.last().unwrap());
        let placed = pack(scale);
        let sheets: Vec<Vec<(View, f64, f64)>> = placed.into_iter().map(|sh| sh.into_iter().map(|(i, x, y)| (list[i].clone(), x, y)).collect()).collect();
        let n = sheets.len();
        let project = self.doc.meta.name.clone();
        let room_label = match (room, floor) {
            (Some(r), Some(f)) if !f.is_empty() => format!("{r} · {f}"),
            (Some(r), _) => r.to_string(),
            _ => cabs.iter().map(|c| c.1.name.clone()).collect::<Vec<_>>().join(", "),
        };
        let out: Vec<Value> = sheets
            .into_iter()
            .enumerate()
            .map(|(i, views)| {
                let mut items = Vec::new();
                let mut boxes = Vec::new();
                for (v, x, y) in views {
                    let [w, h] = v.size();
                    let pl = Place { ox: x + PAD[0], oy: y + PAD[3], s: scale, umin: v.min[0], vmax: v.max[1] };
                    render(&v, &pl, &mut items);
                    boxes.push(json!({ "title": v.title, "x": r(x), "y": r(y), "w": r(w / scale + PAD[0] + PAD[1]), "h": r(h / scale + PAD[2] + PAD[3]) }));
                }
                title_block(&mut items, pw, ph, &project, &room_label, drawer, date, scale, i + 1, n);
                json!({ "paper": [pw, ph], "scale": format!("1:{}", scale as i64), "views": boxes, "items": items })
            })
            .collect();
        Ok(json!({ "sheets": out, "scale": format!("1:{}", scale as i64) }))
    }

    /// Part rectangles of a cabinet in its own frame: (min, max) AABB, role, hinge, is hardware.
    fn part_boxes(&self, cab: ObjectId) -> Vec<([f64; 3], [f64; 3], Option<PanelRole>, Option<EdgeSide>, &'static str)> {
        let Some(l) = self.doc.cabinet_layout(cab) else { return Vec::new() };
        l.parts
            .iter()
            .map(|p| {
                let b = Obb::new(p.size, Transform3D::new(p.translation, p.rotation_deg)).aabb();
                let (role, hinge, cls) = match &p.kind {
                    PartKind::Panel { role, hinge, .. } => (Some(*role), *hinge, if matches!(role, PanelRole::Door | PanelRole::DrawerFront) { "front" } else { "part" }),
                    PartKind::Hardware { .. } => (None, None, "hw"),
                };
                (b.min, b.max, role, hinge, cls)
            })
            .collect()
    }

    /// Mặt đứng: một hình cho mỗi hướng tường (tủ cùng góc xoay), nhìn từ trước.
    fn elevations(&self, cabs: &[(ObjectId, aic_domain::Cabinet)]) -> Vec<View> {
        let mut groups: Vec<(i64, Vec<ObjectId>)> = Vec::new();
        for (id, _) in cabs {
            let yaw = (self.doc.scene.world(*id).rotation_deg[1].rem_euclid(360.0) / 90.0).round() as i64 % 4;
            match groups.iter_mut().find(|g| g.0 == yaw) {
                Some(g) => g.1.push(*id),
                None => groups.push((yaw, vec![*id])),
            }
        }
        groups
            .into_iter()
            .map(|(yaw, ids)| {
                let title = if yaw == 0 { "Mặt đứng".to_string() } else { format!("Mặt đứng (tường xoay {}°)", yaw * 90) };
                let mut v = View::new(title);
                let first = self.doc.scene.world(ids[0]);
                let o0 = first.transform_point([0.0, 0.0, 0.0]);
                let x1 = first.transform_point([1.0, 0.0, 0.0]);
                let axis = [x1[0] - o0[0], x1[1] - o0[1], x1[2] - o0[2]];
                let mut spans = Vec::new();
                let mut all: Vec<(f64, [f64; 4], &'static str, Option<EdgeSide>)> = Vec::new();
                for id in &ids {
                    let w = self.doc.scene.world(*id);
                    let o = w.transform_point([0.0, 0.0, 0.0]);
                    let u0 = o[0] * axis[0] + o[1] * axis[1] + o[2] * axis[2];
                    let (cw, ch) = (self.doc.param_value(*id, "width").unwrap_or(0.0), self.doc.param_value(*id, "height").unwrap_or(0.0));
                    spans.push((u0, u0 + cw, o[1], o[1] + ch, self.doc.objects.get(id).map(|o| o.name().to_string()).unwrap_or_default()));
                    for (mn, mx, _, hinge, cls) in self.part_boxes(*id) {
                        all.push((o[2] + mx[2], [u0 + mn[0], o[1] + mn[1], u0 + mx[0], o[1] + mx[1]], cls, hinge));
                    }
                }
                // Vẽ từ sau ra trước: cánh che phần trong.
                all.sort_by(|a, b| a.0.total_cmp(&b.0));
                for (_, [a, b, c, d], cls, hinge) in all {
                    v.rect(a, b, c, d, cls);
                    if let Some(h) = hinge {
                        opening(&mut v, [a, b, c, d], h);
                    }
                }
                chain(&mut v, &spans);
                v
            })
            .collect()
    }

    /// Mặt bằng: hình chiếu đứng xuống (x, −z) của từng tủ.
    fn plan(&self, cabs: &[(ObjectId, aic_domain::Cabinet)]) -> View {
        let mut v = View::new("Mặt bằng".into());
        let mut xs = Vec::new();
        for (id, c) in cabs {
            let b = self.doc.world_aabb(*id);
            if b.is_empty() {
                continue;
            }
            v.rect(b.min[0], -b.max[2], b.max[0], -b.min[2], "cab");
            v.texts.push(((b.min[0] + b.max[0]) / 2.0, (-b.max[2] - b.min[2]) / 2.0, c.name.clone(), 2.2));
            xs.push((b.min[0], b.max[0]));
        }
        if let (Some(a), Some(b)) = (xs.iter().map(|x| x.0).reduce(f64::min), xs.iter().map(|x| x.1).reduce(f64::max)) {
            let top = v.max[1];
            v.dims.push(Dim { a, b, at: top, side: Side::Above, level: 0 });
        }
        v
    }

    /// Chi tiết một tủ: mặt trước (ẩn cánh nếu cần) + mặt bên, kích thước W / H / D.
    fn details(&self, id: ObjectId, c: &aic_domain::Cabinet, hide_fronts: bool) -> Vec<View> {
        let p = |n: &str| self.doc.param_value(id, n).unwrap_or(0.0);
        let (w, h, d) = (p("width"), p("height"), p("depth"));
        let boxes = self.part_boxes(id);
        let mut front = View::new(format!("{} · mặt trước{}", c.name, if hide_fronts { " (ẩn cánh)" } else { "" }));
        let mut side = View::new(format!("{} · mặt bên", c.name));
        let mut sorted = boxes.clone();
        sorted.sort_by(|a, b| a.1[2].total_cmp(&b.1[2]));
        for (mn, mx, role, hinge, cls) in sorted {
            if hide_fronts && (cls == "front" || cls == "hw") {
                continue;
            }
            front.rect(mn[0], mn[1], mx[0], mx[1], cls);
            if let (Some(hg), false) = (hinge, hide_fronts) {
                opening(&mut front, [mn[0], mn[1], mx[0], mx[1]], hg);
            }
            let _ = role;
        }
        let mut by_x = boxes;
        by_x.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
        for (mn, mx, _, _, cls) in by_x.into_iter().rev() {
            // Nhìn từ trái: u = z (trước ở bên trái), v = y.
            side.rect(-mx[2], mn[1], -mn[2], mx[1], cls);
        }
        front.dims.push(Dim { a: 0.0, b: w, at: front.min[1], side: Side::Below, level: 0 });
        front.dims.push(Dim { a: 0.0, b: h, at: front.min[0], side: Side::Left, level: 0 });
        side.dims.push(Dim { a: -d, b: 0.0, at: side.min[1], side: Side::Below, level: 0 });
        vec![front, side]
    }
}

/// Ký hiệu chiều mở: 2 nét đứt từ 2 góc phía bản lề tới giữa cạnh đối diện.
fn opening(v: &mut View, [a, b, c, d]: [f64; 4], hinge: EdgeSide) {
    let (mx, my) = ((a + c) / 2.0, (b + d) / 2.0);
    let segs = match hinge {
        EdgeSide::Left => [(c, b, a, my), (c, d, a, my)],
        EdgeSide::Right => [(a, b, c, my), (a, d, c, my)],
        EdgeSide::Top => [(a, b, mx, d), (c, b, mx, d)],
        EdgeSide::Bottom => [(a, d, mx, b), (c, d, mx, b)],
    };
    for (x1, y1, x2, y2) in segs {
        v.lines.push((x1, y1, x2, y2, "open"));
    }
}

/// Chuỗi kích thước mặt đứng: rộng từng tủ + tổng (dưới), cao từng loại tủ (trái), tên tủ (trên).
fn chain(v: &mut View, spans: &[(f64, f64, f64, f64, String)]) {
    let bottom = v.min[1];
    // Tủ sàn (đáy thấp nhất) ghi dưới; tủ treo ghi trên.
    let floor_y = spans.iter().map(|s| s.2).fold(f64::MAX, f64::min);
    let (low, high): (Vec<_>, Vec<_>) = spans.iter().partition(|s| s.2 <= floor_y + 1.0);
    for s in &low {
        v.dims.push(Dim { a: s.0, b: s.1, at: bottom, side: Side::Below, level: 0 });
    }
    if low.len() > 1 {
        let (a, b) = (low.iter().map(|s| s.0).fold(f64::MAX, f64::min), low.iter().map(|s| s.1).fold(f64::MIN, f64::max));
        v.dims.push(Dim { a, b, at: bottom, side: Side::Below, level: 1 });
    }
    let top = v.max[1];
    for s in &high {
        v.dims.push(Dim { a: s.0, b: s.1, at: top, side: Side::Above, level: 0 });
    }
    let left = v.min[0];
    let mut hs: Vec<(f64, f64)> = Vec::new();
    for s in spans {
        if !hs.iter().any(|h| (h.0 - s.2).abs() < 1.0 && (h.1 - s.3).abs() < 1.0) {
            hs.push((s.2, s.3));
        }
    }
    for (i, (a, b)) in hs.into_iter().enumerate() {
        v.dims.push(Dim { a, b, at: left, side: Side::Left, level: i as u32 });
    }
    for s in spans {
        v.texts.push(((s.0 + s.1) / 2.0, (s.2 + s.3) / 2.0, s.4.clone(), 2.0));
    }
}

fn render(v: &View, pl: &Place, items: &mut Vec<Value>) {
    let [w, _] = v.size();
    let (tx, ty) = pl.p(v.min[0] + 0.0, v.max[1]);
    items.push(json!({ "t": "text", "x": r(tx), "y": r(ty - PAD[3] + 5.0), "s": format!("{} · 1:{}", v.title, pl.s as i64), "size": 3.2, "anchor": "start", "cls": "title" }));
    let _ = w;
    for (a, b, c, d, cls) in &v.rects {
        let (x0, y0) = pl.p(*a, *d);
        let (x1, y1) = pl.p(*c, *b);
        items.push(json!({ "t": "rect", "x": x0, "y": y0, "w": r(x1 - x0), "h": r(y1 - y0), "cls": cls }));
    }
    for (a, b, c, d, cls) in &v.lines {
        let (x1, y1) = pl.p(*a, *b);
        let (x2, y2) = pl.p(*c, *d);
        items.push(json!({ "t": "line", "x1": x1, "y1": y1, "x2": x2, "y2": y2, "cls": cls }));
    }
    for (u, vv, s, size) in &v.texts {
        let (x, y) = pl.p(*u, *vv);
        items.push(json!({ "t": "text", "x": x, "y": y, "s": s, "size": size, "anchor": "middle", "cls": "label" }));
    }
    for d in &v.dims {
        let gap = 6.0 + 6.0 * d.level as f64;
        let txt = fmt((d.b - d.a).abs());
        match d.side {
            Side::Below | Side::Above => {
                let (xa, yb) = pl.p(d.a, d.at);
                let (xb, _) = pl.p(d.b, d.at);
                let y = if matches!(d.side, Side::Below) { yb + gap } else { yb - gap };
                items.push(json!({ "t": "line", "x1": xa, "y1": r(yb), "x2": xa, "y2": r(y), "cls": "ext" }));
                items.push(json!({ "t": "line", "x1": xb, "y1": r(yb), "x2": xb, "y2": r(y), "cls": "ext" }));
                items.push(json!({ "t": "line", "x1": xa, "y1": r(y), "x2": xb, "y2": r(y), "cls": "dim" }));
                items.push(json!({ "t": "text", "x": r((xa + xb) / 2.0), "y": r(y - 1.0), "s": txt, "size": 2.2, "anchor": "middle", "cls": "dimtext", "value": r(d.b - d.a) }));
            }
            Side::Left => {
                let (xl, ya) = pl.p(d.at, d.a);
                let (_, yb) = pl.p(d.at, d.b);
                let x = xl - gap;
                items.push(json!({ "t": "line", "x1": r(xl), "y1": ya, "x2": r(x), "y2": ya, "cls": "ext" }));
                items.push(json!({ "t": "line", "x1": r(xl), "y1": yb, "x2": r(x), "y2": yb, "cls": "ext" }));
                items.push(json!({ "t": "line", "x1": r(x), "y1": ya, "x2": r(x), "y2": yb, "cls": "dim" }));
                items.push(json!({ "t": "text", "x": r(x - 1.0), "y": r((ya + yb) / 2.0), "s": txt, "size": 2.2, "anchor": "middle", "rotate": -90, "cls": "dimtext", "value": r(d.b - d.a) }));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn title_block(items: &mut Vec<Value>, pw: f64, ph: f64, project: &str, room: &str, drawer: &str, date: &str, scale: f64, page: usize, pages: usize) {
    items.push(json!({ "t": "rect", "x": MARGIN / 2.0, "y": MARGIN / 2.0, "w": pw - MARGIN, "h": ph - MARGIN, "cls": "frame" }));
    let (x0, y0, w) = (pw - MARGIN - 180.0, ph - MARGIN - TITLE_H + 4.0, 180.0);
    items.push(json!({ "t": "rect", "x": x0, "y": y0, "w": w, "h": TITLE_H - 4.0, "cls": "frame" }));
    let cells = [("Dự án", project.to_string()), ("Phòng / tủ", room.to_string()), ("Người vẽ", drawer.to_string()), ("Ngày", date.to_string()), ("Tỷ lệ", format!("1:{}", scale as i64)), ("Trang", format!("{page}/{pages}"))];
    for (i, (k, v)) in cells.iter().enumerate() {
        let (cx, cy) = (x0 + (i % 3) as f64 * 60.0, y0 + (i / 3) as f64 * 10.0);
        items.push(json!({ "t": "rect", "x": cx, "y": cy, "w": 60.0, "h": 10.0, "cls": "cell" }));
        items.push(json!({ "t": "text", "x": cx + 2.0, "y": cy + 3.5, "s": k, "size": 2.0, "anchor": "start", "cls": "key" }));
        items.push(json!({ "t": "text", "x": cx + 2.0, "y": cy + 8.2, "s": v, "size": 3.0, "anchor": "start", "cls": "val" }));
    }
}
