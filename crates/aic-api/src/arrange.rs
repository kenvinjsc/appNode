//! Căn / phân bố / xoay 90° / đặt sát tường cho các đối tượng đã chọn (D27). Tính trên hộp bao thế giới;
//! mỗi thao tác là một `Command::Batch` các `SetTransform` (một bước undo).

use crate::zones::bad;
use crate::Engine;
use aic_domain::{DomainObject, ObjectId};
use aic_math::{Aabb, Transform3D};
use aic_project::{Command, CoreError};

impl Engine {
    fn boxes(&self, ids: &[ObjectId]) -> Vec<(ObjectId, Aabb)> {
        ids.iter().map(|&i| (i, self.doc.world_aabb(i))).filter(|(_, b)| !b.is_empty()).collect()
    }

    fn batch(&mut self, label: &str, cmds: Vec<Command>) -> Result<usize, CoreError> {
        let n = cmds.len();
        if n > 0 {
            self.exec_cmd(Command::Batch { label: label.into(), commands: cmds })?;
        }
        Ok(n)
    }

    /// `mode`: LEFT / RIGHT / BOTTOM / TOP / BACK / FRONT (theo mép ngoài cùng) · CENTER_X / CENTER_Y / CENTER_Z (theo tâm hộp bao chung).
    pub(crate) fn align_objects(&mut self, ids: Vec<ObjectId>, mode: &str) -> Result<usize, CoreError> {
        let b = self.boxes(&self.top_level_only(ids));
        if b.len() < 2 {
            return Err(bad("arrange", "select two or more objects"));
        }
        let all = b.iter().fold(Aabb::empty(), |a, (_, x)| a.union(x));
        let (axis, target): (usize, Box<dyn Fn(&Aabb) -> f64>) = match mode.to_ascii_uppercase().as_str() {
            "LEFT" => (0, Box::new(move |x: &Aabb| all.min[0] - x.min[0])),
            "RIGHT" => (0, Box::new(move |x: &Aabb| all.max[0] - x.max[0])),
            "BOTTOM" => (1, Box::new(move |x: &Aabb| all.min[1] - x.min[1])),
            "TOP" => (1, Box::new(move |x: &Aabb| all.max[1] - x.max[1])),
            "BACK" => (2, Box::new(move |x: &Aabb| all.min[2] - x.min[2])),
            "FRONT" => (2, Box::new(move |x: &Aabb| all.max[2] - x.max[2])),
            m @ ("CENTER_X" | "CENTER_Y" | "CENTER_Z") => {
                let a = match m {
                    "CENTER_X" => 0,
                    "CENTER_Y" => 1,
                    _ => 2,
                };
                let c = (all.min[a] + all.max[a]) / 2.0;
                (a, Box::new(move |x: &Aabb| c - (x.min[a] + x.max[a]) / 2.0))
            }
            _ => return Err(bad("arrange", "LEFT | RIGHT | BOTTOM | TOP | BACK | FRONT | CENTER_X | CENTER_Y | CENTER_Z")),
        };
        let mut cmds = Vec::new();
        for (id, x) in &b {
            let d = target(x);
            if d.abs() > 1e-6 {
                let mut v = [0.0; 3];
                v[axis] = d;
                cmds.push(self.shift_world(*id, v)?);
            }
        }
        self.batch("Căn chỉnh", cmds)
    }

    /// Phân bố đều theo trục (X / Y / Z): giữ đối tượng đầu / cuối, khe giữa các đối tượng bằng nhau.
    pub(crate) fn distribute_objects(&mut self, ids: Vec<ObjectId>, axis: &str) -> Result<usize, CoreError> {
        let a = match axis.to_ascii_uppercase().as_str() {
            "X" => 0,
            "Y" => 1,
            "Z" => 2,
            _ => return Err(bad("arrange", "X | Y | Z")),
        };
        let mut b = self.boxes(&self.top_level_only(ids));
        if b.len() < 3 {
            return Err(bad("arrange", "select three or more objects"));
        }
        b.sort_by(|p, q| p.1.min[a].total_cmp(&q.1.min[a]));
        let (first, last) = (b[0].1.min[a], b[b.len() - 1].1.max[a]);
        let sizes: f64 = b.iter().map(|(_, x)| x.max[a] - x.min[a]).sum();
        let gap = (last - first - sizes) / (b.len() - 1) as f64;
        let mut at = first;
        let mut cmds = Vec::new();
        for (id, x) in &b {
            let d = at - x.min[a];
            if d.abs() > 1e-6 {
                let mut v = [0.0; 3];
                v[a] = d;
                cmds.push(self.shift_world(*id, v)?);
            }
            at += x.max[a] - x.min[a] + gap;
        }
        self.batch("Phân bố đều", cmds)
    }

    /// Xoay ±90° quanh trục đứng qua `pivot` của hộp bao từng đối tượng (CENTER / LEFT_BACK / RIGHT_BACK).
    pub(crate) fn rotate_objects(&mut self, ids: Vec<ObjectId>, deg: f64, pivot: &str) -> Result<usize, CoreError> {
        if !deg.is_finite() {
            return Err(CoreError::InvalidTransform);
        }
        let b = self.boxes(&self.top_level_only(ids));
        let mut cmds = Vec::new();
        for (id, x) in &b {
            let p = match pivot.to_ascii_uppercase().as_str() {
                "LEFT_BACK" => [x.min[0], 0.0, x.min[2]],
                "RIGHT_BACK" => [x.max[0], 0.0, x.min[2]],
                _ => [(x.min[0] + x.max[0]) / 2.0, 0.0, (x.min[2] + x.max[2]) / 2.0],
            };
            let rot = Transform3D::from_translation(p[0], p[1], p[2]).compose(&Transform3D::new([0.0; 3], [0.0, deg, 0.0])).compose(&Transform3D::from_translation(-p[0], -p[1], -p[2]));
            let world = rot.compose(&self.doc.scene.world(*id));
            let local = match self.doc.scene.parent(*id) {
                Some(par) => self.doc.scene.world(par).inverse().compose(&world),
                None => world,
            };
            cmds.push(Command::SetTransform { id: *id, transform: local });
        }
        self.batch("Xoay 90°", cmds)
    }

    /// Đặt sát tường gần nhất của phòng (tường sau z = 0, trái x = 0, phải x = rộng phòng), cách `gap`.
    pub(crate) fn snap_to_wall(&mut self, ids: Vec<ObjectId>, gap: f64) -> Result<usize, CoreError> {
        let rooms: Vec<(ObjectId, f64, f64)> = self
            .doc
            .objects
            .iter()
            .filter_map(|(id, o)| match o {
                DomainObject::Room(r) => Some((*id, r.width_mm, r.depth_mm)),
                _ => None,
            })
            .collect();
        let &(room, rw, _rd) = rooms.first().ok_or_else(|| bad("arrange", "no room"))?;
        let inv = self.doc.scene.world(room).inverse();
        let rw_world = self.doc.scene.world(room);
        let b = self.boxes(&self.top_level_only(ids));
        let mut cmds = Vec::new();
        for (id, x) in &b {
            // Hộp bao trong khung phòng.
            let mut mn = [f64::MAX; 3];
            let mut mx = [f64::MIN; 3];
            for i in 0..8 {
                let q = inv.transform_point([if i & 1 != 0 { x.max[0] } else { x.min[0] }, if i & 2 != 0 { x.max[1] } else { x.min[1] }, if i & 4 != 0 { x.max[2] } else { x.min[2] }]);
                for k in 0..3 {
                    mn[k] = mn[k].min(q[k]);
                    mx[k] = mx[k].max(q[k]);
                }
            }
            let cands = [(mn[2] - gap, [0.0, 0.0, -(mn[2] - gap)]), (mn[0] - gap, [-(mn[0] - gap), 0.0, 0.0]), (rw - gap - mx[0], [rw - gap - mx[0], 0.0, 0.0])];
            let (_, v) = cands.iter().min_by(|p, q| p.0.abs().total_cmp(&q.0.abs())).copied().unwrap();
            let vw = rw_world.transform_vector(v);
            if vw.iter().any(|c| c.abs() > 1e-6) {
                cmds.push(self.shift_world(*id, vw)?);
            }
        }
        self.batch("Đặt sát tường", cmds)
    }
}
