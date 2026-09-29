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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub axis: usize,
    pub panels: Vec<SplitPanel>,
    /// `panels.len() + 1` sub-zones, in order along the axis.
    pub children: Vec<Zone>,
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
    /// None = cabinet `door_gap`.
    #[serde(default)]
    pub gap: Option<f64>,
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
                zone.split = Some(Box::new(Split { axis: kind.axis(), panels: uids.iter().map(|u| make(*u)).collect(), children }));
            }
        }
        Ok(uids)
    }

    /// Remove a split panel; its two neighbouring zones merge (gộp zone).
    pub fn remove_panel(&mut self, uid: Uid) -> Result<(), String> {
        let zone = owner_of_panel_mut(&mut self.root, uid).ok_or("panel not found")?;
        let split = zone.split.as_mut().unwrap();
        let i = split.panels.iter().position(|p| p.uid == uid).unwrap();
        split.panels.remove(i);
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
