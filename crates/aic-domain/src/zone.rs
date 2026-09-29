//! Zone model: a cabinet's interior is a tree of *zones* (clear openings).
//! Adding a shelf / divider / sub-back splits a zone; doors, drawers and
//! accessories (rails) attach to a zone. Everything is resolved to sizes by
//! [`crate::layout`] from the cabinet's current parameters, so resizing a
//! cabinet re-flows every part.

use serde::{Deserialize, Serialize};

pub type Uid = u32;

/// Kind of a zone-splitting panel. The kind decides the split axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SplitKind {
    /// Kệ di động: shelf on pins, set back from the front.
    ShelfAdjustable,
    /// Kệ cố định: fixed shelf, full depth, jointed to the sides.
    ShelfFixed,
    /// Hông giữa: vertical divider.
    Divider,
    /// Hậu phụ: sub-back parallel to the back.
    BackSub,
}

impl SplitKind {
    /// 0 = X (splits width), 1 = Y (splits height), 2 = Z (splits depth).
    pub fn axis(&self) -> usize {
        match self {
            SplitKind::ShelfAdjustable | SplitKind::ShelfFixed => 1,
            SplitKind::Divider => 0,
            SplitKind::BackSub => 2,
        }
    }

    pub fn base_name(&self) -> &'static str {
        match self {
            SplitKind::ShelfAdjustable => "KệDiĐộng",
            SplitKind::ShelfFixed => "KệCốĐịnh",
            SplitKind::Divider => "HôngGiữa",
            SplitKind::BackSub => "HậuPhụ",
        }
    }
}

/// Which of the three position values is locked; the other two follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Lock {
    /// Evenly distributed with the other `Even` panels of the same gap.
    #[default]
    Even,
    /// Ratio (0..1) of the free length `L - t`.
    Ratio,
    /// Clear distance (mm) from the start of the zone (bottom / left / back).
    FromStart,
    /// Clear distance (mm) from the end of the zone (top / right / front).
    FromEnd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitPanel {
    pub uid: Uid,
    pub kind: SplitKind,
    pub thickness: f64,
    pub lock: Lock,
    /// Value of the locked quantity (ratio 0..1 or mm). Ignored for `Even`.
    #[serde(default)]
    pub value: f64,
    /// Tilt in degrees (front-back, side-side); presentation of slanted shelves.
    #[serde(default)]
    pub tilt_deg: [f64; 2],
}

/// Sizing mode of one bay (khoang) of a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BayMode {
    /// Takes an equal share of what LOCK / PERCENT bays leave.
    #[default]
    Auto,
    /// Fixed clear size (mm).
    Lock,
    /// Percent (0..100) of the usable length (zone length minus split panels).
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Bay {
    pub mode: BayMode,
    #[serde(default)]
    pub value: f64,
}

impl Bay {
    pub fn auto() -> Self {
        Bay { mode: BayMode::Auto, value: 0.0 }
    }
    pub fn lock(mm: f64) -> Self {
        Bay { mode: BayMode::Lock, value: mm }
    }
    pub fn percent(p: f64) -> Self {
        Bay { mode: BayMode::Percent, value: p }
    }
}

/// Clear sizes of the bays of a split of length `len` with panels of thickness `t`.
/// LOCK bays keep their mm, PERCENT bays their share of the usable length, AUTO bays
/// share the rest equally. Without AUTO bays the rest goes to the PERCENT bays
/// (proportionally), else to the last bay. `ok` is false when a bay would be
/// smaller than 1 mm or an all-LOCK split cannot absorb the difference.
pub fn solve_bays(len: f64, t: &[f64], bays: &[Bay]) -> (Vec<f64>, bool) {
    let usable = len - t.iter().sum::<f64>();
    let mut size: Vec<f64> = bays
        .iter()
        .map(|b| match b.mode {
            BayMode::Lock => b.value.max(0.0),
            BayMode::Percent => b.value.max(0.0) / 100.0 * usable,
            BayMode::Auto => 0.0,
        })
        .collect();
    let rest = usable - size.iter().sum::<f64>();
    let autos: Vec<usize> = (0..bays.len()).filter(|&i| bays[i].mode == BayMode::Auto).collect();
    let pcts: Vec<usize> = (0..bays.len()).filter(|&i| bays[i].mode == BayMode::Percent).collect();
    let mut ok = true;
    if !autos.is_empty() {
        for &i in &autos {
            size[i] = rest / autos.len() as f64;
        }
    } else if !pcts.is_empty() {
        let psum: f64 = pcts.iter().map(|&i| size[i]).sum();
        for &i in &pcts {
            size[i] += if psum > 0.0 { rest * size[i] / psum } else { rest / pcts.len() as f64 };
        }
    } else if let Some(last) = size.last_mut() {
        *last += rest;
        ok = rest.abs() < 0.01;
    }
    if size.iter().any(|s| *s < 1.0) {
        ok = false;
    }
    (size, ok)
}

/// Move the divider after bay `i` so bay `i` becomes `before` mm.
/// Only the two adjacent bays change: the non-AUTO ones are rewritten (an
/// AUTO neighbour absorbs the change); two AUTO neighbours → the first is locked.
pub fn move_between(bays: &mut [Bay], i: usize, sizes: &[f64], before: f64) -> Result<(), String> {
    if i + 1 >= bays.len() || sizes.len() != bays.len() {
        return Err("no such divider".into());
    }
    let delta = before - sizes[i];
    if sizes[i] + delta < 1.0 || sizes[i + 1] - delta < 1.0 {
        return Err("bay would be smaller than 1 mm".into());
    }
    let usable: f64 = sizes.iter().sum();
    let (a, b) = (bays[i].mode, bays[i + 1].mode);
    let mut set = |k: usize, d: f64| {
        let bay = &mut bays[k];
        match bay.mode {
            BayMode::Lock => bay.value = sizes[k] + d,
            BayMode::Percent => bay.value = (sizes[k] + d) / usable * 100.0,
            BayMode::Auto => *bay = Bay::lock(sizes[k] + d),
        }
    };
    match (a == BayMode::Auto, b == BayMode::Auto) {
        (true, true) => set(i, delta),
        (true, false) => set(i + 1, -delta),
        (false, true) => set(i, delta),
        (false, false) => {
            set(i, delta);
            set(i + 1, -delta);
        }
    }
    Ok(())
}

/// Change one bay's mode and/or value (value: mm for LOCK, % for PERCENT).
pub fn set_bay_in(bays: &mut [Bay], k: usize, sizes: &[f64], mode: Option<BayMode>, value: Option<f64>) -> Result<(), String> {
    if k >= bays.len() || sizes.len() != bays.len() {
        return Err("no such bay".into());
    }
    let usable: f64 = sizes.iter().sum();
    let mode = mode.unwrap_or(bays[k].mode);
    let value = match (mode, value) {
        (BayMode::Auto, _) => 0.0,
        (_, Some(v)) if v.is_finite() && v >= 0.0 => v,
        (_, Some(_)) => return Err("value must be ≥ 0".into()),
        (BayMode::Lock, None) => sizes[k],
        (BayMode::Percent, None) => if usable > 0.0 { sizes[k] / usable * 100.0 } else { 0.0 },
    };
    let old = bays[k];
    bays[k] = Bay { mode, value };
    // A typed size with no AUTO bay elsewhere: the nearest neighbour absorbs the
    // difference and keeps its own mode (LOCK mm / PERCENT %).
    let flexible_other = (0..bays.len()).any(|j| j != k && bays[j].mode == BayMode::Auto);
    if mode != BayMode::Auto && !flexible_other && bays.len() > 1 {
        let new_k = if mode == BayMode::Lock { value } else { value / 100.0 * usable };
        let delta = new_k - sizes[k];
        let j = if k + 1 < bays.len() { k + 1 } else { k - 1 };
        let nj = sizes[j] - delta;
        if nj < 1.0 || new_k < 1.0 {
            bays[k] = old;
            return Err("bay would be smaller than 1 mm".into());
        }
        let bj = &mut bays[j];
        bj.value = match bj.mode {
            BayMode::Percent => nj / usable * 100.0,
            _ => nj,
        };
        return Ok(());
    }
    // Locking a bay by value: an AUTO/PERCENT bay must remain to absorb resizes.
    if !bays.iter().any(|b| b.mode != BayMode::Lock) {
        // Let the nearest neighbour absorb (ties: the larger one), so earlier locks stay.
        let other = (0..bays.len())
            .filter(|&j| j != k)
            .min_by(|&a, &b| (a as i64 - k as i64).abs().cmp(&(b as i64 - k as i64).abs()).then(sizes[b].total_cmp(&sizes[a])));
        match other {
            Some(j) => bays[j] = Bay::auto(),
            None => {
                bays[k] = old;
                return Err("at least one bay must be AUTO or PERCENT".into());
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub axis: usize,
    pub panels: Vec<SplitPanel>,
    /// `panels.len() + 1` sub-zones, in order along the axis.
    pub children: Vec<Zone>,
    /// Bay sizing (`panels.len() + 1` entries). Empty = legacy per-panel locks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bays: Vec<Bay>,
}

impl Split {
    pub fn has_bays(&self) -> bool {
        self.bays.len() == self.panels.len() + 1
    }

    /// Switch to bay sizing from the current clear sizes, keeping the intent of the
    /// legacy locks (ratio → percent, from start/end → lock + auto, even → auto).
    pub fn adopt_bays(&mut self, sizes: &[f64]) {
        if self.has_bays() || sizes.len() != self.panels.len() + 1 {
            return;
        }
        let usable: f64 = sizes.iter().sum();
        let pct = |s: f64| if usable > 0.0 { s / usable * 100.0 } else { 0.0 };
        self.bays = if self.panels.iter().all(|p| p.lock == Lock::Even) {
            vec![Bay::auto(); sizes.len()]
        } else if self.panels.len() == 1 {
            match self.panels[0].lock {
                Lock::Ratio => vec![Bay::percent(pct(sizes[0])), Bay::percent(pct(sizes[1]))],
                Lock::FromStart => vec![Bay::lock(sizes[0]), Bay::auto()],
                Lock::FromEnd => vec![Bay::auto(), Bay::lock(sizes[1])],
                Lock::Even => vec![Bay::auto(); 2],
            }
        } else {
            let big = (0..sizes.len()).max_by(|&a, &b| sizes[a].total_cmp(&sizes[b])).unwrap_or(0);
            sizes.iter().enumerate().map(|(i, s)| if i == big { Bay::auto() } else { Bay::lock(*s) }).collect()
        };
    }

    /// Kéo vách / kệ: move panel `i` so the bay before it becomes `before` mm (see [`move_between`]).
    pub fn move_panel(&mut self, i: usize, sizes: &[f64], before: f64) -> Result<(), String> {
        if !self.has_bays() || i >= self.panels.len() {
            return Err("split has no bay sizing".into());
        }
        move_between(&mut self.bays, i, sizes, before)
    }

    /// Change one bay's mode and/or value (see [`set_bay_in`]).
    pub fn set_bay(&mut self, k: usize, sizes: &[f64], mode: Option<BayMode>, value: Option<f64>) -> Result<(), String> {
        if !self.has_bays() {
            return Err("no such bay".into());
        }
        set_bay_in(&mut self.bays, k, sizes, mode, value)
    }

    /// Divide equally (Equal Divide): every bay AUTO.
    pub fn equalize(&mut self) {
        self.bays = vec![Bay::auto(); self.panels.len() + 1];
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Mount {
    /// Phủ bì: front covers the carcass edges.
    #[default]
    Overlay,
    /// Lọt lòng: front sits inside the opening.
    Inset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DoorKind {
    #[default]
    Single,
    Double,
    Sliding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HingeSide {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

/// Door stop rail (thanh chặn cánh) at the top of the opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StopRail {
    #[default]
    None,
    /// Chữ L: one vertical + one horizontal strip.
    LShape,
    /// Thẳng: one vertical strip.
    Straight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopRailSpec {
    pub kind: StopRail,
    /// Cao vùng (height of the strip).
    pub height: f64,
    /// Cửa phủ lên: how far the doors cover the rail.
    pub cover_up: f64,
    /// Sâu chân: depth of the horizontal leg.
    pub leg_depth: f64,
    /// Lùi thanh: setback from the cabinet front.
    pub setback: f64,
}

impl Default for StopRailSpec {
    fn default() -> Self {
        Self { kind: StopRail::None, height: 75.0, cover_up: 25.0, leg_depth: 40.0, setback: 20.2 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoorSpec {
    pub uid: Uid,
    pub kind: DoorKind,
    /// Doors across × down.
    pub cols: u32,
    pub rows: u32,
    pub mount: Mount,
    pub hinge: HingeSide,
    /// None = cabinet `door_thickness`.
    #[serde(default)]
    pub thickness: Option<f64>,
    /// None = cabinet `door_gap` (also the gap between leaves).
    #[serde(default)]
    pub gap: Option<f64>,
    /// Khe từng phía [trái, phải, dưới, trên]; None = `gap` on every side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side_gaps: Option<[f64; 4]>,
    #[serde(default)]
    pub stop: StopRailSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawerSpec {
    pub uid: Uid,
    /// Drawers across × stacked.
    #[serde(default = "one")]
    pub cols: u32,
    pub count: u32,
    pub mount: Mount,
    #[serde(default)]
    pub face_thickness: Option<f64>,
    /// Hở hông: clearance of the front to the opening sides (inset) / reveal.
    pub side_gap: f64,
    /// Khe giữa 2 ngăn.
    pub gap: f64,
    /// Drawer box board thickness (Ván 17mm).
    pub box_thickness: f64,
    pub bottom_thickness: f64,
    /// Ball-bearing slide clearance per side.
    pub slide_clearance: f64,
    /// Generate the drawer box (sides / back / bottom).
    pub with_box: bool,
    /// Front heights bottom → top (LOCK mm / PERCENT / AUTO); empty = equal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heights: Vec<Bay>,
}

fn one() -> u32 {
    1
}

impl DrawerSpec {
    pub fn new(uid: Uid, count: u32, mount: Mount) -> Self {
        Self {
            uid,
            cols: 1,
            count,
            mount,
            face_thickness: None,
            side_gap: 3.0,
            gap: 3.0,
            box_thickness: 17.2,
            bottom_thickness: 8.6,
            slide_clearance: 13.0,
            with_box: true,
            heights: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Front {
    Doors(DoorSpec),
    Drawers(DrawerSpec),
}

impl Front {
    pub fn uid(&self) -> Uid {
        match self {
            Front::Doors(d) => d.uid,
            Front::Drawers(d) => d.uid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LinkKind {
    /// Thanh treo oval + 2 chén.
    OvalRail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub uid: Uid,
    pub kind: LinkKind,
    /// Distance from the top of the zone.
    pub offset: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub id: Uid,
    #[serde(default)]
    pub split: Option<Box<Split>>,
    #[serde(default)]
    pub front: Option<Front>,
    #[serde(default)]
    pub links: Vec<Link>,
}

impl Zone {
    pub fn new(id: Uid) -> Self {
        Self { id, split: None, front: None, links: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.split.is_none() && self.front.is_none() && self.links.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneTree {
    pub root: Zone,
    pub next_uid: Uid,
}

impl Default for ZoneTree {
    fn default() -> Self {
        Self { root: Zone::new(1), next_uid: 2 }
    }
}

fn find(z: &Zone, id: Uid) -> Option<&Zone> {
    if z.id == id {
        return Some(z);
    }
    z.split.as_ref()?.children.iter().find_map(|c| find(c, id))
}

fn find_mut(z: &mut Zone, id: Uid) -> Option<&mut Zone> {
    if z.id == id {
        return Some(z);
    }
    z.split.as_mut()?.children.iter_mut().find_map(|c| find_mut(c, id))
}

/// Zone that owns the split containing panel `uid`.
fn owner_of_panel_mut(z: &mut Zone, uid: Uid) -> Option<&mut Zone> {
    let owns = z.split.as_ref().is_some_and(|s| s.panels.iter().any(|p| p.uid == uid));
    if owns {
        return Some(z);
    }
    z.split.as_mut()?.children.iter_mut().find_map(|c| owner_of_panel_mut(c, uid))
}

fn visit<'a>(z: &'a Zone, f: &mut impl FnMut(&'a Zone)) {
    f(z);
    if let Some(s) = &z.split {
        for c in &s.children {
            visit(c, f);
        }
    }
}

impl ZoneTree {
    pub fn alloc(&mut self) -> Uid {
        let u = self.next_uid;
        self.next_uid += 1;
        u
    }

    pub fn zone(&self, id: Uid) -> Option<&Zone> {
        find(&self.root, id)
    }

    pub fn zone_mut(&mut self, id: Uid) -> Option<&mut Zone> {
        find_mut(&mut self.root, id)
    }

    pub fn zones(&self) -> Vec<&Zone> {
        let mut v = Vec::new();
        visit(&self.root, &mut |z| v.push(z));
        v
    }

    pub fn panel(&self, uid: Uid) -> Option<&SplitPanel> {
        self.zones().into_iter().find_map(|z| z.split.as_ref().and_then(|s| s.panels.iter().find(|p| p.uid == uid)))
    }

    pub fn panel_mut(&mut self, uid: Uid) -> Option<&mut SplitPanel> {
        owner_of_panel_mut(&mut self.root, uid)?.split.as_mut()?.panels.iter_mut().find(|p| p.uid == uid)
    }

    /// Zone owning front / link `uid`.
    pub fn zone_of_attachment(&self, uid: Uid) -> Option<Uid> {
        self.zones()
            .into_iter()
            .find(|z| z.front.as_ref().is_some_and(|f| f.uid() == uid) || z.links.iter().any(|l| l.uid == uid))
            .map(|z| z.id)
    }

    /// Split zone `zone_id` with `count` new panels. A zone that already has a
    /// split along the same axis receives the panels into that split.
    pub fn add_panels(&mut self, zone_id: Uid, kind: SplitKind, count: u32, thickness: f64, lock: Lock, value: f64) -> Result<Vec<Uid>, String> {
        if count == 0 || count > 50 {
            return Err("count must be 1..50".into());
        }
        if !(thickness > 0.0) {
            return Err("thickness must be > 0".into());
        }
        let uids: Vec<Uid> = (0..count).map(|_| self.alloc()).collect();
        let child_ids: Vec<Uid> = (0..count).map(|_| self.alloc()).collect();
        let zone = self.zone_mut(zone_id).ok_or("zone not found")?;
        let lock = if count > 1 { Lock::Even } else { lock };
        let make = |uid| SplitPanel { uid, kind, thickness, lock, value, tilt_deg: [0.0; 2] };
        match &mut zone.split {
            Some(s) if s.axis == kind.axis() => {
                // Append into the existing split (new zones are empty).
                for (u, c) in uids.iter().zip(child_ids.iter()) {
                    s.panels.push(make(*u));
                    s.children.push(Zone::new(*c));
                    if !s.bays.is_empty() {
                        s.bays.push(Bay::auto());
                    }
                }
            }
            Some(_) => return Err("zone already split along another axis; pick a sub-zone".into()),
            None => {
                // The current content of the zone (links) moves to the first sub-zone;
                // fronts stay on the parent (they cover the whole opening).
                let first_id = self.next_uid;
                self.next_uid += 1;
                let zone = self.zone_mut(zone_id).unwrap();
                let mut first = Zone::new(first_id);
                first.links = std::mem::take(&mut zone.links);
                let mut children = vec![first];
                children.extend(child_ids.iter().map(|c| Zone::new(*c)));
                zone.split = Some(Box::new(Split { axis: kind.axis(), panels: uids.iter().map(|u| make(*u)).collect(), children, bays: Vec::new() }));
            }
        }
        Ok(uids)
    }

    /// Remove a split panel; its two neighbouring zones merge (gộp zone).
    /// The split that owns panel `uid` (and the panel's index in it).
    pub fn split_of_panel_mut(&mut self, uid: Uid) -> Option<(&mut Split, usize)> {
        let z = owner_of_panel_mut(&mut self.root, uid)?;
        let s = z.split.as_mut()?;
        let i = s.panels.iter().position(|p| p.uid == uid)?;
        Some((s, i))
    }

    pub fn split_of_panel(&self, uid: Uid) -> Option<&Split> {
        fn find(z: &Zone, uid: Uid) -> Option<&Split> {
            let s = z.split.as_deref()?;
            if s.panels.iter().any(|p| p.uid == uid) {
                return Some(s);
            }
            s.children.iter().find_map(|c| find(c, uid))
        }
        find(&self.root, uid)
    }

    pub fn split_mut(&mut self, zone_id: Uid) -> Option<&mut Split> {
        self.zone_mut(zone_id)?.split.as_deref_mut()
    }

    pub fn remove_panel(&mut self, uid: Uid) -> Result<(), String> {
        let zone = owner_of_panel_mut(&mut self.root, uid).ok_or("panel not found")?;
        let split = zone.split.as_mut().unwrap();
        let i = split.panels.iter().position(|p| p.uid == uid).unwrap();
        split.panels.remove(i);
        if split.bays.len() == split.panels.len() + 2 {
            // Merge bays i and i+1: AUTO wins, two LOCK add up (with the removed panel).
            let (x, y) = (split.bays[i], split.bays.remove(i + 1));
            split.bays[i] = match (x.mode, y.mode) {
                (BayMode::Lock, BayMode::Lock) => Bay::lock(x.value + y.value),
                (BayMode::Percent, BayMode::Percent) => Bay::percent(x.value + y.value),
                _ => Bay::auto(),
            };
            if !split.bays.iter().any(|b| b.mode != BayMode::Lock) {
                split.bays[i] = Bay::auto();
            }
        }
        let b = split.children.remove(i + 1);
        let a = &mut split.children[i];
        if a.is_empty() {
            *a = b;
        } else if !b.is_empty() {
            // Keep A's structure; carry over B's attachments where A has none.
            if a.front.is_none() {
                a.front = b.front;
            }
            a.links.extend(b.links);
        }
        if split.panels.is_empty() {
            let only = split.children.remove(0);
            zone.split = only.split;
            if zone.front.is_none() {
                zone.front = only.front;
            }
            zone.links.extend(only.links);
        }
        Ok(())
    }

    pub fn set_front(&mut self, zone_id: Uid, front: Option<Front>) -> Result<Option<Front>, String> {
        let z = self.zone_mut(zone_id).ok_or("zone not found")?;
        Ok(std::mem::replace(&mut z.front, front))
    }

    pub fn front_mut(&mut self, uid: Uid) -> Option<&mut Front> {
        let zid = self.zone_of_attachment(uid)?;
        self.zone_mut(zid)?.front.as_mut().filter(|f| f.uid() == uid)
    }

    pub fn remove_attachment(&mut self, uid: Uid) -> Result<(), String> {
        let zid = self.zone_of_attachment(uid).ok_or("not found")?;
        let z = self.zone_mut(zid).unwrap();
        if z.front.as_ref().is_some_and(|f| f.uid() == uid) {
            z.front = None;
        }
        z.links.retain(|l| l.uid != uid);
        Ok(())
    }

    pub fn add_link(&mut self, zone_id: Uid, kind: LinkKind, offset: f64) -> Result<Uid, String> {
        let uid = self.alloc();
        let z = self.zone_mut(zone_id).ok_or("zone not found")?;
        z.links.push(Link { uid, kind, offset });
        Ok(uid)
    }

    /// Legacy helper: evenly spaced adjustable shelves in `zone_id` (count may be 0).
    pub fn set_even_shelves(&mut self, zone_id: Uid, count: u32, thickness: f64) -> Result<(), String> {
        let existing: Vec<Uid> = match self.zone(zone_id).and_then(|z| z.split.as_ref()) {
            Some(s) if s.axis == 1 => s.panels.iter().map(|p| p.uid).collect(),
            Some(_) => return Err("zone split along another axis".into()),
            None => Vec::new(),
        };
        let n = count as usize;
        if existing.len() > n {
            for uid in existing[n..].iter().rev() {
                self.remove_panel(*uid)?;
            }
        } else if existing.len() < n {
            self.add_panels(zone_id, SplitKind::ShelfAdjustable, (n - existing.len()) as u32, thickness, Lock::Even, 0.0)?;
        }
        Ok(())
    }

    /// Zone used by legacy shelf counts: root, or root's first child when root is divided.
    pub fn shelf_zone(&self) -> Uid {
        match &self.root.split {
            Some(s) if s.axis != 1 => s.children[0].id,
            _ => self.root.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_merge() {
        let mut t = ZoneTree::default();
        let root = t.root.id;
        let d = t.add_panels(root, SplitKind::Divider, 1, 17.2, Lock::Ratio, 0.5).unwrap();
        let left = t.root.split.as_ref().unwrap().children[0].id;
        t.add_panels(left, SplitKind::ShelfAdjustable, 3, 17.2, Lock::Ratio, 0.5).unwrap();
        assert_eq!(t.zone(left).unwrap().split.as_ref().unwrap().panels.len(), 3);
        assert!(t.add_panels(root, SplitKind::ShelfFixed, 1, 17.2, Lock::Even, 0.0).is_err());
        t.remove_panel(d[0]).unwrap();
        // Divider removed: the shelves' zone becomes the root content.
        assert_eq!(t.root.split.as_ref().unwrap().axis, 1);
        assert_eq!(t.root.split.as_ref().unwrap().panels.len(), 3);
    }

    #[test]
    fn legacy_even_shelves() {
        let mut t = ZoneTree::default();
        let z = t.root.id;
        t.set_even_shelves(z, 4, 18.0).unwrap();
        let first = t.root.split.as_ref().unwrap().panels[0].uid;
        t.set_even_shelves(z, 2, 18.0).unwrap();
        assert_eq!(t.root.split.as_ref().unwrap().panels.len(), 2);
        assert_eq!(t.root.split.as_ref().unwrap().panels[0].uid, first);
        t.set_even_shelves(z, 0, 18.0).unwrap();
        assert!(t.root.split.is_none());
    }
}

impl Zone {
    /// Lật gương theo chiều rộng: splits along X reverse, hinges swap sides.
    pub fn mirror_x(&mut self) {
        if let Some(s) = &mut self.split {
            if s.axis == 0 {
                s.panels.reverse();
                s.children.reverse();
                s.bays.reverse();
                for p in &mut s.panels {
                    match p.lock {
                        Lock::FromStart => p.lock = Lock::FromEnd,
                        Lock::FromEnd => p.lock = Lock::FromStart,
                        Lock::Ratio => p.value = 1.0 - p.value,
                        Lock::Even => {}
                    }
                }
            }
            for c in &mut s.children {
                c.mirror_x();
            }
        }
        if let Some(Front::Doors(d)) = &mut self.front {
            d.hinge = match d.hinge {
                HingeSide::Left => HingeSide::Right,
                HingeSide::Right => HingeSide::Left,
                h => h,
            };
        }
    }
}

#[cfg(test)]
mod bay_tests {
    use super::*;

    #[test]
    fn lock_auto_lock_keeps_locked_bays() {
        let t = [17.2, 17.2];
        let bays = [Bay::lock(600.0), Bay::auto(), Bay::lock(400.0)];
        let (s, ok) = solve_bays(1600.0 - 2.0 * 17.2, &t, &bays);
        assert!(ok);
        assert_eq!(s[0], 600.0);
        assert_eq!(s[2], 400.0);
        let (s2, _) = solve_bays(1800.0 - 2.0 * 17.2, &t, &bays);
        assert_eq!((s2[0], s2[2]), (600.0, 400.0));
        assert!((s2[1] - s[1] - 200.0).abs() < 1e-9, "only the AUTO bay grows");
    }

    #[test]
    fn percent_keeps_ratio_and_conflict_is_reported() {
        let bays = [Bay::percent(40.0), Bay::percent(20.0), Bay::percent(40.0)];
        let (s, ok) = solve_bays(1034.4, &[17.2, 17.2], &bays);
        assert!(ok);
        assert!((s[0] - 400.0).abs() < 1e-9 && (s[1] - 200.0).abs() < 1e-9);
        let (_, ok) = solve_bays(500.0, &[17.2], &[Bay::lock(400.0), Bay::lock(400.0)]);
        assert!(!ok);
        let (_, ok) = solve_bays(500.0, &[17.2], &[Bay::lock(600.0), Bay::auto()]);
        assert!(!ok, "auto bay would be negative");
    }

    #[test]
    fn move_panel_changes_adjacent_bays_only() {
        let mut sp = Split { axis: 0, panels: vec![], children: vec![], bays: vec![] };
        for u in 0..2 {
            sp.panels.push(SplitPanel { uid: u, kind: SplitKind::Divider, thickness: 17.2, lock: Lock::Even, value: 0.0, tilt_deg: [0.0; 2] });
        }
        sp.bays = vec![Bay::lock(600.0), Bay::auto(), Bay::lock(400.0)];
        let sizes = [600.0, 500.0, 400.0];
        sp.move_panel(0, &sizes, 700.0).unwrap();
        assert_eq!(sp.bays[0], Bay::lock(700.0));
        assert_eq!(sp.bays[1], Bay::auto());
        sp.move_panel(1, &[700.0, 400.0, 400.0], 300.0).unwrap();
        assert_eq!(sp.bays[2], Bay::lock(500.0), "AUTO absorbs; the locked neighbour is rewritten");
        // Two AUTO neighbours: the first becomes LOCK.
        sp.bays = vec![Bay::auto(); 3];
        sp.move_panel(0, &[500.0, 500.0, 500.0], 742.0).unwrap();
        assert_eq!(sp.bays[0], Bay::lock(742.0));
        assert!(sp.move_panel(0, &[500.0, 500.0, 500.0], 1200.0).is_err());
    }

    #[test]
    fn typed_size_in_percent_split_goes_to_the_neighbour() {
        let mut sp = Split { axis: 0, panels: vec![], children: vec![], bays: vec![Bay::percent(50.0), Bay::percent(50.0)] };
        sp.panels.push(SplitPanel { uid: 0, kind: SplitKind::Divider, thickness: 17.2, lock: Lock::Even, value: 0.0, tilt_deg: [0.0; 2] });
        let sizes = [774.2, 774.2];
        sp.set_bay(0, &sizes, Some(BayMode::Percent), Some(600.0 / 1548.4 * 100.0)).unwrap();
        let (s, ok) = solve_bays(1548.4 + 17.2, &[17.2], &sp.bays);
        assert!(ok && (s[0] - 600.0).abs() < 1e-6 && (s[1] - 948.4).abs() < 1e-6, "{s:?}");
        assert_eq!(sp.bays[1].mode, BayMode::Percent);
    }
}
