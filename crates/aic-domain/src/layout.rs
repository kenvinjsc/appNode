//! Cabinet builder: resolves a cabinet definition (carcass options + zone tree)
//! against current parameter values into concrete parts in the cabinet frame.
//! Pure function: no ids, no scene, no geometry kernel.
//!
//! Cabinet frame: X = width, Y = height, Z = depth (front at Z = depth).
//! Panel local frame: X = width, Y = height, Z = thickness.

use crate::cabinet::{MaterialSlot, ROT_HORIZONTAL, ROT_SIDE};
use crate::zone::*;
use crate::structure::{BaseType, HandlePos, HandleType, PinRow, ShopRules, SlideType};
use crate::{
    Axis2, Cabinet, CabinetKind, DrillFeature, DrillPurpose, EdgeSide, FaceSide, GrainDirection, GrooveFeature, HardwareKind,
    JoinStyle, MachiningFeature, PanelRole, PocketFeature,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod products;

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
    /// Offset (lùi) of the part's faces in the cabinet frame, mm inward:
    /// [left, right, bottom, top, back, front]. Converted to edge extension (or a
    /// move, along the thickness) using the part's rotation.
    #[serde(default, skip_serializing_if = "is_zero6")]
    pub offsets: [f64; 6],
    /// Ràng buộc động: an edge of this part follows a face of another part.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<EdgeAnchor>,
    /// Chia tấm: the part becomes `count` pieces along an axis with a gap between them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<PartSplit>,
    /// Gia công theo tham số (khấu góc, khấu bề mặt neo góc, rãnh LED / V-bit): tính lại theo kích
    /// thước tấm mỗi lần dựng, nên tấm đổi cỡ thì vị trí vẫn đúng.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_features: Vec<ParamFeature>,
}

/// Góc / điểm neo trên tấm (local: x sang phải, y lên trên).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PanelAnchor {
    #[default]
    Bl,
    Br,
    Tl,
    Tr,
    Center,
}

/// Gia công tham số của tool (lưu ý định, không lưu tọa độ tuyệt đối).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParamFeature {
    /// Khấu góc xuyên tấm `width × depth` tại một góc.
    Notch { corner: PanelAnchor, width: f64, depth: f64 },
    /// Khấu bề mặt: hốc cách điểm neo (x, y) — với góc phải / trên, x / y đo vào trong từ cạnh đó.
    Pocket { anchor: PanelAnchor, x: f64, y: f64, width: f64, height: f64, depth: f64, side: FaceSide },
    /// Rãnh chạy suốt tấm (LED, V-bit), cách mép đầu (hoặc mép cuối) `offset`.
    GrooveLine { direction: Axis2, offset: f64, #[serde(default)] from_end: bool, width: f64, depth: f64, side: FaceSide },
}

impl ParamFeature {
    /// Feature gia công cho tấm kích thước (w, h, t).
    pub fn resolve(&self, w: f64, h: f64, t: f64) -> MachiningFeature {
        match *self {
            ParamFeature::Notch { corner, width, depth } => {
                let (x, y) = match corner {
                    PanelAnchor::Bl => (0.0, 0.0),
                    PanelAnchor::Br => (w - width, 0.0),
                    PanelAnchor::Tl => (0.0, h - depth),
                    PanelAnchor::Tr => (w - width, h - depth),
                    PanelAnchor::Center => ((w - width) / 2.0, (h - depth) / 2.0),
                };
                MachiningFeature::Pocket(PocketFeature { x, y, width, height: depth, depth: t + 1.0, side: FaceSide::A, corner_radius: 0.0 })
            }
            ParamFeature::Pocket { anchor, x, y, width, height, depth, side } => {
                let (px, py) = match anchor {
                    PanelAnchor::Bl => (x, y),
                    PanelAnchor::Br => (w - x - width, y),
                    PanelAnchor::Tl => (x, h - y - height),
                    PanelAnchor::Tr => (w - x - width, h - y - height),
                    PanelAnchor::Center => ((w - width) / 2.0 + x, (h - height) / 2.0 + y),
                };
                MachiningFeature::Pocket(PocketFeature { x: px, y: py, width, height, depth, side, corner_radius: 0.0 })
            }
            ParamFeature::GrooveLine { direction, offset, from_end, width, depth, side } => match direction {
                Axis2::X => {
                    let y = if from_end { h - offset - width } else { offset };
                    MachiningFeature::Groove(GrooveFeature { x: 0.0, y, length: w, width, depth, direction, side })
                }
                Axis2::Y => {
                    let x = if from_end { w - offset - width } else { offset };
                    MachiningFeature::Groove(GrooveFeature { x, y: 0.0, length: h, width, depth, direction, side })
                }
            },
        }
    }
}

/// Which face of the target an anchored edge follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnchorFace {
    /// The face turned towards the constrained part (mặt trong).
    #[default]
    Inner,
    /// The far face (mặt ngoài).
    Outer,
    /// The target face on the same side as the edge (bằng mặt: edges coplanar).
    Flush,
}

/// `edge` of the part → `face` of part `target` (key in the same cabinet), keeping
/// `offset` mm between them. Example: Shelf.Right → HồiPhải.Inner, 0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeAnchor {
    pub edge: EdgeSide,
    pub target: String,
    #[serde(default)]
    pub face: AnchorFace,
    #[serde(default)]
    pub offset: f64,
}

/// Local edge of a panel pointing along `dir` (cabinet frame), if any.
pub fn edge_along(rotation_deg: [f64; 3], dir: [f64; 3], min_dot: f64) -> Option<EdgeSide> {
    let t = aic_math::Transform3D::new([0.0; 3], rotation_deg);
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len < 1e-9 {
        return None;
    }
    [(EdgeSide::Left, [-1.0, 0.0, 0.0]), (EdgeSide::Right, [1.0, 0.0, 0.0]), (EdgeSide::Bottom, [0.0, -1.0, 0.0]), (EdgeSide::Top, [0.0, 1.0, 0.0])]
        .into_iter()
        .map(|(e, l)| {
            let v = t.transform_vector(l);
            (e, (v[0] * dir[0] + v[1] * dir[1] + v[2] * dir[2]) / len)
        })
        .filter(|(_, d)| *d > min_dot)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(e, _)| e)
}

/// Cabinet-frame AABB of a part.
pub fn part_aabb(p: &Part) -> ([f64; 3], [f64; 3]) {
    let t = aic_math::Transform3D::new(p.translation, p.rotation_deg);
    let (mut mn, mut mx) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for i in 0..8 {
        let c = [if i & 1 != 0 { p.size[0] } else { 0.0 }, if i & 2 != 0 { p.size[1] } else { 0.0 }, if i & 4 != 0 { p.size[2] } else { 0.0 }];
        let w = t.transform_point(c);
        for k in 0..3 {
            mn[k] = mn[k].min(w[k]);
            mx[k] = mx[k].max(w[k]);
        }
    }
    (mn, mx)
}

/// Grow edges [l, r, b, t] of a panel (negative shrinks), keeping machining on the material.
fn stretch(p: &mut Part, ext: [f64; 4]) {
    let [l, r, bo, to] = ext;
    if l == 0.0 && r == 0.0 && bo == 0.0 && to == 0.0 {
        return;
    }
    p.size[0] = pos(p.size[0] + l + r);
    p.size[1] = pos(p.size[1] + bo + to);
    let t = aic_math::Transform3D::new(p.translation, p.rotation_deg);
    p.translation = t.transform_point([-l, -bo, 0.0]);
    if let PartKind::Panel { features, .. } = &mut p.kind {
        for f in features.iter_mut() {
            shift_feature(f, l, bo);
        }
    }
}

/// Resolve edge anchors (after the other mods): each anchored edge is stretched to
/// the target face ± offset. Targets are read before any anchor is applied.
fn solve_anchors(out: &mut Layout, mods: &BTreeMap<String, PartMod>) {
    let boxes: BTreeMap<String, ([f64; 3], [f64; 3])> = out.parts.iter().map(|p| (p.key.clone(), part_aabb(p))).collect();
    for p in &mut out.parts {
        let Some(m) = mods.get(&p.key).filter(|m| !m.anchors.is_empty()) else { continue };
        let (smn, smx) = part_aabb(p);
        let t = aic_math::Transform3D::new([0.0; 3], p.rotation_deg);
        let mut ext = [0.0; 4];
        for a in &m.anchors {
            let Some((tmn, tmx)) = boxes.get(&a.target) else { continue };
            let (local, i) = match a.edge {
                EdgeSide::Left => ([-1.0, 0.0, 0.0], 0),
                EdgeSide::Right => ([1.0, 0.0, 0.0], 1),
                EdgeSide::Bottom => ([0.0, -1.0, 0.0], 2),
                EdgeSide::Top => ([0.0, 1.0, 0.0], 3),
            };
            let d = t.transform_vector(local);
            let axis = (0..3).max_by(|&x, &y| d[x].abs().total_cmp(&d[y].abs())).unwrap();
            let s = d[axis].signum();
            let edge = if s > 0.0 { smx[axis] } else { smn[axis] };
            let centre = (smn[axis] + smx[axis]) / 2.0;
            let target_ahead = (tmn[axis] + tmx[axis]) / 2.0 > centre;
            let (inner, outer) = if target_ahead { (tmn[axis], tmx[axis]) } else { (tmx[axis], tmn[axis]) };
            let face = match a.face {
                AnchorFace::Inner => inner,
                AnchorFace::Outer => outer,
                AnchorFace::Flush => if s > 0.0 { tmx[axis] } else { tmn[axis] },
            };
            let goal = face - s * a.offset;
            ext[i] += (goal - edge) * s;
        }
        stretch(p, ext);
    }
}

fn is_zero6(v: &[f64; 6]) -> bool {
    v.iter().all(|x| *x == 0.0)
}

/// Cabinet-frame directions of the six offsets.
const OFFSET_DIRS: [[f64; 3]; 6] = [[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0], [0.0, 0.0, 1.0]];

/// Offsets → (edge extension [l, r, b, t] in the panel frame, translation in the
/// cabinet frame for offsets along the thickness).
pub fn offsets_to_local(rotation_deg: [f64; 3], offsets: &[f64; 6]) -> ([f64; 4], [f64; 3]) {
    let t = aic_math::Transform3D::new([0.0; 3], rotation_deg);
    let (ax, ay) = (t.transform_vector([1.0, 0.0, 0.0]), t.transform_vector([0.0, 1.0, 0.0]));
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut ext = [0.0; 4];
    let mut mv = [0.0; 3];
    for (d, off) in OFFSET_DIRS.iter().zip(offsets) {
        if *off == 0.0 {
            continue;
        }
        let (dx, dy) = (dot(ax, *d), dot(ay, *d));
        if dx > 0.9 {
            ext[1] -= off;
        } else if dx < -0.9 {
            ext[0] -= off;
        } else if dy > 0.9 {
            ext[3] -= off;
        } else if dy < -0.9 {
            ext[2] -= off;
        } else {
            for k in 0..3 {
                mv[k] -= d[k] * off;
            }
        }
    }
    (ext, mv)
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
    /// Núm tay nắm.
    #[serde(default)]
    pub knobs: u32,
    /// Bộ nhấn mở (push-open).
    #[serde(default)]
    pub push_latches: u32,
    /// Ray âm giảm chấn (dài → bộ).
    #[serde(default)]
    pub undermount: BTreeMap<u32, u32>,
    /// Hộp kim loại tandem (dài → bộ).
    #[serde(default)]
    pub tandem: BTreeMap<u32, u32>,
    /// Chân nhựa tăng chỉnh.
    #[serde(default)]
    pub legs: u32,
    /// Ke treo tủ.
    #[serde(default)]
    pub hangers: u32,
    /// Vít bắt hậu ốp.
    #[serde(default)]
    pub back_screws: u32,
    /// Ben hơi (giường nâng).
    #[serde(default)]
    pub gas_lifts: u32,
    /// Profile nhôm cánh lùa / cánh kính (mm dài).
    #[serde(default)]
    pub alu_profile_mm: f64,
    /// Kính / gương (mm²).
    #[serde(default)]
    pub glass_mm2: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub parts: Vec<Part>,
    pub zones: Vec<ZoneBox>,
    pub positions: Vec<PanelPosition>,
    /// Every bay (khoang) of every split, for editable dimensions.
    pub bays: Vec<BayInfo>,
    /// Drawer fronts (heights) for editable dimensions.
    pub front_bays: Vec<FrontBay>,
    pub fittings: Fittings,
    /// Zones whose bays cannot be solved (too small / conflicting locks).
    pub problems: Vec<Uid>,
}

/// One drawer front of a stack, resolved (cabinet frame), for editable heights.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrontBay {
    /// Drawer spec uid.
    pub uid: Uid,
    /// 0 = bottom front.
    pub index: usize,
    pub start: f64,
    pub size: f64,
    pub x0: f64,
    pub x1: f64,
    pub z: f64,
    pub mode: Option<BayMode>,
    pub value: f64,
    pub usable: f64,
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
    /// Hàng lỗ chốt 32 đã khoan (part, y0, h) — không khoan trùng khi nhiều kệ cùng khoang.
    pin_rows: std::collections::HashSet<(usize, i64, i64)>,
    /// Hậu chờ dựng sau khoang (kệ cố định, khoét hậu).
    back: Option<BackPlan>,
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
    let mut cx = Ctx { cab, v, out: Layout::default(), counters: BTreeMap::new(), drawer_sets: 0, pin_rows: Default::default(), back: None };
    if let Some(dg) = cab.rules.diagonal.clone() {
        diagonal_corner(&mut cx, &dg);
        apply_mods(&mut cx.out, &cab.mods);
        return cx.out;
    }
    if let Some(p) = cab.rules.product.clone() {
        products::build(&mut cx, &p);
        apply_mods(&mut cx.out, &cab.mods);
        return cx.out;
    }
    let root = carcass(&mut cx);
    zone(&mut cx, &cab.zones.root, root, 0);
    backs(&mut cx);
    trims(&mut cx);
    apply_mods(&mut cx.out, &cab.mods);
    cx.out
}

/// Đa giác (tọa độ local tấm) → feature đường bao ngoài.
fn outline(pts: &[(f64, f64)], t: f64) -> MachiningFeature {
    MachiningFeature::Contour(crate::ContourFeature {
        polygon: aic_math::Polygon2D::new(pts.iter().map(|(x, y)| aic_math::Point2::new(*x, *y)).collect()),
        inner: false,
        depth: t,
    })
}

/// Tủ góc chéo (mặt bằng): góc tường ở (x = 0, z = 0), hai tường theo trục x và z, dài `w`;
/// hai hồi vuông góc tường ở hai đầu (sâu `d`), mặt cánh chéo từ (w, d) tới (d, w).
/// Nóc / đáy / kệ là tấm 5 cạnh (đường bao ngoài), cánh xoay 45° quanh trục đứng.
fn diagonal_corner(cx: &mut Ctx, dg: &crate::structure::DiagonalCorner) {
    let v = cx.v;
    let (w, h, d, t, bt, p) = (v.width, v.height, v.depth.min(v.width - 150.0).max(200.0), v.thickness, v.back_thickness, v.plinth_height);
    let hi = h - p - 2.0 * t;
    // Hồi phải (đầu dãy tường x) và hồi trái (đầu dãy tường z), đặt trên đáy, dưới nóc.
    cx.panel("c:right".into(), "HồiPhải".into(), PanelRole::RightSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, hi, t], [w - t, p + t, d], ROT_SIDE);
    cx.panel("c:left".into(), "HồiTrái".into(), PanelRole::LeftSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, hi, t], [0.0, p + t, w - t], [0.0; 3]);
    // Nóc / đáy 5 cạnh: tấm vuông w × w (local x = x, local y = w − z), cắt góc chéo.
    let penta = [(0.0, 0.0), (d, 0.0), (w, w - d), (w, w), (0.0, w)];
    let b = cx.panel("c:bottom".into(), "Đáy".into(), PanelRole::Bottom, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, w, t], [0.0, p, w], ROT_HORIZONTAL);
    cx.add_features(b, vec![outline(&penta, t)]);
    let top = cx.panel("c:top".into(), "Nóc".into(), PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, w, t], [0.0, h - t, w], ROT_HORIZONTAL);
    cx.add_features(top, vec![outline(&penta, t)]);
    // Hai hậu áp tường, giữa đáy và nóc.
    if cx.cab.back_panel {
        cx.panel("c:back".into(), "Hậu".into(), PanelRole::Back, MaterialSlot::Back, GrainDirection::AlongHeight, [w - t, hi, bt], [0.0, p + t, 0.0], [0.0; 3]);
        cx.panel("c:back2".into(), "Hậu_02".into(), PanelRole::Back, MaterialSlot::Back, GrainDirection::AlongHeight, [w - t - bt, hi, bt], [0.0, p + t, w - t], ROT_SIDE);
    }
    // Kệ cố định 5 cạnh, lùi sau mặt cánh.
    let (x0, x1) = (bt, w - t);
    let s = dg.shelf_setback.max(0.0) * std::f64::consts::SQRT_2;
    let diag = w + d - s; // x + z ≤ diag
    let n = dg.shelves.min(8);
    for i in 0..n {
        let y = p + t + hi * (i + 1) as f64 / (n + 1) as f64 - t / 2.0;
        let sz = x1 - x0;
        // local (x − x0, x1 − z)
        let pts = [(0.0, 0.0), (diag - x1 - x0, 0.0), (sz, sz - (diag - x1 - x0)), (sz, sz), (0.0, sz)];
        let k = cx.next("KệCốĐịnh");
        let idx = cx.panel(format!("p:dg{i}"), format!("KệCốĐịnh_{k:02}"), PanelRole::ShelfFixed, MaterialSlot::Carcass, GrainDirection::AlongWidth, [sz, sz, t], [x0, y, x1], ROT_HORIZONTAL);
        cx.add_features(idx, vec![outline(&pts, t)]);
    }
    // Cánh chéo: mặt sau nằm trên đường x + z = w + d, local x chạy từ (d, w) tới (w, d).
    let gap = cx.v.door_gap;
    let dt = cx.v.door_thickness;
    let len = std::f64::consts::SQRT_2 * (w - d) - 2.0 * gap;
    let dh = h - p - 2.0 * gap;
    let u = std::f64::consts::FRAC_1_SQRT_2;
    let (sx, sz) = (d + gap * u, w - gap * u);
    let hinge = if dg.hinge_left { HingeSide::Left } else { HingeSide::Right };
    let door = cx.panel("d:dg".into(), "CửaChéo".into(), PanelRole::Door, MaterialSlot::Front, GrainDirection::AlongHeight, [len, dh, dt], [sx, p + gap, sz], [0.0, 45.0, 0.0]);
    let sr = cx.cab.rules.shop.clone();
    cx.add_features(door, hinge_cups(&sr, len, dh, hinge));
    if let PartKind::Panel { hinge: hs, .. } = &mut cx.out.parts[door].kind {
        *hs = Some(to_edge(hinge));
    }
    cx.out.fittings.hinges += sr.hinge_count(dh);
    cx.out.zones.push(ZoneBox { id: cx.cab.zones.root.id, min: [bt, p + t, bt], size: [w - t - bt, hi, w - t - bt], leaf: true, depth: 0, has_front: true });
}

/// Chân tủ: len chân (trước / 3 mặt), chân nhựa, ke treo.
fn base(cx: &mut Ctx, side_y: f64, inner_w: f64) {
    let v = cx.v;
    let (w, h, d, t, p) = (v.width, v.height, v.depth, v.thickness, v.plinth_height);
    let rules = cx.cab.rules.clone();
    let kind = match rules.base_type {
        BaseType::Auto => {
            if matches!(cx.cab.kind, CabinetKind::Wardrobe | CabinetKind::Drawer | CabinetKind::Base) && p > 1.0 {
                BaseType::Plinth
            } else {
                BaseType::None
            }
        }
        k => k,
    };
    let sb = rules.plinth_setback;
    let front_z = d - sb - t;
    let has_plinth = matches!(kind, BaseType::Plinth | BaseType::Plinth3 | BaseType::LegsPlinth) && p > 1.0;
    if has_plinth {
        // Hồi chạm sàn: len nằm giữa 2 hồi; hồi đặt trên đáy phủ: len chạy suốt rộng tủ.
        let (pw, px) = if side_y > 0.0 { (w - 2.0 * rules.plinth_side_setback, rules.plinth_side_setback) } else { (inner_w, t) };
        cx.panel("c:plinth".into(), "ChânTủ".into(), PanelRole::Plinth, MaterialSlot::Carcass, GrainDirection::AlongWidth, [pw, p, t], [px, 0.0, front_z], [0.0; 3]);
        if kind == BaseType::Plinth3 && side_y > 0.0 {
            let len = front_z.max(1.0);
            let ss = rules.plinth_side_setback;
            cx.panel("c:plinth_l".into(), "ChânHôngTrái".into(), PanelRole::Plinth, MaterialSlot::Carcass, GrainDirection::AlongWidth, [len, p, t], [ss, 0.0, front_z], ROT_SIDE);
            cx.panel("c:plinth_r".into(), "ChânHôngPhải".into(), PanelRole::Plinth, MaterialSlot::Carcass, GrainDirection::AlongWidth, [len, p, t], [w - t - ss, 0.0, front_z], ROT_SIDE);
        }
    }
    if matches!(kind, BaseType::Legs | BaseType::LegsPlinth) && p > 1.0 {
        let n = if rules.leg_count > 0 { rules.leg_count.clamp(4, 16) } else if w <= 600.0 { 4 } else if w <= 1200.0 { 6 } else { 8 };
        let cols = (n / 2).max(2);
        let dia = 30.0;
        let zs = [60.0, (front_z - 40.0).max(60.0)];
        let mut k = 0;
        for c in 0..cols {
            let x = 50.0 + (w - 100.0 - dia) * c as f64 / (cols - 1) as f64;
            for z in zs {
                k += 1;
                cx.hardware(format!("c:leg:{k}"), format!("ChânNhựa_{k:02}"), HardwareKind::Leg, "LEG-ADJ-100", [dia, p, dia], [x, 0.0, z - dia / 2.0]);
            }
        }
        cx.out.fittings.legs += k;
    }
    if kind == BaseType::Hanging {
        // 2 ke treo ở hai góc sau trên, thanh treo tường (tùy chọn) chạy trong lòng tủ.
        cx.hardware("c:hang:l".into(), "KeTreo_01".into(), HardwareKind::Hinge, "HANGER", [30.0, 60.0, 40.0], [t, h - t - 70.0, 0.0]);
        cx.hardware("c:hang:r".into(), "KeTreo_02".into(), HardwareKind::Hinge, "HANGER", [30.0, 60.0, 40.0], [w - t - 30.0, h - t - 70.0, 0.0]);
        cx.out.fittings.hangers += 2;
        if rules.hang_rail {
            cx.panel("c:hang_rail".into(), "ThanhTreoTường".into(), PanelRole::Rail, MaterialSlot::Carcass, GrainDirection::AlongWidth, [inner_w, 60.0, t], [t, h - t - 70.0, -t], [0.0; 3]);
        }
    }
}

/// Carcass parts; returns the interior root zone box.
fn carcass(cx: &mut Ctx) -> ZBox {
    let v = cx.v;
    let (w, h, d, t, bt, p) = (v.width, v.height, v.depth, v.thickness, v.back_thickness, v.plinth_height);
    let cab = cx.cab;
    let top_overlay = cab.top_style == JoinStyle::Overlay;
    let rails = cab.top_style == JoinStyle::Rails;
    let bottom_overlay = cab.bottom_style == JoinStyle::Overlay;
    let rules = &cab.rules;
    // Hậu ốp: the back covers the whole rear; the carcass stands in front of it.
    let overlay = cab.back_panel && rules.back.overlay;
    let groove = cab.back_panel && !overlay && v.back_groove > 0.0;
    // Z where the carcass horizontals start (behind them: the back).
    let back_front = if !cab.back_panel {
        0.0
    } else if groove {
        v.back_offset + bt
    } else {
        bt
    };
    // Where the top / bottom start in depth: behind them sits a lapped back unless
    // they cover it (nóc / đáy trùm hậu).
    let (top_start, bottom_start) = if groove || !cab.back_panel {
        (0.0, 0.0)
    } else if overlay {
        (bt, bt)
    } else {
        (if rules.back.top_covers == Some(true) { 0.0 } else { bt }, if rules.back.bottom_covers == Some(true) { 0.0 } else { bt })
    };
    // Hồi đứng trên chân nhựa (không chạm sàn) khi dùng chân nhựa.
    let on_legs = matches!(rules.base_type, crate::structure::BaseType::Legs | crate::structure::BaseType::LegsPlinth);
    let side_y = if bottom_overlay { p + t } else if on_legs { p } else { 0.0 };
    let side_h = h - side_y - if top_overlay { t } else { 0.0 };

    let side_d = if overlay { d - bt } else { d };
    let l = cx.panel("c:left".into(), "HồiTrái".into(), PanelRole::LeftSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [side_d, side_h, t], [0.0, side_y, d], ROT_SIDE);
    let r = cx.panel("c:right".into(), "HồiPhải".into(), PanelRole::RightSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [side_d, side_h, t], [w - t, side_y, d], ROT_SIDE);

    let inner_w = w - 2.0 * t;
    let top_w = if top_overlay { w } else { inner_w };
    let top_x = if top_overlay { 0.0 } else { t };
    let bottom_w = if bottom_overlay { w } else { inner_w };
    let bottom_x = if bottom_overlay { 0.0 } else { t };
    let top_depth = d - top_start;
    let bottom_depth = d - bottom_start;
    let mut top_idx = None;
    if rails {
        top_rails(cx, &rules.top_rails, inner_w, top_depth, top_start);
    } else {
        top_idx = Some(cx.panel("c:top".into(), "Nóc".into(), PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [top_w, top_depth, t], [top_x, h - t, d], ROT_HORIZONTAL));
    }
    let b = cx.panel("c:bottom".into(), "Đáy".into(), PanelRole::Bottom, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bottom_w, bottom_depth, t], [bottom_x, p, d], ROT_HORIZONTAL);

    if cab.back_panel {
        let [gl, gr, gt, gb] = rules.back.gaps;
        if groove {
            // Rãnh sâu C (= back_groove); the back enters C − khe hở.
            let g = (v.back_groove - rules.back.clearance).max(0.0);
            let bw = inner_w + 2.0 * g;
            let bh = h - p - 2.0 * t + 2.0 * g;
            cx.back = Some(BackPlan { rect: [t - g + gl, p + t - g + gb, bw - gl - gr, bh - gb - gt], z: v.back_offset, bt, lapped: false });
            let tol = 0.5;
            let gw = bt + tol;
            // Groove across the inner faces: sides (along height), top/bottom (along width).
            // Side local X runs from the front (x = 0 ↔ z = d) to the back.
            let gx = d - v.back_offset - bt - tol / 2.0;
            let sy0 = p + t - g - side_y;
            let side_len = bh;
            let depth = v.back_groove.min(t - 4.0).max(1.0);
            cx.add_features(l, vec![MachiningFeature::Groove(GrooveFeature { x: gx, y: sy0, length: side_len, width: gw, depth, direction: Axis2::Y, side: FaceSide::A })]);
            cx.add_features(r, vec![MachiningFeature::Groove(GrooveFeature { x: gx, y: sy0, length: side_len, width: gw, depth, direction: Axis2::Y, side: FaceSide::B })]);
            let hy = d - v.back_offset - bt - tol / 2.0;
            let gx_top = (t - g) - top_x;
            if let Some(ti) = top_idx {
                cx.add_features(ti, vec![MachiningFeature::Groove(GrooveFeature { x: gx_top, y: hy, length: bw, width: gw, depth, direction: Axis2::X, side: FaceSide::B })]);
            }
            let gx_bot = (t - g) - bottom_x;
            cx.add_features(b, vec![MachiningFeature::Groove(GrooveFeature { x: gx_bot, y: hy, length: bw, width: gw, depth, direction: Axis2::X, side: FaceSide::A })]);
        } else if overlay {
            // Hậu ốp: whole rear, from the bottom of the sides to the top; screwed on
            // the carcass edges every ~150 mm.
            let (bw, bh) = (w - gl - gr, h - side_y - gb - gt);
            cx.back = Some(BackPlan { rect: [gl, side_y + gb, bw, bh], z: 0.0, bt, lapped: false });
            let n = |len: f64| ((len - 60.0).max(0.0) / 150.0).ceil() as u32 + 1;
            cx.out.fittings.back_screws += 2 * n(bw) + 2 * n(bh);
        } else {
            // Lapped back: from the bottom (or on it when the bottom covers the back)
            // to under the top (or to the top edge when the top does not cover it).
            let y0 = if rules.back.bottom_covers == Some(true) { p + t } else { p };
            let y1 = if rules.back.top_covers == Some(false) { h } else { h - t };
            cx.back = Some(BackPlan { rect: [t + gl, y0 + gb, inner_w - gl - gr, y1 - y0 - gb - gt], z: 0.0, bt, lapped: true });
        }
    }

    base(cx, side_y, inner_w);

    let outer = |part| Neighbor { t, outer: true, part };
    ZBox {
        min: [t, p + t, back_front],
        size: [inner_w, h - p - 2.0 * t, d - back_front],
        nb: [outer(Some(l)), outer(Some(r)), outer(Some(b)), outer(top_idx)],
    }
}

/// Tấm hậu chờ dựng: được dựng sau các khoang để biết vị trí kệ cố định.
#[derive(Debug, Clone, Copy)]
struct BackPlan {
    /// [x, y, w, h] in the cabinet front plane.
    rect: [f64; 4],
    z: f64,
    bt: f64,
    /// Hậu lọt giữa hồi (chia theo kệ cố định được).
    lapped: bool,
}

/// Back board(s): one panel, vertical pieces when "Chia dọc" is on, sections between
/// full-width fixed shelves when "Hậu chia theo kệ cố định" is on; cutouts (khoét hậu)
/// become inner contours on the piece that holds them.
fn backs(cx: &mut Ctx) {
    let Some(plan) = cx.back else { return };
    let rule = &cx.cab.rules.back;
    let [x, y, w, h] = plan.rect;
    let (t, bt) = (cx.v.thickness, plan.bt);
    // Sections along the height.
    let mut rows = vec![(y, y + h)];
    if plan.lapped && rule.split_at_fixed {
        let inner_w = cx.v.width - 2.0 * t;
        let mut cuts: Vec<(usize, f64, f64)> = Vec::new();
        for (i, p) in cx.out.parts.iter().enumerate() {
            let fixed = matches!(p.kind, PartKind::Panel { role: PanelRole::ShelfFixed, .. });
            let rear = p.translation[2] - p.size[1];
            if fixed && p.rotation_deg == ROT_HORIZONTAL && p.size[0] >= inner_w - 1.0 && (rear - plan.bt).abs() < 0.5 {
                let (s0, s1) = (p.translation[1], p.translation[1] + p.size[2]);
                if s0 > y + 20.0 && s1 < y + h - 20.0 {
                    cuts.push((i, s0, s1));
                }
            }
        }
        cuts.sort_by(|a, b| a.1.total_cmp(&b.1));
        if !cuts.is_empty() {
            rows.clear();
            let mut at = y;
            for (i, s0, s1) in cuts {
                // The shelf runs through to the rear edge, between two backs.
                cx.out.parts[i].size[1] += bt;
                rows.push((at, s0));
                at = s1;
            }
            rows.push((at, y + h));
        }
    }
    let (v, cuts) = (cx.v, rule.cutouts.clone());
    let centers: Vec<(f64, f64)> = cuts
        .iter()
        .map(|c| {
            let cxw = match c.anchor {
                crate::structure::HAnchor::Left => t + c.x,
                crate::structure::HAnchor::Center => v.width / 2.0 + c.x,
                crate::structure::HAnchor::Right => v.width - t - c.x,
            };
            (cxw, v.plinth_height + t + c.y)
        })
        .collect();
    let mut n = 0;
    for (y0, y1) in rows {
        let mut at = x;
        for pw in rule.pieces(w.max(1.0)) {
            let (key, name) = if n == 0 { ("c:back".to_string(), "Hậu".to_string()) } else { (format!("c:back:{}", n + 1), format!("Hậu_{}", n + 1)) };
            n += 1;
            let ph = (y1 - y0).max(1.0);
            let idx = cx.panel(key, name, PanelRole::Back, MaterialSlot::Back, GrainDirection::AlongHeight, [pw, ph, bt], [at, y0, plan.z], [0.0; 3]);
            let mut feats = Vec::new();
            for (c, (ccx, ccy)) in cuts.iter().zip(&centers) {
                let (cw, ch, r) = c.shape();
                let (lx, ly) = (ccx - at, ccy - y0);
                let m = 5.0;
                if cw > 0.0 && lx - cw / 2.0 >= m && lx + cw / 2.0 <= pw - m && ly - ch / 2.0 >= m && ly + ch / 2.0 <= ph - m {
                    feats.push(MachiningFeature::Contour(crate::ContourFeature { polygon: rounded_rect(lx, ly, cw, ch, r), inner: true, depth: bt }));
                }
            }
            cx.add_features(idx, feats);
            at += pw;
        }
    }
}

/// Phào & ốp (D13): ốp hông ngoài hồi, nẹp che khe, phào nóc / phào chân 1–3 mặt.
/// Phào hông chạy ra ngoài ốp hông; góc vát 45° ghi ở tên thanh (danh sách cắt).
fn trims(cx: &mut Ctx) {
    let tr = cx.cab.rules.trim;
    let v = cx.v;
    let (w, h, d, t) = (v.width, v.height, v.depth, v.thickness);
    let et = if tr.end_t > 0.0 { tr.end_t } else { t };
    let ef = tr.end_front.clamp(0.0, 50.0);
    let (y0, eh) = if tr.end_to_floor { (0.0, h) } else { (v.plinth_height, h - v.plinth_height) };
    let (mut xl, mut xr, mut df) = (0.0, w, d);
    if tr.end_left {
        cx.panel("c:end_l".into(), "ỐpHôngTrái".into(), PanelRole::Trim, MaterialSlot::Front, GrainDirection::AlongHeight, [d + ef, eh, et], [-et, y0, d + ef], ROT_SIDE);
        xl = -et;
        df = df.max(d + ef);
    }
    if tr.end_right {
        cx.panel("c:end_r".into(), "ỐpHôngPhải".into(), PanelRole::Trim, MaterialSlot::Front, GrainDirection::AlongHeight, [d + ef, eh, et], [w, y0, d + ef], ROT_SIDE);
        xr = w + et;
        df = df.max(d + ef);
    }
    // Nẹp che khe: thanh đứng sát mặt trước, ngoài cùng hai bên.
    for (on, key, name, left) in [(tr.scribe_left, "c:scribe_l", "NẹpTrái", true), (tr.scribe_right, "c:scribe_r", "NẹpPhải", false)] {
        if on > 0.0 {
            let sw = on.clamp(10.0, 100.0);
            let x = if left { xl - sw } else { xr };
            cx.panel(key.into(), name.into(), PanelRole::Trim, MaterialSlot::Front, GrainDirection::AlongHeight, [sw, eh, t], [x, y0, df - t], [0.0; 3]);
            if left {
                xl -= sw;
            } else {
                xr += sw;
            }
        }
    }
    let (front, left, right) = tr.cornice.sides();
    let o = tr.cornice_overhang.clamp(0.0, 100.0);
    let tag = if tr.cornice_miter { " (vát 45°)" } else { "" };
    for (base, hh, y, name) in [("c:cornice", tr.cornice_h.clamp(20.0, 200.0), h, "PhàoNóc"), ("c:skirt", tr.skirting_h.clamp(0.0, 200.0), 0.0, "PhàoChân")] {
        if !front || (base == "c:skirt" && tr.skirting_h <= 0.0) {
            continue;
        }
        // Phào chân nằm trong khoảng chân tủ, phào nóc trên nóc.
        let zf = df + o;
        let (x0, x1) = (if left { xl - o } else { xl }, if right { xr + o } else { xr });
        cx.panel(format!("{base}_f"), format!("{name}Trước{tag}"), PanelRole::Trim, MaterialSlot::Front, GrainDirection::AlongWidth, [x1 - x0, hh, t], [x0, y, zf - t], [0.0; 3]);
        if left {
            cx.panel(format!("{base}_l"), format!("{name}Trái{tag}"), PanelRole::Trim, MaterialSlot::Front, GrainDirection::AlongWidth, [zf, hh, t], [xl - o, y, zf], ROT_SIDE);
        }
        if right {
            cx.panel(format!("{base}_r"), format!("{name}Phải{tag}"), PanelRole::Trim, MaterialSlot::Front, GrainDirection::AlongWidth, [zf, hh, t], [xr + o - t, y, zf], ROT_SIDE);
        }
    }
}

/// Chữ nhật bo góc tâm (cx, cy), cạnh (w, h), bán kính r (r = w/2 = h/2 → hình tròn).
fn rounded_rect(cx: f64, cy: f64, w: f64, h: f64, r: f64) -> aic_math::Polygon2D {
    let (hw, hh) = (w / 2.0, h / 2.0);
    let r = r.clamp(0.0, hw.min(hh));
    let mut pts = Vec::new();
    if r < 0.5 {
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            pts.push(aic_math::Point2::new(cx + sx * hw, cy + sy * hh));
        }
    } else {
        // Corner centres counter-clockwise from bottom-right, 8 segments per corner.
        let corners = [(hw - r, -(hh - r), -90.0), (hw - r, hh - r, 0.0), (-(hw - r), hh - r, 90.0), (-(hw - r), -(hh - r), 180.0)];
        for (ox, oy, a0) in corners {
            for k in 0..=8 {
                let a = (a0 + k as f64 * 90.0 / 8.0_f64).to_radians();
                let p = aic_math::Point2::new(cx + ox + r * a.cos(), cy + oy + r * a.sin());
                if pts.last().is_none_or(|q: &aic_math::Point2| (q.x - p.x).abs() > 1e-6 || (q.y - p.y).abs() > 1e-6) {
                    pts.push(p);
                }
            }
        }
        if let (Some(a), Some(b)) = (pts.first(), pts.last()) {
            if (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6 {
                pts.pop();
            }
        }
    }
    aic_math::Polygon2D::new(pts)
}

/// Thanh giằng (trên): front / back / extra rail sets, flat or on edge.
fn top_rails(cx: &mut Ctx, r: &crate::structure::TopRails, inner_w: f64, depth: f64, start: f64) {
    let (t, h, d) = (cx.v.thickness, cx.v.height, cx.v.depth);
    let rw = cx.v.rail_width;
    let size = |s: f64| if s > 0.0 { s } else { rw.min(depth / 2.0) };
    let step = |rs: &crate::structure::RailSet| if rs.horizontal { size(rs.size) } else { t };
    // `z_face` = front face of the rail.
    let put = |cx: &mut Ctx, key: String, name: String, rs: &crate::structure::RailSet, z_face: f64| {
        let s = size(rs.size);
        if rs.horizontal {
            cx.panel(key, name, PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [inner_w, s, t], [t, h - t, z_face], ROT_HORIZONTAL);
        } else {
            cx.panel(key, name, PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [inner_w, s, t], [t, h - s, z_face - t], [0.0; 3]);
        }
    };
    let f0 = d + r.front.offset;
    for i in 0..r.front.count {
        let key = if i == 0 { "c:rail_front".to_string() } else { format!("c:rail_front{}", i + 1) };
        let name = if i == 0 { "GiằngTrước".to_string() } else { format!("GiằngTrước_{}", i + 1) };
        put(cx, key, name, &r.front, f0 - i as f64 * step(&r.front));
    }
    let b0 = start + r.back.offset;
    for i in 0..r.back.count {
        let key = if i == 0 { "c:rail_back".to_string() } else { format!("c:rail_back{}", i + 1) };
        let name = if i == 0 { "GiằngSau".to_string() } else { format!("GiằngSau_{}", i + 1) };
        put(cx, key, name, &r.back, b0 + (i + 1) as f64 * step(&r.back));
    }
    // Extra rails, evenly spread in the free depth between both sets.
    let n = r.extra.count;
    if n > 0 {
        let front_end = f0 - r.front.count as f64 * step(&r.front);
        let back_end = b0 + r.back.count as f64 * step(&r.back);
        let se = step(&r.extra);
        let free = front_end - back_end - n as f64 * se;
        if free > 0.0 {
            let gap = free / (n + 1) as f64;
            for k in 0..n {
                let z_face = back_end + (k + 1) as f64 * (gap + se);
                put(cx, format!("c:rail_mid{}", k + 1), format!("GiằngGiữa_{}", k + 1), &r.extra, z_face);
            }
        }
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
        // Hàng lỗ hệ 32: kệ di động bắt vào lỗ gần nhất (không lấn sang tấm kề).
        let sr = &cx.cab.rules.shop;
        let mut starts = starts;
        if a == 1 && sr.pin_row == PinRow::Row32 && sr.pin_snap {
            let p = sr.pin_pitch.max(8.0);
            let base = sr.pin_start + sr.pin_below;
            for i in 0..s.panels.len() {
                if s.panels[i].kind != SplitKind::ShelfAdjustable {
                    continue;
                }
                let lo = if i == 0 { 1.0 } else { starts[i - 1] + s.panels[i - 1].thickness + 1.0 };
                let hi = if i + 1 < s.panels.len() { starts[i + 1] } else { len } - s.panels[i].thickness - 1.0;
                let snapped = base + ((starts[i] - base) / p).round() * p;
                let cand = [snapped, snapped - p, snapped + p];
                if let Some(v) = cand.into_iter().filter(|v| *v >= lo && *v <= hi).min_by(|x, y| (x - starts[i]).abs().total_cmp(&(y - starts[i]).abs())) {
                    starts[i] = v;
                }
            }
        }
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
                // Chia ảo: không có tấm, hai mặt cánh kề nhau chỉ cách một khe (t = 0).
                if i > 0 {
                    cb.nb[lo] = Neighbor { t: s.panels[i - 1].thickness, outer: false, part: part_idx[i - 1] };
                }
                if i < s.panels.len() {
                    cb.nb[hi] = Neighbor { t: s.panels[i].thickness, outer: false, part: part_idx[i] };
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

fn split_panel(cx: &mut Ctx, p: &SplitPanel, b: &ZBox, st: f64) -> Option<usize> {
    if p.kind.is_virtual() {
        return None;
    }
    let [x, y, z] = b.min;
    let [w, h, d] = b.size;
    let t = p.thickness;
    let n = cx.next(p.kind.base_name());
    let name = format!("{}_{:02}", p.kind.base_name(), n);
    let key = format!("p:{}", p.uid);
    let front = z + d;
    Some(match p.kind {
        SplitKind::ShelfAdjustable if p.tilt_deg[0].abs() >= 0.5 => {
            let sb = cx.v.shelf_setback.min(d / 2.0);
            let c = cx.cab.rules.shop.shelf_clear.clamp(0.0, 10.0);
            tilted_shelf(cx, p, key, name, PanelRole::Shelf, [x + c, y + st, front - sb], w - 2.0 * c, d - sb, t)
        }
        SplitKind::ShelfAdjustable => {
            let sb = cx.v.shelf_setback.min(d / 2.0);
            let sr = cx.cab.rules.shop.clone();
            let c = sr.shelf_clear.clamp(0.0, 10.0);
            let idx = cx.panel(key, name, PanelRole::Shelf, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w - 2.0 * c, d - sb, t], [x + c, y + st, front - sb], ROT_HORIZONTAL);
            cx.out.fittings.shelf_pins += 4;
            // Shelf pins (chốt tầng) in the side / divider on each side of the shelf:
            // 4 holes under the shelf, or the whole 32-mm row of the zone.
            let pin_y = y + st - sr.pin_below;
            let zs = [front - sb - sr.pin_edge, z + sr.pin_edge_back];
            let ys: Vec<f64> = match sr.pin_row {
                PinRow::AtShelf => vec![pin_y],
                PinRow::Row32 => sr.pin_grid(y, y + h),
            };
            for (nb, face) in [(b.nb[0], FaceSide::A), (b.nb[1], FaceSide::B)] {
                let Some(pi) = nb.part else { continue };
                if sr.pin_row == PinRow::Row32 && !cx.pin_rows.insert((pi, (y * 10.0).round() as i64, (h * 10.0).round() as i64)) {
                    continue;
                }
                let part = &cx.out.parts[pi];
                if part.rotation_deg != ROT_SIDE {
                    continue;
                }
                let (tz, ty) = (part.translation[2], part.translation[1]);
                let feats = ys
                    .iter()
                    .flat_map(|py| zs.iter().map(move |wz| (*py, *wz)))
                    .map(|(py, wz)| MachiningFeature::Drill(DrillFeature { x: tz - wz, y: py - ty, diameter: sr.pin_d, depth: sr.pin_depth, side: face, purpose: DrillPurpose::ShelfPin }))
                    .collect();
                cx.add_features(pi, feats);
            }
            idx
        }
        SplitKind::VirtualH | SplitKind::VirtualV => unreachable!(),
        SplitKind::ShelfFixed if p.tilt_deg[0].abs() >= 0.5 => tilted_shelf(cx, p, key, name, PanelRole::ShelfFixed, [x, y + st, front], w, d, t),
        SplitKind::ShelfFixed => cx.panel(key, name, PanelRole::ShelfFixed, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, d, t], [x, y + st, front], ROT_HORIZONTAL),
        SplitKind::Divider => match p.extent {
            // Vách lửng: cao `extent` từ đáy khoang (dương) hoặc từ nóc (âm).
            Some(e) if e.abs() >= 1.0 && e.abs() < h => {
                let hh = e.abs();
                let yy = if e > 0.0 { y } else { y + h - hh };
                cx.panel(key, name, PanelRole::Divider, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, hh, t], [x + st, yy, front], ROT_SIDE)
            }
            _ => cx.panel(key, name, PanelRole::Divider, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, h, t], [x + st, y, front], ROT_SIDE),
        },
        SplitKind::BackSub => cx.panel(key, name, PanelRole::BackSub, MaterialSlot::Back, GrainDirection::AlongHeight, [w, h, t], [x, y, z + st], [0.0; 3]),
    })
}

/// Kệ nghiêng (kệ giày): xoay quanh cạnh sau, mép trước thấp hơn; hình chiếu ngang vẫn bằng `depth`.
/// Nghiêng ≥ 5° có thanh chặn gót cao 30 ở mép trước. Không khoan chốt tầng (đỡ bằng ke / thanh).
#[allow(clippy::too_many_arguments)]
fn tilted_shelf(cx: &mut Ctx, p: &SplitPanel, key: String, name: String, role: PanelRole, at: [f64; 3], w: f64, depth: f64, t: f64) -> usize {
    let th = p.tilt_deg[0].clamp(-45.0, 45.0);
    let r = th.to_radians();
    let len = depth / r.cos();
    let [x, y_back, zf] = at;
    let y_front = y_back - len * r.sin();
    let idx = cx.panel(key.clone(), name, role, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, len, t], [x, y_front, zf], [-90.0 + th, 0.0, 0.0]);
    if th.abs() >= 5.0 {
        let k = cx.next("ThanhChặnGót");
        cx.panel(format!("{key}:heel"), format!("ThanhChặnGót_{k:02}"), PanelRole::Rail, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, 30.0, t], [x, y_front, zf - t], [0.0; 3]);
    }
    idx
}

fn hinge_cups(sr: &ShopRules, w: f64, h: f64, side: HingeSide) -> Vec<MachiningFeature> {
    let along = if matches!(side, HingeSide::Left | HingeSide::Right) { h } else { w };
    let edge = sr.hinge_edge;
    sr.hinge_positions(along)
        .into_iter()
        .map(|s| {
            let (x, y) = match side {
                HingeSide::Left => (edge, s),
                HingeSide::Right => (w - edge, s),
                HingeSide::Top => (s, h - edge),
                HingeSide::Bottom => (s, edge),
            };
            MachiningFeature::Drill(DrillFeature { x, y, diameter: sr.cup_d, depth: sr.cup_depth, side: FaceSide::B, purpose: DrillPurpose::HingeCup })
        })
        .collect()
}

/// Front rectangle (x0, y0, x1, y1) and front Z for a zone.
/// Front rectangle of an opening; `gaps` = reveal per side [left, right, bottom, top].
fn front_rect(b: &ZBox, mount: Mount, gaps: [f64; 4], depth: f64, thickness: f64) -> (f64, f64, f64, f64, f64) {
    let [x, y, _] = b.min;
    let [w, h, _] = b.size;
    match mount {
        Mount::Overlay => {
            let ext = |n: Neighbor, gap: f64| if n.outer { n.t - gap } else { n.t / 2.0 - gap / 2.0 };
            (x - ext(b.nb[0], gaps[0]), y - ext(b.nb[2], gaps[2]), x + w + ext(b.nb[1], gaps[1]), y + h + ext(b.nb[3], gaps[3]), depth)
        }
        Mount::Inset => (x + gaps[0], y + gaps[2], x + w - gaps[1], y + h - gaps[3], depth - thickness),
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

/// Cánh lùa theo hệ ray: n cánh chồng `overlap`, cánh xen kẽ các ray, trừ cao theo bánh xe;
/// khung nhôm (4 thanh + nẹp ngang) và kính / gương là phụ kiện (không vào xếp tấm ván).
fn sliding_doors(cx: &mut Ctx, spec: &DoorSpec, rect: [f64; 5], t: f64) {
    let [x0, y0, x1, y1, z] = rect;
    let sp = spec.sliding.clone();
    let n = spec.cols.max(2);
    let overlap = sp.as_ref().map(|s| s.overlap).unwrap_or(cx.cab.rules.shop.slide_overlap).max(0.0);
    let tracks = sp.as_ref().map(|s| s.tracks.clamp(2, 3)).unwrap_or(2);
    let (dt, db) = sp.as_ref().map(|s| (s.deduct_top.max(0.0), s.deduct_bottom.max(0.0))).unwrap_or((0.0, 0.0));
    let total = x1 - x0;
    let lw = (total + (n - 1) as f64 * overlap) / n as f64;
    let hgt = (y1 - y0 - dt - db).max(50.0);
    let y = y0 + db;
    let pw = sp.as_ref().map(|s| s.profile_w()).unwrap_or(0.0);
    let infill = sp.as_ref().map(|s| s.infill).unwrap_or_default();
    let rails = sp.as_ref().map(|s| s.rails_h.min(3)).unwrap_or(0);
    for i in 0..n {
        let k = cx.next("CửaLùa");
        let lx = x0 + i as f64 * (lw - overlap);
        // Ray ngoài cùng (trước) cho cánh chẵn; 3 ray: xoay vòng.
        let track = (i % tracks) as f64;
        let lz = z + (tracks as f64 - 1.0 - track) * (t + 4.0);
        let key = format!("d:{}:0:{i}", spec.uid);
        if pw <= 0.0 && infill == Infill::Board {
            cx.panel(key, format!("CửaLùa_{k:02}"), PanelRole::Door, MaterialSlot::Front, GrainDirection::AlongHeight, [lw, hgt, t], [lx, y, lz], [0.0; 3]);
            continue;
        }
        // Khung nhôm: 2 thanh đứng + 2 thanh ngang + nẹp ngang; ô nhét chia đều theo nẹp.
        let bars = [
            ("l", [pw, hgt, t], [lx, y, lz]),
            ("r", [pw, hgt, t], [lx + lw - pw, y, lz]),
            ("b", [lw - 2.0 * pw, pw, t], [lx + pw, y, lz]),
            ("t", [lw - 2.0 * pw, pw, t], [lx + pw, y + hgt - pw, lz]),
        ];
        if pw > 0.0 {
            for (s, size, at) in bars {
                cx.hardware(format!("{key}:alu_{s}"), format!("KhungNhôm_{k:02}{s}"), HardwareKind::Profile, "ALU-SLIDE", size, at);
            }
            cx.out.fittings.alu_profile_mm += 2.0 * (lw + hgt) + rails as f64 * (lw - 2.0 * pw);
        }
        let cells = rails + 1;
        let ih = (hgt - 2.0 * pw - rails as f64 * pw) / cells as f64;
        for c in 0..cells {
            let cy = y + pw + c as f64 * (ih + pw);
            if c > 0 && pw > 0.0 {
                cx.hardware(format!("{key}:alu_m{c}"), format!("NẹpNgang_{k:02}_{c}"), HardwareKind::Profile, "ALU-SLIDE", [lw - 2.0 * pw, pw, t], [lx + pw, cy - pw, lz]);
            }
            let size = [lw - 2.0 * pw, ih, if infill == Infill::Board { t.min(10.0) } else { 5.0 }];
            let at = [lx + pw, cy, lz + (t - size[2]) / 2.0];
            match infill {
                Infill::Board => {
                    cx.panel(format!("{key}:in{c}"), format!("ÔNhétCửaLùa_{k:02}_{}", c + 1), PanelRole::Door, MaterialSlot::Front, GrainDirection::AlongHeight, size, at, [0.0; 3]);
                }
                Infill::Glass | Infill::Mirror => {
                    let (name, code) = if infill == Infill::Glass { ("Kính", "GLASS-5") } else { ("Gương", "MIRROR-5") };
                    cx.hardware(format!("{key}:in{c}"), format!("{name}CửaLùa_{k:02}_{}", c + 1), HardwareKind::Glass, code, size, at);
                    cx.out.fittings.glass_mm2 += size[0] * size[1];
                }
            }
        }
    }
    let depth = tracks as f64 * (t + 4.0);
    cx.hardware(format!("t:{}:top", spec.uid), "RayLùaTrên".into(), HardwareKind::Rail, "TRACK-SLIDE", [total, 20.0, depth], [x0, y1, z]);
    cx.hardware(format!("t:{}:bot", spec.uid), "RayLùaDưới".into(), HardwareKind::Rail, "TRACK-SLIDE", [total, 10.0, depth], [x0, y0 - 10.0, z]);
    cx.out.fittings.sliding_tracks += 1;
}

fn doors(cx: &mut Ctx, spec: &DoorSpec, b: &ZBox) {
    let gap = spec.gap.unwrap_or(cx.v.door_gap);
    let t = spec.thickness.unwrap_or(cx.v.door_thickness);
    let d = cx.v.depth;
    let (mut x0, y0, mut x1, mut y1, z) = front_rect(b, spec.mount, spec.side_gaps.unwrap_or([gap; 4]), d, t);
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
        sliding_doors(cx, spec, [x0, y0, x1, y1, z], t);
        return;
    }
    // Hinged doors: grid of leaves.
    let _ = &mut x0;
    let _ = &mut x1;
    let cw = ((x1 - x0) - (cols - 1) as f64 * gap) / cols as f64;
    let ch = ((y1 - y0) - (rows - 1) as f64 * gap) / rows as f64;
    let base = if spec.fixed { "TấmMù" } else if spec.kind == DoorKind::Double { "CửaĐôi" } else { "CửaĐơn" };
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
            if spec.fixed {
                // Tấm mù cố định: bắt vít vào hồi / vách, không bản lề, không tay nắm.
                continue;
            }
            let sr = cx.cab.rules.shop.clone();
            cx.add_features(idx, hinge_cups(&sr, cw, ch, hinge));
            if let PartKind::Panel { hinge: hs, .. } = &mut cx.out.parts[idx].kind {
                *hs = Some(to_edge(hinge));
            }
            cx.out.fittings.hinges += sr.hinge_count(if matches!(hinge, HingeSide::Left | HingeSide::Right) { ch } else { cw });
            // Đế bản lề: 2 lỗ Ø5 trên hồi / vách phía lề (cách mép trước 37, bước 32).
            if sr.hinge_plate && matches!(hinge, HingeSide::Left | HingeSide::Right) {
                let (nb, face) = if hinge == HingeSide::Left { (b.nb[0], FaceSide::A) } else { (b.nb[1], FaceSide::B) };
                if let Some(pi) = nb.part {
                    let part = &cx.out.parts[pi];
                    if part.rotation_deg == ROT_SIDE {
                        let (tz, ty) = (part.translation[2], part.translation[1]);
                        let feats = sr
                            .hinge_positions(ch)
                            .into_iter()
                            .flat_map(|s| [37.0, 69.0].map(|e| (dy + s, d - e)))
                            .map(|(wy, wz)| MachiningFeature::Drill(DrillFeature { x: tz - wz, y: wy - ty, diameter: 5.0, depth: 12.0, side: face, purpose: DrillPurpose::HingeScrew }))
                            .collect();
                        cx.add_features(pi, feats);
                    }
                }
            }
            if cx.cab.handles {
                match sr.handle_type {
                    HandleType::None => continue,
                    HandleType::PushOpen => {
                        cx.out.fittings.push_latches += if ch > 1200.0 { 2 } else { 1 };
                        continue;
                    }
                    _ => {}
                }
                // Handle on the opening side; height by the shop rule (bếp dưới → trên, bếp trên → dưới).
                let knob = sr.handle_type == HandleType::Knob;
                let l = if knob { 30.0 } else { sr.handle_len.clamp(10.0, ch.max(10.0)) };
                let e = sr.handle_edge;
                let pos = match sr.handle_pos {
                    HandlePos::Auto => match cx.cab.kind {
                        CabinetKind::Base | CabinetKind::Drawer => HandlePos::Top,
                        CabinetKind::Wall => HandlePos::Bottom,
                        _ => HandlePos::Center,
                    },
                    p => p,
                };
                let vy = match pos {
                    HandlePos::Top => dy + ch - sr.handle_from_end - l,
                    HandlePos::Bottom => dy + sr.handle_from_end,
                    _ => dy + ch / 2.0 - l / 2.0,
                };
                let vertical = matches!(hinge, HingeSide::Left | HingeSide::Right);
                let (hx, hy, size) = match hinge {
                    HingeSide::Left => (dx + cw - e - 6.0, vy, [12.0, l, 30.0]),
                    HingeSide::Right => (dx + e - 6.0, vy, [12.0, l, 30.0]),
                    HingeSide::Bottom => (dx + cw / 2.0 - l / 2.0, dy + ch - e - 6.0, [l, 12.0, 30.0]),
                    HingeSide::Top => (dx + cw / 2.0 - l / 2.0, dy + e - 6.0, [l, 12.0, 30.0]),
                };
                let size = if knob { [30.0, 30.0, 30.0] } else { size };
                let k = cx.next("TayNắm");
                let code = if knob { "HDL-KNOB".to_string() } else { format!("HDL-BAR-{}", l.round()) };
                cx.hardware(format!("h:{}:{r}:{c}", spec.uid), format!("TayNắm_{k:02}"), HardwareKind::Handle, &code, size, [hx, hy, z + t]);
                if sr.handle_drill {
                    // Lỗ bắt tay nắm xuyên cánh (tọa độ local của cánh).
                    let (cx0, cy0) = if vertical { (hx + 6.0 - dx, hy + l / 2.0 - dy) } else { (hx + l / 2.0 - dx, hy + 6.0 - dy) };
                    let half = if knob { 0.0 } else { sr.handle_pitch.min(l) / 2.0 };
                    let pts: Vec<(f64, f64)> = if knob { vec![(cx0, cy0)] } else if vertical { vec![(cx0, cy0 - half), (cx0, cy0 + half)] } else { vec![(cx0 - half, cy0), (cx0 + half, cy0)] };
                    let feats = pts.into_iter().map(|(x, y)| MachiningFeature::Drill(DrillFeature { x, y, diameter: 5.0, depth: t, side: FaceSide::A, purpose: DrillPurpose::Handle })).collect();
                    cx.add_features(idx, feats);
                }
                if knob {
                    cx.out.fittings.knobs += 1;
                } else {
                    cx.out.fittings.handles += 1;
                }
            }
        }
    }
}

fn drawers(cx: &mut Ctx, spec: &DrawerSpec, b: &ZBox) {
    let d = cx.v.depth;
    let ft = spec.face_thickness.unwrap_or(cx.v.door_thickness);
    let gap = spec.gap;
    // Ngăn kéo trong: mặt lọt lòng, lùi sau cánh.
    let mount = if spec.inner { Mount::Inset } else { spec.mount };
    let side_gap = if mount == Mount::Inset { spec.side_gap } else { cx.v.door_gap };
    let (x0, y0, x1, y1, z) = front_rect(b, mount, [side_gap; 4], d, ft);
    let z = if spec.inner { z - spec.inner_setback.max(0.0) } else { z };
    let n = spec.count.max(1);
    let cols = spec.cols.max(1);
    let fw = ((x1 - x0) - (cols - 1) as f64 * gap) / cols as f64;
    // Front heights: LOCK / PERCENT / AUTO bays (bottom → top), else equal.
    let gaps = vec![gap; n as usize - 1];
    let heights: Vec<f64> = if spec.heights.len() == n as usize {
        let (h, ok) = solve_bays(y1 - y0, &gaps, &spec.heights);
        if !ok {
            cx.out.problems.push(spec.uid);
        }
        h
    } else {
        vec![((y1 - y0) - (n - 1) as f64 * gap) / n as f64; n as usize]
    };
    let starts: Vec<f64> = heights.iter().scan(y0, |at, h| {
        let s = *at;
        *at += h + gap;
        Some(s)
    }).collect();
    let usable = (y1 - y0) - (n - 1) as f64 * gap;
    for i in 0..n as usize {
        let bay = spec.heights.get(i).filter(|_| spec.heights.len() == n as usize);
        cx.out.front_bays.push(FrontBay {
            uid: spec.uid,
            index: i,
            start: starts[i],
            size: heights[i],
            x0,
            x1,
            z: z + ft,
            mode: bay.map(|b| b.mode),
            value: bay.map(|b| b.value).unwrap_or(0.0),
            usable,
        });
    }
    let [zx, zy, _] = b.min;
    let [zw, zh, zd] = b.size;
    let cw = (zw - (cols - 1) as f64 * cx.v.thickness) / cols as f64;
    let sr = cx.cab.rules.shop.clone();
    // Ngăn kéo trong lùi sau cánh: ray ngắn hơn phần lùi.
    let avail = zd - sr.slide_margin - if spec.inner { spec.inner_setback.max(0.0) + ft } else { 0.0 };
    let slide = STD_SLIDES.iter().copied().filter(|l| *l <= avail).fold(STD_SLIDES[0], f64::max);
    let zf = if spec.inner { z } else if spec.mount == Mount::Overlay { d } else { d - ft };
    for c in 0..cols {
        for i in 0..n {
            cx.drawer_sets += 1;
            let set = cx.drawer_sets;
            let fx = x0 + c as f64 * (fw + gap);
            let fh = heights[i as usize];
            let fy = starts[i as usize];
            let key = |part: &str| format!("w:{}:{c}:{i}:{part}", spec.uid);
            let face_name = if spec.false_front { "MặtGiả" } else if spec.inner { "MặtNgănTrong" } else { "MặtNgăn" };
            let front_idx = cx.panel(key("front"), format!("{face_name} [Bộ {set}]"), PanelRole::DrawerFront, MaterialSlot::Front, GrainDirection::AlongWidth, [fw, fh, ft], [fx, fy, z], [0.0; 3]);
            if spec.false_front {
                // Mặt giả: bắt cố định, không tay nắm / hộc / ray.
                continue;
            }
            if cx.cab.handles && !spec.inner {
                match sr.handle_type {
                    HandleType::None => {}
                    HandleType::PushOpen => cx.out.fittings.push_latches += 1,
                    t => {
                        let knob = t == HandleType::Knob;
                        let l = if knob { 30.0 } else { sr.handle_len.clamp(10.0, fw.max(10.0)) };
                        let code = if knob { "HDL-KNOB".to_string() } else { format!("HDL-BAR-{}", l.round()) };
                        let size = if knob { [30.0, 30.0, 30.0] } else { [l, 12.0, 30.0] };
                        cx.hardware(key("handle"), format!("TayNắm [Bộ {set}]"), HardwareKind::Handle, &code, size, [fx + fw / 2.0 - l / 2.0, fy + fh / 2.0 - 6.0, z + ft]);
                        if sr.handle_drill {
                            let (hx, hy) = (fw / 2.0, fh / 2.0);
                            let half = if knob { 0.0 } else { sr.handle_pitch.min(l) / 2.0 };
                            let pts: Vec<f64> = if knob { vec![hx] } else { vec![hx - half, hx + half] };
                            let feats = pts.into_iter().map(|x| MachiningFeature::Drill(DrillFeature { x, y: hy, diameter: 5.0, depth: ft, side: FaceSide::A, purpose: DrillPurpose::Handle })).collect();
                            cx.add_features(front_idx, feats);
                        }
                        if knob {
                            cx.out.fittings.knobs += 1;
                        } else {
                            cx.out.fittings.handles += 1;
                        }
                    }
                }
            }
            let fitting = match sr.slide_type {
                SlideType::Ball => &mut cx.out.fittings.slides,
                SlideType::Undermount => &mut cx.out.fittings.undermount,
                SlideType::Tandem => &mut cx.out.fittings.tandem,
            };
            *fitting.entry(slide as u32).or_default() += 1;
            if !spec.with_box {
                continue;
            }
            // Drawer box inside the opening of this front.
            let bxz = zx + c as f64 * (cw + cx.v.thickness);
            // Box cell follows its front (mapped onto the opening height).
            let k = if y1 > y0 { zh / (y1 - y0) } else { 1.0 };
            let cell_y0 = zy + (fy - y0) * k;
            let cell_h = (fh + gap) * k;
            let bt = spec.box_thickness;
            let bb = spec.bottom_thickness;
            let hb = (cell_h - sr.box_top_gap).clamp(sr.box_min.max(10.0), sr.box_max.max(sr.box_min.max(10.0)));
            let by = cell_y0 + sr.box_bottom_gap;
            let (hw_name, hw_code) = match sr.slide_type {
                SlideType::Ball => ("RayBi", "RAYBI"),
                SlideType::Undermount => ("RayÂm", "RAYAM"),
                SlideType::Tandem => ("Tandem", "TANDEM"),
            };
            let label = format!("{hw_name} {} [Bộ {set}]", slide as u32);
            let code = format!("{hw_code}-{}", slide as u32);
            match sr.slide_type {
                SlideType::Tandem => {
                    // Thành hộc kim loại: chỉ cắt đáy (LW − 75) + hậu hộc (LW − 87).
                    let len = slide - 10.0;
                    cx.panel(key("back"), format!("HậuHộc [Bộ {set}]"), PanelRole::DrawerBack, MaterialSlot::Carcass, GrainDirection::AlongWidth, [(cw - 87.0).max(10.0), (hb - 30.0).max(10.0), bt], [bxz + 43.5, by + bb, zf - len], [0.0; 3]);
                    cx.panel(key("bottom"), format!("ĐáyNgănKéo [Bộ {set}]"), PanelRole::DrawerBottom, MaterialSlot::Back, GrainDirection::AlongWidth, [(cw - 75.0).max(10.0), len - bt, bb], [bxz + 37.5, by, zf - bt], ROT_HORIZONTAL);
                    cx.hardware(key("slideL"), label.clone(), HardwareKind::Slide, &code, [18.0, hb, len], [bxz + 5.0, by, zf - len]);
                    cx.hardware(key("slideR"), label, HardwareKind::Slide, &code, [18.0, hb, len], [bxz + cw - 23.0, by, zf - len]);
                }
                st => {
                    let (sc, len, raise) = if st == SlideType::Undermount { (5.0, slide - 10.0, 12.0) } else { (spec.slide_clearance, slide, 0.0) };
                    let bw = (cw - 2.0 * sc).max(2.0 * bt + 10.0);
                    let bx = bxz + sc;
                    cx.panel(key("sideL"), format!("ThànhTrái [Bộ {set}]"), PanelRole::DrawerSide, MaterialSlot::Carcass, GrainDirection::AlongWidth, [len, hb, bt], [bx, by, zf], ROT_SIDE);
                    cx.panel(key("sideR"), format!("ThànhPhải [Bộ {set}]"), PanelRole::DrawerSide, MaterialSlot::Carcass, GrainDirection::AlongWidth, [len, hb, bt], [bx + bw - bt, by, zf], ROT_SIDE);
                    cx.panel(key("back"), format!("HậuHộc [Bộ {set}]"), PanelRole::DrawerBack, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bw - 2.0 * bt, hb - bb - raise, bt], [bx + bt, by + bb + raise, zf - len], [0.0; 3]);
                    cx.panel(key("front_inner"), format!("ĐầuHộc [Bộ {set}]"), PanelRole::DrawerBack, MaterialSlot::Carcass, GrainDirection::AlongWidth, [bw - 2.0 * bt, hb - bb - raise, bt], [bx + bt, by + bb + raise, zf - bt], [0.0; 3]);
                    cx.panel(key("bottom"), format!("ĐáyNgănKéo [Bộ {set}]"), PanelRole::DrawerBottom, MaterialSlot::Back, GrainDirection::AlongWidth, [bw - 2.0 * bt, len, bb], [bx + bt, by + raise, zf], ROT_HORIZONTAL);
                    if st == SlideType::Undermount {
                        cx.hardware(key("slideL"), label.clone(), HardwareKind::Slide, &code, [bt, 20.0, slide], [bx, by - 20.0, zf - slide]);
                        cx.hardware(key("slideR"), label, HardwareKind::Slide, &code, [bt, 20.0, slide], [bx + bw - bt, by - 20.0, zf - slide]);
                    } else {
                        cx.hardware(key("slideL"), label.clone(), HardwareKind::Slide, &code, [sc - 0.5, 45.0, slide], [bxz, by + hb / 2.0 - 22.5, zf - slide]);
                        cx.hardware(key("slideR"), label, HardwareKind::Slide, &code, [sc - 0.5, 45.0, slide], [bx + bw + 0.5, by + hb / 2.0 - 22.5, zf - slide]);
                    }
                }
            }
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
        let (oext, omove) = offsets_to_local(p.rotation_deg, &m.offsets);
        for k in 0..3 {
            p.translation[k] += omove[k];
        }
        if matches!(p.kind, PartKind::Panel { .. }) {
            stretch(p, [m.extend[0] + oext[0], m.extend[1] + oext[1], m.extend[2] + oext[2], m.extend[3] + oext[3]]);
        }
        if let PartKind::Panel { features, .. } = &mut p.kind {
            if let Some(tk) = m.thickness {
                p.size[2] = pos(tk);
            }
            features.extend(m.features.iter().cloned());
            let [w, h, t] = p.size;
            features.extend(m.param_features.iter().map(|f| f.resolve(w, h, t)));
        }
    }
    solve_anchors(out, mods);
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
            side_gaps: None,
            stop: StopRailSpec::default(),
            fixed: false,
            sliding: None,
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
        let p = |lock, value| SplitPanel { uid: 1, kind: SplitKind::ShelfAdjustable, thickness: 17.2, lock, value, tilt_deg: [0.0; 2], extent: None };
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
