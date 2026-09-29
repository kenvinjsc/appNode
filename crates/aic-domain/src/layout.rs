//! Cabinet builder: resolves a cabinet definition (carcass options + zone tree)
//! against current parameter values into concrete parts in the cabinet frame.
//! Pure function: no ids, no scene, no geometry kernel.
//!
//! Cabinet frame: X = width, Y = height, Z = depth (front at Z = depth).
//! Panel local frame: X = width, Y = height, Z = thickness.

use crate::cabinet::{MaterialSlot, ROT_HORIZONTAL, ROT_SIDE};
use crate::zone::*;
use crate::{
    Axis2, Cabinet, CabinetKind, DrillFeature, DrillPurpose, EdgeSide, FaceSide, GrainDirection, GrooveFeature, HardwareKind,
    JoinStyle, MachiningFeature, PanelRole,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Numeric cabinet parameters (read from the parameter graph).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CabinetValues {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub thickness: f64,
    pub back_thickness: f64,
    pub plinth_height: f64,
    pub door_thickness: f64,
    pub door_gap: f64,
    pub shelf_setback: f64,
    /// Rãnh hậu: how far the back runs into grooves (0 = no groove, back inset).
    pub back_groove: f64,
    /// Distance of the back from the rear edge (groove mode).
    pub back_offset: f64,
    /// Width of top stretcher rails (giằng) when `top_style` = Rails.
    pub rail_width: f64,
}

/// Per-part user modifications that survive regeneration (keyed by part key).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PartMod {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub deleted: bool,
    /// Co giãn: extension (mm) of the left, right, bottom, top edges (local frame).
    #[serde(default)]
    pub extend: [f64; 4],
    #[serde(default)]
    pub thickness: Option<f64>,
    #[serde(default)]
    pub material: Option<String>,
    /// Tools applied (for "Tool đã áp").
    #[serde(default)]
    pub tools: Vec<String>,
    /// Extra machining added by tools, local panel frame.
    #[serde(default)]
    pub features: Vec<MachiningFeature>,
    /// Manual edge-band overrides (on/off per edge) on top of the cabinet rule.
    #[serde(default)]
    pub edges: BTreeMap<EdgeSide, bool>,
    /// Chia tấm: the part becomes `count` pieces along an axis with a gap between them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<PartSplit>,
}

/// Chia tấm (P11). Pieces get keys `{key}`, `{key}~2`, `{key}~3`, …
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PartSplit {
    pub axis: crate::Axis2,
    pub count: u32,
    #[serde(default)]
    pub gap: f64,
}

impl PartMod {
    pub fn is_default(&self) -> bool {
        *self == PartMod::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PartKind {
    Panel {
        role: PanelRole,
        slot: MaterialSlot,
        grain: GrainDirection,
        features: Vec<MachiningFeature>,
        hinge: Option<EdgeSide>,
    },
    Hardware {
        kind: HardwareKind,
        catalog: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// Stable key inside the cabinet (maps to a persistent object id).
    pub key: String,
    pub name: String,
    pub kind: PartKind,
    pub size: [f64; 3],
    pub translation: [f64; 3],
    pub rotation_deg: [f64; 3],
}

/// Resolved zone box (cabinet frame) for UI pinning and labels.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ZoneBox {
    pub id: Uid,
    pub min: [f64; 3],
    pub size: [f64; 3],
    pub leaf: bool,
    pub depth: u32,
    pub has_front: bool,
}

/// Resolved position of a split panel (all three representations).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PanelPosition {
    pub uid: Uid,
    pub zone: Uid,
    pub axis: usize,
    pub lock: Lock,
    pub ratio: f64,
    pub from_start: f64,
    pub from_end: f64,
    /// Clear sizes of the two neighbouring cells.
    pub cell_before: f64,
    pub cell_after: f64,
    pub zone_length: f64,
}

/// Countable fittings (phụ kiện) of a cabinet.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Fittings {
    pub hinges: u32,
    pub shelf_pins: u32,
    /// (slide length mm → sets)
    pub slides: BTreeMap<u32, u32>,
    /// (rail length rounded mm → pieces)
    pub oval_rails: BTreeMap<u32, u32>,
    pub oval_cups: u32,
    pub handles: u32,
    pub sliding_tracks: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub parts: Vec<Part>,
    pub zones: Vec<ZoneBox>,
    pub positions: Vec<PanelPosition>,
    /// Every bay (khoang) of every split, for editable dimensions.
    pub bays: Vec<BayInfo>,
    pub fittings: Fittings,
    /// Zones whose bays cannot be solved (too small / conflicting locks).
    pub problems: Vec<Uid>,
}

/// One bay of a split, resolved (cabinet frame).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BayInfo {
    /// Zone that owns the split.
    pub zone: Uid,
    pub index: usize,
    /// The child zone this bay is.
    pub child: Uid,
    pub axis: usize,
    /// Start along the axis (cabinet frame) and clear size.
    pub start: f64,
    pub size: f64,
    /// None = legacy per-panel positioning (converted on first edit).
    pub mode: Option<BayMode>,
    pub value: f64,
    /// Usable length of the split (zone length minus split panels).
    pub usable: f64,
}

#[derive(Debug, Clone, Copy)]
struct Neighbor {
    t: f64,
    outer: bool,
    /// Index of the neighbouring part in `Layout::parts` (for shelf-pin drilling).
    part: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct ZBox {
    min: [f64; 3],
    size: [f64; 3],
    /// left, right, bottom, top
    nb: [Neighbor; 4],
}

struct Ctx<'a> {
    cab: &'a Cabinet,
    v: CabinetValues,
    out: Layout,
    counters: BTreeMap<&'static str, u32>,
    drawer_sets: u32,
}

const STD_SLIDES: [f64; 8] = [250.0, 300.0, 350.0, 400.0, 450.0, 500.0, 550.0, 600.0];

fn pos(v: f64) -> f64 {
    v.max(1.0)
}

impl<'a> Ctx<'a> {
    fn next(&mut self, base: &'static str) -> u32 {
        let c = self.counters.entry(base).or_insert(0);
        *c += 1;
        *c
    }

    #[allow(clippy::too_many_arguments)]
    fn panel(
        &mut self,
        key: String,
        name: String,
        role: PanelRole,
        slot: MaterialSlot,
        grain: GrainDirection,
        size: [f64; 3],
        translation: [f64; 3],
        rotation_deg: [f64; 3],
    ) -> usize {
        self.out.parts.push(Part {
            key,
            name,
            kind: PartKind::Panel { role, slot, grain, features: Vec::new(), hinge: None },
            size: size.map(pos),
            translation,
            rotation_deg,
        });
        self.out.parts.len() - 1
    }

    fn hardware(&mut self, key: String, name: String, kind: HardwareKind, catalog: &str, size: [f64; 3], translation: [f64; 3]) {
        self.out.parts.push(Part {
            key,
            name,
            kind: PartKind::Hardware { kind, catalog: catalog.into() },
            size: size.map(pos),
            translation,
            rotation_deg: [0.0; 3],
        });
    }

    fn add_features(&mut self, idx: usize, f: Vec<MachiningFeature>) {
        if let PartKind::Panel { features, .. } = &mut self.out.parts[idx].kind {
            features.extend(f);
        }
    }
}

/// Build all parts of a cabinet.
pub fn build(cab: &Cabinet, v: CabinetValues) -> Layout {
    let mut cx = Ctx { cab, v, out: Layout::default(), counters: BTreeMap::new(), drawer_sets: 0 };
    let root = carcass(&mut cx);
    zone(&mut cx, &cab.zones.root, root, 0);
    apply_mods(&mut cx.out, &cab.mods);
    cx.out
}

/// Carcass parts; returns the interior root zone box.
fn carcass(cx: &mut Ctx) -> ZBox {
    let v = cx.v;
    let (w, h, d, t, bt, p) = (v.width, v.height, v.depth, v.thickness, v.back_thickness, v.plinth_height);
    let cab = cx.cab;
    let top_overlay = cab.top_style == JoinStyle::Overlay;
    let rails = cab.top_style == JoinStyle::Rails;
    let bottom_overlay = cab.bottom_style == JoinStyle::Overlay;
    let groove = cab.back_panel && v.back_groove > 0.0;
    // Z where the carcass horizontals start (behind them: the back).
    let back_front = if !cab.back_panel {
        0.0
    } else if groove {
        v.back_offset + bt
    } else {
        bt
    };
    let horiz_start = if groove || !cab.back_panel { 0.0 } else { bt };
    let side_y = if bottom_overlay { p + t } else { 0.0 };
    let side_h = h - side_y - if top_overlay { t } else { 0.0 };

    let l = cx.panel("c:left".into(), "HồiTrái".into(), PanelRole::LeftSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, side_h, t], [0.0, side_y, d], ROT_SIDE);
    let r = cx.panel("c:right".into(), "HồiPhải".into(), PanelRole::RightSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, side_h, t], [w - t, side_y, d], ROT_SIDE);

    let inner_w = w - 2.0 * t;
    let top_w = if top_overlay { w } else { inner_w };
    let top_x = if top_overlay { 0.0 } else { t };
    let bottom_w = if bottom_overlay { w } else { inner_w };
    let bottom_x = if bottom_overlay { 0.0 } else { t };
    let hdepth = d - horiz_start;
    let mut top_idx = None;
    if rails {
        let rw = v.rail_width.min(hdepth / 2.0);
        cx.panel("c:rail_front".into(), "GiằngTrước".into(), PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [inner_w, rw, t], [t, h - t, d], ROT_HORIZONTAL);
        cx.panel("c:rail_back".into(), "GiằngSau".into(), PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [inner_w, rw, t], [t, h - t, horiz_start + rw], ROT_HORIZONTAL);
    } else {
        top_idx = Some(cx.panel("c:top".into(), "Nóc".into(), PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [top_w, hdepth, t], [top_x, h - t, d], ROT_HORIZONTAL));
    }
    let b = cx.panel("c:bottom".into(), "Đáy".into(), PanelRole::Bottom, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bottom_w, hdepth, t], [bottom_x, p, d], ROT_HORIZONTAL);

    if cab.back_panel {
        if groove {
            let g = v.back_groove;
            let bw = inner_w + 2.0 * g;
            let bh = h - p - 2.0 * t + 2.0 * g;
            cx.panel("c:back".into(), "Hậu".into(), PanelRole::Back, MaterialSlot::Back, GrainDirection::AlongHeight, [bw, bh, bt], [t - g, p + t - g, v.back_offset], [0.0; 3]);
            let tol = 0.5;
            let gw = bt + tol;
            // Groove across the inner faces: sides (along height), top/bottom (along width).
            // Side local X runs from the front (x = 0 ↔ z = d) to the back.
            let gx = d - v.back_offset - bt - tol / 2.0;
            let sy0 = p + t - g - side_y;
            let side_len = bh;
            let depth = g.min(t - 4.0).max(1.0);
            cx.add_features(l, vec![MachiningFeature::Groove(GrooveFeature { x: gx, y: sy0, length: side_len, width: gw, depth, direction: Axis2::Y, side: FaceSide::A })]);
            cx.add_features(r, vec![MachiningFeature::Groove(GrooveFeature { x: gx, y: sy0, length: side_len, width: gw, depth, direction: Axis2::Y, side: FaceSide::B })]);
            let hy = d - v.back_offset - bt - tol / 2.0;
            let gx_top = (t - g) - top_x;
            if let Some(ti) = top_idx {
                cx.add_features(ti, vec![MachiningFeature::Groove(GrooveFeature { x: gx_top, y: hy, length: bw, width: gw, depth, direction: Axis2::X, side: FaceSide::B })]);
            }
            let gx_bot = (t - g) - bottom_x;
            cx.add_features(b, vec![MachiningFeature::Groove(GrooveFeature { x: gx_bot, y: hy, length: bw, width: gw, depth, direction: Axis2::X, side: FaceSide::A })]);
        } else {
            cx.panel("c:back".into(), "Hậu".into(), PanelRole::Back, MaterialSlot::Back, GrainDirection::AlongHeight, [inner_w, h - t - p, bt], [t, p, 0.0], [0.0; 3]);
        }
    }

    if matches!(cab.kind, CabinetKind::Wardrobe | CabinetKind::Drawer | CabinetKind::Base) && p > 1.0 {
        cx.panel("c:plinth".into(), "ChânTủ".into(), PanelRole::Plinth, MaterialSlot::Carcass, GrainDirection::AlongWidth, [inner_w, p, t], [t, 0.0, d - 50.0 - t], [0.0; 3]);
    }

    let outer = |part| Neighbor { t, outer: true, part };
    ZBox {
        min: [t, p + t, back_front],
        size: [inner_w, h - p - 2.0 * t, d - back_front],
        nb: [outer(Some(l)), outer(Some(r)), outer(Some(b)), outer(top_idx)],
    }
}

/// Positions (start along the axis, relative to the zone) of a split's panels.
fn solve_split(len: f64, panels: &[SplitPanel]) -> Vec<f64> {
    let n = panels.len();
    let mut start: Vec<Option<f64>> = panels
        .iter()
        .map(|p| match p.lock {
            Lock::Even => None,
            Lock::Ratio => Some(p.value.clamp(0.0, 1.0) * (len - p.thickness)),
            Lock::FromStart => Some(p.value),
            Lock::FromEnd => Some(len - p.thickness - p.value),
        })
        .collect();
    // Distribute `Even` panels between fixed neighbours.
    let mut i = 0;
    while i < n {
        if start[i].is_some() {
            i += 1;
            continue;
        }
        let j0 = i;
        while i < n && start[i].is_none() {
            i += 1;
        }
        let lo = if j0 == 0 { 0.0 } else { start[j0 - 1].unwrap() + panels[j0 - 1].thickness };
        let hi = if i == n { len } else { start[i].unwrap() };
        let k = (i - j0) as f64;
        let tsum: f64 = panels[j0..i].iter().map(|p| p.thickness).sum();
        let gap = ((hi - lo - tsum) / (k + 1.0)).max(0.0);
        let mut at = lo + gap;
        for (j, s) in start.iter_mut().enumerate().take(i).skip(j0) {
            *s = Some(at);
            at += panels[j].thickness + gap;
        }
    }
    start.into_iter().map(|s| s.unwrap().clamp(0.0, (len - 1.0).max(0.0))).collect()
}

fn zone(cx: &mut Ctx, z: &Zone, b: ZBox, level: u32) {
    let t_default = cx.v.thickness;
    let _ = t_default;
    cx.out.zones.push(ZoneBox { id: z.id, min: b.min, size: b.size, leaf: z.split.is_none(), depth: level, has_front: z.front.is_some() });

    if let Some(s) = &z.split {
        let a = s.axis;
        let len = b.size[a];
        let starts = if s.has_bays() {
            let t: Vec<f64> = s.panels.iter().map(|p| p.thickness).collect();
            let (sizes, ok) = solve_bays(len, &t, &s.bays);
            if !ok {
                cx.out.problems.push(z.id);
            }
            let mut at = 0.0;
            sizes
                .iter()
                .zip(t.iter())
                .map(|(sz, th)| {
                    let st = at + sz.max(0.0);
                    at = st + th;
                    st
                })
                .collect()
        } else {
            solve_split(len, &s.panels)
        };
        let mut part_idx = Vec::with_capacity(s.panels.len());
        let mut cursor = 0.0;
        for (i, (p, st)) in s.panels.iter().zip(starts.iter()).enumerate() {
            let before = st - cursor;
            let after_end = if i + 1 < s.panels.len() { starts[i + 1] } else { len };
            let after = after_end - (st + p.thickness);
            cx.out.positions.push(PanelPosition {
                uid: p.uid,
                zone: z.id,
                axis: a,
                lock: p.lock,
                ratio: if len - p.thickness > 0.0 { st / (len - p.thickness) } else { 0.0 },
                from_start: *st,
                from_end: len - p.thickness - st,
                cell_before: before,
                cell_after: after,
                zone_length: len,
            });
            part_idx.push(split_panel(cx, p, &b, *st));
            cursor = st + p.thickness;
        }
        // Bays (for editable dimensions).
        {
            let usable = len - s.panels.iter().map(|p| p.thickness).sum::<f64>();
            let mut cur = 0.0;
            for (i, c) in s.children.iter().enumerate() {
                let end = if i < s.panels.len() { starts[i] } else { len };
                let bay = s.bays.get(i).filter(|_| s.has_bays());
                cx.out.bays.push(BayInfo {
                    zone: z.id,
                    index: i,
                    child: c.id,
                    axis: a,
                    start: b.min[a] + cur,
                    size: end - cur,
                    mode: bay.map(|x| x.mode),
                    value: bay.map(|x| x.value).unwrap_or(0.0),
                    usable,
                });
                if i < s.panels.len() {
                    cur = starts[i] + s.panels[i].thickness;
                }
            }
        }
        // Child zones.
        let mut cursor = 0.0;
        for (i, c) in s.children.iter().enumerate() {
            let end = if i < s.panels.len() { starts[i] } else { len };
            let mut cb = b;
            cb.min[a] = b.min[a] + cursor;
            cb.size[a] = (end - cursor).max(1.0);
            if a < 2 {
                let (lo, hi) = if a == 0 { (0, 1) } else { (2, 3) };
                if i > 0 {
                    cb.nb[lo] = Neighbor { t: s.panels[i - 1].thickness, outer: false, part: Some(part_idx[i - 1]) };
                }
                if i < s.panels.len() {
                    cb.nb[hi] = Neighbor { t: s.panels[i].thickness, outer: false, part: Some(part_idx[i]) };
                }
            }
            zone(cx, c, cb, level + 1);
            if i < s.panels.len() {
                cursor = starts[i] + s.panels[i].thickness;
            }
        }
    }
    for l in &z.links {
        link(cx, l, &b);
    }
    if let Some(f) = &z.front {
        match f {
            Front::Doors(d) => doors(cx, d, &b),
            Front::Drawers(d) => drawers(cx, d, &b),
        }
    }
}

fn split_panel(cx: &mut Ctx, p: &SplitPanel, b: &ZBox, st: f64) -> usize {
    let [x, y, z] = b.min;
    let [w, h, d] = b.size;
    let t = p.thickness;
    let n = cx.next(p.kind.base_name());
    let name = format!("{}_{:02}", p.kind.base_name(), n);
    let key = format!("p:{}", p.uid);
    let front = z + d;
    match p.kind {
        SplitKind::ShelfAdjustable => {
            let sb = cx.v.shelf_setback.min(d / 2.0);
            let idx = cx.panel(key, name, PanelRole::Shelf, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w - 1.0, d - sb, t], [x + 0.5, y + st, front - sb], ROT_HORIZONTAL);
            cx.out.fittings.shelf_pins += 4;
            // Shelf pins (chốt tầng) in the side / divider on each side of the shelf.
            let pin_y = y + st - 5.0;
            let zs = [front - sb - 37.0, z + 37.0];
            for (nb, face) in [(b.nb[0], FaceSide::A), (b.nb[1], FaceSide::B)] {
                let Some(pi) = nb.part else { continue };
                let part = &cx.out.parts[pi];
                if part.rotation_deg != ROT_SIDE {
                    continue;
                }
                let (tz, ty) = (part.translation[2], part.translation[1]);
                let feats = zs
                    .iter()
                    .map(|wz| MachiningFeature::Drill(DrillFeature { x: tz - wz, y: pin_y - ty, diameter: 5.0, depth: 10.0, side: face, purpose: DrillPurpose::ShelfPin }))
                    .collect();
                cx.add_features(pi, feats);
            }
            idx
        }
        SplitKind::ShelfFixed => cx.panel(key, name, PanelRole::ShelfFixed, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, d, t], [x, y + st, front], ROT_HORIZONTAL),
        SplitKind::Divider => cx.panel(key, name, PanelRole::Divider, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, h, t], [x + st, y, front], ROT_SIDE),
        SplitKind::BackSub => cx.panel(key, name, PanelRole::BackSub, MaterialSlot::Back, GrainDirection::AlongHeight, [w, h, t], [x, y, z + st], [0.0; 3]),
    }
}

fn hinge_count(h: f64) -> u32 {
    if h <= 900.0 {
        2
    } else if h <= 1600.0 {
        3
    } else {
        4
    }
}

fn hinge_cups(w: f64, h: f64, side: HingeSide) -> Vec<MachiningFeature> {
    let along = if matches!(side, HingeSide::Left | HingeSide::Right) { h } else { w };
    let n = hinge_count(along);
    let edge = 22.5;
    let inset = 100.0f64.min(along / 4.0);
    (0..n)
        .map(|i| {
            let s = inset + (along - 2.0 * inset) * i as f64 / (n - 1) as f64;
            let (x, y) = match side {
                HingeSide::Left => (edge, s),
                HingeSide::Right => (w - edge, s),
                HingeSide::Top => (s, h - edge),
                HingeSide::Bottom => (s, edge),
            };
            MachiningFeature::Drill(DrillFeature { x, y, diameter: 35.0, depth: 13.0, side: FaceSide::B, purpose: DrillPurpose::HingeCup })
        })
        .collect()
}

/// Front rectangle (x0, y0, x1, y1) and front Z for a zone.
fn front_rect(b: &ZBox, mount: Mount, gap: f64, depth: f64, thickness: f64) -> (f64, f64, f64, f64, f64) {
    let [x, y, _] = b.min;
    let [w, h, _] = b.size;
    match mount {
        Mount::Overlay => {
            let ext = |n: Neighbor| if n.outer { n.t - gap } else { n.t / 2.0 - gap / 2.0 };
            (x - ext(b.nb[0]), y - ext(b.nb[2]), x + w + ext(b.nb[1]), y + h + ext(b.nb[3]), depth)
        }
        Mount::Inset => (x + gap, y + gap, x + w - gap, y + h - gap, depth - thickness),
    }
}

fn to_edge(h: HingeSide) -> EdgeSide {
    match h {
        HingeSide::Left => EdgeSide::Left,
        HingeSide::Right => EdgeSide::Right,
        HingeSide::Top => EdgeSide::Top,
        HingeSide::Bottom => EdgeSide::Bottom,
    }
}

fn doors(cx: &mut Ctx, spec: &DoorSpec, b: &ZBox) {
    let gap = spec.gap.unwrap_or(cx.v.door_gap);
    let t = spec.thickness.unwrap_or(cx.v.door_thickness);
    let d = cx.v.depth;
    let (mut x0, y0, mut x1, mut y1, z) = front_rect(b, spec.mount, gap, d, t);
    // Door stop rail at the top of the opening.
    if spec.stop.kind != StopRail::None {
        let s = &spec.stop;
        let [bx, _, _] = b.min;
        let [bw, _, _] = b.size;
        let top = b.min[1] + b.size[1];
        let ry = top - s.height;
        let rz = d - s.setback;
        let k = cx.next("ThanhChặnCánh");
        cx.panel(format!("s:{}:v", spec.uid), format!("ThanhChặnCánh_{k:02}"), PanelRole::Rail, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bw, s.height, cx.v.thickness], [bx, ry, rz - cx.v.thickness], [0.0; 3]);
        if s.kind == StopRail::LShape {
            let k = cx.next("ThanhChặnCánh");
            cx.panel(format!("s:{}:h", spec.uid), format!("ThanhChặnCánh_{k:02}"), PanelRole::Rail, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bw, s.leg_depth, cx.v.thickness], [bx, top - cx.v.thickness, rz - cx.v.thickness], ROT_HORIZONTAL);
        }
        // Doors end `cover_up` above the rail's bottom edge.
        y1 = ry + s.cover_up;
    }
    let cols = spec.cols.max(1);
    let rows = spec.rows.max(1);
    if spec.kind == DoorKind::Sliding {
        let n = cols.max(2);
        let overlap = 30.0;
        let total = x1 - x0;
        let lw = (total + (n - 1) as f64 * overlap) / n as f64;
        let hgt = y1 - y0;
        for i in 0..n {
            let k = cx.next("CửaLùa");
            let lx = x0 + i as f64 * (lw - overlap);
            let lz = if i % 2 == 0 { z + t + 4.0 } else { z };
            cx.panel(format!("d:{}:0:{i}", spec.uid), format!("CửaLùa_{k:02}"), PanelRole::Door, MaterialSlot::Front, GrainDirection::AlongHeight, [lw, hgt, t], [lx, y0, lz], [0.0; 3]);
        }
        cx.hardware(format!("t:{}:top", spec.uid), "RayLùaTrên".into(), HardwareKind::Rail, "TRACK-SLIDE", [total, 20.0, 2.0 * t + 8.0], [x0, y1, z]);
        cx.hardware(format!("t:{}:bot", spec.uid), "RayLùaDưới".into(), HardwareKind::Rail, "TRACK-SLIDE", [total, 10.0, 2.0 * t + 8.0], [x0, y0 - 10.0, z]);
        cx.out.fittings.sliding_tracks += 1;
        return;
    }
    // Hinged doors: grid of leaves.
    let _ = &mut x0;
    let _ = &mut x1;
    let cw = ((x1 - x0) - (cols - 1) as f64 * gap) / cols as f64;
    let ch = ((y1 - y0) - (rows - 1) as f64 * gap) / rows as f64;
    let base = if spec.kind == DoorKind::Double { "CửaĐôi" } else { "CửaĐơn" };
    for r in 0..rows {
        for c in 0..cols {
            let hinge = match (spec.kind, spec.hinge) {
                (DoorKind::Double, HingeSide::Left | HingeSide::Right) => {
                    if c % 2 == 0 { HingeSide::Left } else { HingeSide::Right }
                }
                (DoorKind::Double, _) => {
                    if r % 2 == 0 { HingeSide::Bottom } else { HingeSide::Top }
                }
                (_, h) => h,
            };
            let k = cx.next(base);
            let dx = x0 + c as f64 * (cw + gap);
            let dy = y0 + r as f64 * (ch + gap);
            let idx = cx.panel(format!("d:{}:{r}:{c}", spec.uid), format!("{base}_{k:02}"), PanelRole::Door, MaterialSlot::Front, GrainDirection::AlongHeight, [cw, ch, t], [dx, dy, z], [0.0; 3]);
            cx.add_features(idx, hinge_cups(cw, ch, hinge));
            if let PartKind::Panel { hinge: hs, .. } = &mut cx.out.parts[idx].kind {
                *hs = Some(to_edge(hinge));
            }
            cx.out.fittings.hinges += hinge_count(if matches!(hinge, HingeSide::Left | HingeSide::Right) { ch } else { cw });
            if cx.cab.handles {
                // Handle on the opening side, vertical bar 160.
                let (hx, hy, size) = match hinge {
                    HingeSide::Left => (dx + cw - 40.0 - 6.0, dy + ch / 2.0 - 80.0, [12.0, 160.0, 30.0]),
                    HingeSide::Right => (dx + 40.0 - 6.0, dy + ch / 2.0 - 80.0, [12.0, 160.0, 30.0]),
                    HingeSide::Bottom => (dx + cw / 2.0 - 80.0, dy + ch - 46.0, [160.0, 12.0, 30.0]),
                    HingeSide::Top => (dx + cw / 2.0 - 80.0, dy + 34.0, [160.0, 12.0, 30.0]),
                };
                let k = cx.next("TayNắm");
                cx.hardware(format!("h:{}:{r}:{c}", spec.uid), format!("TayNắm_{k:02}"), HardwareKind::Handle, "HDL-BAR-160", size, [hx, hy, z + t]);
                cx.out.fittings.handles += 1;
            }
        }
    }
}

fn drawers(cx: &mut Ctx, spec: &DrawerSpec, b: &ZBox) {
    let d = cx.v.depth;
    let ft = spec.face_thickness.unwrap_or(cx.v.door_thickness);
    let gap = spec.gap;
    let side_gap = if spec.mount == Mount::Inset { spec.side_gap } else { cx.v.door_gap };
    let (x0, y0, x1, y1, z) = front_rect(b, spec.mount, side_gap, d, ft);
    let n = spec.count.max(1);
    let cols = spec.cols.max(1);
    let fw = ((x1 - x0) - (cols - 1) as f64 * gap) / cols as f64;
    let fh = ((y1 - y0) - (n - 1) as f64 * gap) / n as f64;
    let [zx, zy, _] = b.min;
    let [zw, zh, zd] = b.size;
    let cw = (zw - (cols - 1) as f64 * cx.v.thickness) / cols as f64;
    let slide = STD_SLIDES.iter().copied().filter(|l| *l <= zd - 10.0).fold(STD_SLIDES[0], f64::max);
    let zf = if spec.mount == Mount::Overlay { d } else { d - ft };
    for c in 0..cols {
        for i in 0..n {
            cx.drawer_sets += 1;
            let set = cx.drawer_sets;
            let fx = x0 + c as f64 * (fw + gap);
            let fy = y0 + i as f64 * (fh + gap);
            let key = |part: &str| format!("w:{}:{c}:{i}:{part}", spec.uid);
            cx.panel(key("front"), format!("MặtNgăn [Bộ {set}]"), PanelRole::DrawerFront, MaterialSlot::Front, GrainDirection::AlongWidth, [fw, fh, ft], [fx, fy, z], [0.0; 3]);
            if cx.cab.handles {
                cx.hardware(key("handle"), format!("TayNắm [Bộ {set}]"), HardwareKind::Handle, "HDL-BAR-160", [160.0, 12.0, 30.0], [fx + fw / 2.0 - 80.0, fy + fh / 2.0 - 6.0, z + ft]);
                cx.out.fittings.handles += 1;
            }
            *cx.out.fittings.slides.entry(slide as u32).or_default() += 1;
            if !spec.with_box {
                continue;
            }
            // Drawer box inside the opening of this front.
            let bxz = zx + c as f64 * (cw + cx.v.thickness);
            let cell_y0 = zy + zh * i as f64 / n as f64;
            let cell_h = zh / n as f64;
            let bt = spec.box_thickness;
            let bb = spec.bottom_thickness;
            let sc = spec.slide_clearance;
            let bw = (cw - 2.0 * sc).max(2.0 * bt + 10.0);
            let hb = (cell_h - 40.0).clamp(60.0, 250.0);
            let by = cell_y0 + 15.0;
            let bx = bxz + sc;
            cx.panel(key("sideL"), format!("ThànhTrái [Bộ {set}]"), PanelRole::DrawerSide, MaterialSlot::Carcass, GrainDirection::AlongWidth, [slide, hb, bt], [bx, by, zf], ROT_SIDE);
            cx.panel(key("sideR"), format!("ThànhPhải [Bộ {set}]"), PanelRole::DrawerSide, MaterialSlot::Carcass, GrainDirection::AlongWidth, [slide, hb, bt], [bx + bw - bt, by, zf], ROT_SIDE);
            cx.panel(key("back"), format!("HậuHộc [Bộ {set}]"), PanelRole::DrawerBack, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bw - 2.0 * bt, hb - bb, bt], [bx + bt, by + bb, zf - slide], [0.0; 3]);
            cx.panel(key("front_inner"), format!("ĐầuHộc [Bộ {set}]"), PanelRole::DrawerBack, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bw - 2.0 * bt, hb - bb, bt], [bx + bt, by + bb, zf - bt], [0.0; 3]);
            cx.panel(key("bottom"), format!("ĐáyNgănKéo [Bộ {set}]"), PanelRole::DrawerBottom, MaterialSlot::Back, GrainDirection::AlongWidth, [bw - 2.0 * bt, slide, bb], [bx + bt, by, zf], ROT_HORIZONTAL);
            cx.hardware(key("slideL"), format!("RayBi {} [Bộ {set}]", slide as u32), HardwareKind::Slide, &format!("RAYBI-{}", slide as u32), [sc - 0.5, 45.0, slide], [bxz, by + hb / 2.0 - 22.5, zf - slide]);
            cx.hardware(key("slideR"), format!("RayBi {} [Bộ {set}]", slide as u32), HardwareKind::Slide, &format!("RAYBI-{}", slide as u32), [sc - 0.5, 45.0, slide], [bx + bw + 0.5, by + hb / 2.0 - 22.5, zf - slide]);
        }
    }
}

fn link(cx: &mut Ctx, l: &Link, b: &ZBox) {
    match l.kind {
        LinkKind::OvalRail => {
            let [x, y, z] = b.min;
            let [w, h, d] = b.size;
            let len = (w - 4.0).max(10.0);
            let k = cx.next("ThanhOval");
            cx.hardware(format!("l:{}", l.uid), format!("ThanhOval_{k:02}"), HardwareKind::Rail, "OVAL-30x15", [len, 15.0, 30.0], [x + 2.0, y + h - l.offset - 15.0, z + d / 2.0 - 15.0]);
            *cx.out.fittings.oval_rails.entry(len.round() as u32).or_default() += 1;
            cx.out.fittings.oval_cups += 2;
        }
    }
}

/// Reference position of a feature along an axis (for assigning it to a piece).
fn feature_anchor(f: &MachiningFeature, axis: crate::Axis2) -> f64 {
    let x = axis == crate::Axis2::X;
    match f {
        MachiningFeature::Drill(d) => if x { d.x } else { d.y },
        MachiningFeature::Pocket(p) => if x { p.x + p.width / 2.0 } else { p.y + p.height / 2.0 },
        MachiningFeature::Groove(g) => if x { g.x } else { g.y },
        MachiningFeature::EdgeDrill(e) => e.offset,
        MachiningFeature::Contour(c) => {
            let (mn, mx) = c.polygon.bounds();
            if x { (mn.x + mx.x) / 2.0 } else { (mn.y + mx.y) / 2.0 }
        }
    }
}

/// Replace split parts by their pieces (after the other mods).
fn split_parts(out: &mut Layout, mods: &BTreeMap<String, PartMod>) {
    let mut parts = Vec::with_capacity(out.parts.len());
    for p in std::mem::take(&mut out.parts) {
        let Some(sp) = mods.get(&p.key).and_then(|m| m.split) else {
            parts.push(p);
            continue;
        };
        let n = sp.count.max(1) as usize;
        let ax = if sp.axis == crate::Axis2::X { 0 } else { 1 };
        let len = p.size[ax];
        let gap = sp.gap.max(0.0);
        let piece = (len - gap * (n as f64 - 1.0)) / n as f64;
        if n < 2 || piece < 1.0 || !matches!(p.kind, PartKind::Panel { .. }) {
            parts.push(p);
            continue;
        }
        let t = aic_math::Transform3D::new(p.translation, p.rotation_deg);
        for i in 0..n {
            let start = i as f64 * (piece + gap);
            let mut q = p.clone();
            if i > 0 {
                q.key = format!("{}~{}", p.key, i + 1);
            }
            q.name = format!("{}.{}", p.name, i + 1);
            q.size[ax] = piece;
            let mut off = [0.0; 3];
            off[ax] = start;
            q.translation = t.transform_point(off);
            if let PartKind::Panel { features, .. } = &mut q.kind {
                // Keep machining that falls on this piece; an outer shape does not survive a split.
                features.retain(|f| {
                    let a = feature_anchor(f, sp.axis);
                    a >= start - 1e-6 && a < start + piece + gap && !matches!(f, MachiningFeature::Contour(c) if !c.inner)
                });
                for f in features.iter_mut() {
                    if ax == 0 { shift_feature(f, -start, 0.0) } else { shift_feature(f, 0.0, -start) }
                }
            }
            // Pieces follow their own name / delete mods.
            if let Some(m) = mods.get(&q.key).filter(|_| i > 0) {
                if m.deleted {
                    continue;
                }
                if let Some(nm) = &m.name {
                    q.name = nm.clone();
                }
            }
            parts.push(q);
        }
    }
    out.parts = parts;
}

fn apply_mods(out: &mut Layout, mods: &BTreeMap<String, PartMod>) {
    out.parts.retain(|p| !mods.get(&p.key).is_some_and(|m| m.deleted));
    for p in &mut out.parts {
        let Some(m) = mods.get(&p.key) else { continue };
        if let Some(n) = &m.name {
            p.name = n.clone();
        }
        if let PartKind::Panel { features, .. } = &mut p.kind {
            let [l, r, bo, to] = m.extend;
            if l != 0.0 || r != 0.0 || bo != 0.0 || to != 0.0 {
                p.size[0] = pos(p.size[0] + l + r);
                p.size[1] = pos(p.size[1] + bo + to);
                // Move the local origin by (-l, -bo) in the panel frame.
                let t = aic_math::Transform3D::new(p.translation, p.rotation_deg);
                let o = t.transform_point([-l, -bo, 0.0]);
                p.translation = o;
                // Existing machining keeps its position relative to the original origin.
                for f in features.iter_mut() {
                    shift_feature(f, l, bo);
                }
            }
            if let Some(tk) = m.thickness {
                p.size[2] = pos(tk);
            }
            features.extend(m.features.iter().cloned());
        }
    }
    split_parts(out, mods);
}

/// Move a feature by (dx, dy) in the panel's local frame.
pub fn shift_feature(f: &mut MachiningFeature, dx: f64, dy: f64) {
    match f {
        MachiningFeature::Drill(d) => {
            d.x += dx;
            d.y += dy;
        }
        MachiningFeature::Pocket(p) => {
            p.x += dx;
            p.y += dy;
        }
        MachiningFeature::Groove(g) => {
            g.x += dx;
            g.y += dy;
        }
        MachiningFeature::EdgeDrill(e) => {
            e.offset += if matches!(e.edge, EdgeSide::Left | EdgeSide::Right) { dy } else { dx };
        }
        MachiningFeature::Contour(c) => {
            for p in &mut c.polygon.points {
                p.x += dx;
                p.y += dy;
            }
        }
    }
}

/// Default zone tree for a preset (keeps the legacy counts meaningful).
pub fn default_zones(kind: CabinetKind, shelves: u32, doors: u32, drawers: u32, thickness: f64) -> ZoneTree {
    let mut t = ZoneTree::default();
    let root = t.root.id;
    if kind == CabinetKind::Wardrobe {
        t.add_panels(root, SplitKind::Divider, 1, thickness, Lock::Ratio, 0.5).expect("divider");
        let (left, right) = {
            let s = t.root.split.as_ref().unwrap();
            (s.children[0].id, s.children[1].id)
        };
        if shelves > 0 {
            t.add_panels(left, SplitKind::ShelfAdjustable, shelves, thickness, Lock::Even, 0.0).expect("shelves");
        }
        t.add_link(right, LinkKind::OvalRail, 60.0).expect("rail");
    } else if shelves > 0 {
        t.add_panels(root, SplitKind::ShelfAdjustable, shelves, thickness, Lock::Even, 0.0).expect("shelves");
    }
    set_legacy_front(&mut t, doors, drawers);
    t
}

/// Doors / drawers counts → root front.
pub fn set_legacy_front(t: &mut ZoneTree, doors: u32, drawers: u32) {
    let root = t.root.id;
    let front = if drawers > 0 {
        let uid = t.alloc();
        Some(Front::Drawers(DrawerSpec::new(uid, drawers, Mount::Overlay)))
    } else if doors > 0 {
        let uid = t.alloc();
        Some(Front::Doors(DoorSpec {
            uid,
            kind: if doors >= 2 && doors.is_multiple_of(2) { DoorKind::Double } else { DoorKind::Single },
            cols: doors,
            rows: 1,
            mount: Mount::Overlay,
            hinge: HingeSide::Left,
            thickness: None,
            gap: None,
            stop: StopRailSpec::default(),
        }))
    } else {
        None
    };
    let _ = t.set_front(root, front);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CabinetSpec, MaterialId};

    fn values(w: f64, h: f64, d: f64) -> CabinetValues {
        CabinetValues {
            width: w,
            height: h,
            depth: d,
            thickness: 17.2,
            back_thickness: 8.6,
            plinth_height: 0.0,
            door_thickness: 17.2,
            door_gap: 2.0,
            shelf_setback: 20.0,
            back_groove: 0.0,
            back_offset: 0.0,
            rail_width: 100.0,
        }
    }

    fn cabinet(kind: CabinetKind) -> Cabinet {
        let s = CabinetSpec::preset(kind);
        Cabinet::from_spec(crate::ObjectId(1), "T".into(), &s)
    }

    #[test]
    fn split_positions_ratio_and_even() {
        let p = |lock, value| SplitPanel { uid: 1, kind: SplitKind::ShelfAdjustable, thickness: 17.2, lock, value, tilt_deg: [0.0; 2] };
        // Zone 733.2 tall, 50% → 358 each side (spec P3).
        let s = solve_split(733.2, &[p(Lock::Ratio, 0.5)]);
        assert!((s[0] - 358.0).abs() < 1e-9);
        let s = solve_split(733.2, &[p(Lock::FromStart, 500.0)]);
        assert_eq!(s[0], 500.0);
        let s = solve_split(1000.0, &[p(Lock::Even, 0.0), p(Lock::Even, 0.0)]);
        let gap = (1000.0 - 34.4) / 3.0;
        assert!((s[0] - gap).abs() < 1e-9 && (s[1] - (2.0 * gap + 17.2)).abs() < 1e-9);
    }

    #[test]
    fn base_cabinet_has_five_boards_and_back() {
        let mut c = cabinet(CabinetKind::Base);
        c.zones = ZoneTree::default();
        let l = build(&c, values(800.0, 850.0, 600.0));
        let boards = l.parts.iter().filter(|p| matches!(p.kind, PartKind::Panel { .. }) && p.size[2] == 17.2).count();
        let backs = l.parts.iter().filter(|p| p.size[2] == 8.6).count();
        assert_eq!((boards, backs), (5, 1), "2 sides + bottom + 2 rails, 1 back");
    }

    #[test]
    fn divider_at_half_gives_equal_bays() {
        let mut c = cabinet(CabinetKind::Wardrobe);
        c.zones = ZoneTree::default();
        let root = c.zones.root.id;
        c.zones.add_panels(root, SplitKind::Divider, 1, 17.2, Lock::Ratio, 0.5).unwrap();
        let l = build(&c, values(1600.0, 2400.0, 600.0));
        let pos = &l.positions[0];
        // (1600 - 2*17.2 - 17.2)/2 = 774.2 (spec P4).
        assert!((pos.cell_before - 774.2).abs() < 1e-9 && (pos.cell_after - 774.2).abs() < 1e-9);
    }

    #[test]
    fn double_overlay_doors_and_hinges() {
        let mut c = cabinet(CabinetKind::Wardrobe);
        c.zones = ZoneTree::default();
        set_legacy_front(&mut c.zones, 4, 0);
        let l = build(&c, values(1600.0, 2400.0, 600.0));
        let doors: Vec<&Part> = l.parts.iter().filter(|p| matches!(p.kind, PartKind::Panel { role: PanelRole::Door, .. })).collect();
        assert_eq!(doors.len(), 4);
        assert!((doors[0].size[0] - (1600.0 - 2.0 * 2.0 - 3.0 * 2.0) / 4.0).abs() < 1e-9);
        assert_eq!(l.fittings.hinges, 16, "4 doors > 1600 tall → 4 hinges each");
    }

    #[test]
    fn drawers_generate_box_and_slides() {
        let mut c = cabinet(CabinetKind::Base);
        c.zones = ZoneTree::default();
        set_legacy_front(&mut c.zones, 0, 2);
        let l = build(&c, values(800.0, 850.0, 600.0));
        assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("MặtNgăn")).count(), 2);
        assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("ThànhTrái")).count(), 2);
        assert_eq!(l.fittings.slides.values().sum::<u32>(), 2);
        assert!(l.fittings.slides.contains_key(&550));
    }

    #[test]
    fn groove_back_extends_into_carcass() {
        let mut c = cabinet(CabinetKind::Base);
        c.zones = ZoneTree::default();
        let mut v = values(800.0, 850.0, 600.0);
        v.back_groove = 13.0;
        v.back_offset = 10.0;
        let l = build(&c, v);
        let back = l.parts.iter().find(|p| p.key == "c:back").unwrap();
        assert!((back.size[0] - (800.0 - 34.4 + 26.0)).abs() < 1e-9);
        let left = l.parts.iter().find(|p| p.key == "c:left").unwrap();
        match &left.kind {
            PartKind::Panel { features, .. } => assert!(matches!(features[0], MachiningFeature::Groove(_))),
            _ => unreachable!(),
        }
    }

    #[test]
    fn mods_stretch_and_delete() {
        let mut c = cabinet(CabinetKind::Base);
        c.zones = ZoneTree::default();
        c.mods.insert("c:left".into(), PartMod { extend: [0.0, 0.0, 0.0, 20.0], ..Default::default() });
        c.mods.insert("c:bottom".into(), PartMod { deleted: true, ..Default::default() });
        let l = build(&c, values(800.0, 850.0, 600.0));
        assert!(l.parts.iter().all(|p| p.key != "c:bottom"));
        let left = l.parts.iter().find(|p| p.key == "c:left").unwrap();
        assert!((left.size[1] - 870.0).abs() < 1e-9);
        let _ = MaterialId::new("x");
    }
}
