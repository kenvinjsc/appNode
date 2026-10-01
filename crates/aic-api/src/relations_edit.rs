//! Quan hệ giữa 2 tấm (Lọt / Phủ / Bằng mặt / Khe) and edge drags, built on the
//! dynamic anchors of the part mods: the layout keeps them when the cabinet changes.

use crate::zones::bad;
use crate::Engine;
use aic_domain::layout::{edge_along, part_aabb, AnchorFace, EdgeAnchor};
use aic_domain::{EdgeSide, ObjectId};
use aic_project::CoreError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationKind {
    /// Lọt: A stops at B's inner face, B runs past A.
    Inset,
    /// Phủ: A covers B's edge (to B's outer face), B stops at A.
    Overlay,
    /// Bằng mặt: A's front edge flush with B's front edge.
    Flush,
    /// Khe: like inset with a gap between A and B.
    Gap,
    /// Bỏ quan hệ: remove the anchors between A and B.
    None,
}

/// Cabinet-frame side of a drag (2D handles / numeric resize).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Left,
    Right,
    Bottom,
    Top,
    Back,
    Front,
}

/// Cạnh tấm bám vào mặt nào, cách bao nhiêu.
type EdgeRule = (EdgeSide, AnchorFace, f64);

impl Side {
    fn index(self) -> usize {
        self as usize
    }
    fn dir(self) -> [f64; 3] {
        match self {
            Side::Left => [-1.0, 0.0, 0.0],
            Side::Right => [1.0, 0.0, 0.0],
            Side::Bottom => [0.0, -1.0, 0.0],
            Side::Top => [0.0, 1.0, 0.0],
            Side::Back => [0.0, 0.0, -1.0],
            Side::Front => [0.0, 0.0, 1.0],
        }
    }
}

fn centre(b: &([f64; 3], [f64; 3])) -> [f64; 3] {
    [(b.0[0] + b.1[0]) / 2.0, (b.0[1] + b.1[1]) / 2.0, (b.0[2] + b.1[2]) / 2.0]
}

impl Engine {
    /// SET_RELATION between two panels of one cabinet (one undo step).
    pub(crate) fn set_relation(&mut self, a: ObjectId, b: ObjectId, kind: RelationKind, gap: f64) -> Result<Value, CoreError> {
        let (cab, ka) = self.part_ref(a).ok_or_else(|| bad("relation", "cabinet parts only"))?;
        let (cb, kb) = self.part_ref(b).ok_or_else(|| bad("relation", "cabinet parts only"))?;
        if cab != cb || ka == kb {
            return Err(bad("relation", "two parts of the same cabinet"));
        }
        let layout = self.doc.cabinet_layout(cab).ok_or(CoreError::NotFound { id: cab })?;
        let pa = layout.parts.iter().find(|p| p.key == ka).ok_or_else(|| bad("relation", "part not found"))?.clone();
        let pb = layout.parts.iter().find(|p| p.key == kb).ok_or_else(|| bad("relation", "part not found"))?.clone();
        let (ba, bb) = (part_aabb(&pa), part_aabb(&pb));
        let (ca, cb2) = (centre(&ba), centre(&bb));
        let ab = [cb2[0] - ca[0], cb2[1] - ca[1], cb2[2] - ca[2]];
        let ba_dir = [-ab[0], -ab[1], -ab[2]];
        let ea = edge_along(pa.rotation_deg, ab, 0.3);
        let eb = edge_along(pb.rotation_deg, ba_dir, 0.3);
        let front = edge_along(pa.rotation_deg, [0.0, 0.0, 1.0], 0.9);
        let (kind_a, kind_b): (Option<EdgeRule>, Option<EdgeRule>) = match kind {
            RelationKind::Inset => (ea.map(|e| (e, AnchorFace::Inner, 0.0)), eb.map(|e| (e, AnchorFace::Outer, 0.0))),
            RelationKind::Gap => (ea.map(|e| (e, AnchorFace::Inner, gap)), eb.map(|e| (e, AnchorFace::Outer, 0.0))),
            RelationKind::Overlay => (ea.map(|e| (e, AnchorFace::Outer, 0.0)), eb.map(|e| (e, AnchorFace::Inner, 0.0))),
            RelationKind::Flush => (front.map(|e| (e, AnchorFace::Flush, 0.0)), None),
            RelationKind::None => (None, None),
        };
        if kind != RelationKind::None && kind_a.is_none() {
            return Err(bad("relation", "no edge of the first panel faces the second"));
        }
        self.edit_cabinet_checked(cab, "Quan hệ tấm", |c| {
            // Drop the previous anchors between the two parts.
            for (k, other) in [(&ka, &kb), (&kb, &ka)] {
                if let Some(m) = c.mods.get_mut(k) {
                    m.anchors.retain(|x| &x.target != other);
                }
            }
            for (k, other, spec) in [(&ka, &kb, kind_a), (&kb, &ka, kind_b)] {
                if let Some((edge, face, offset)) = spec {
                    let m = c.mods.entry(k.clone()).or_default();
                    m.anchors.retain(|x| x.edge != edge);
                    m.anchors.push(EdgeAnchor { edge, target: other.clone(), face, offset });
                }
            }
            c.mods.retain(|_, m| !m.is_default());
            Ok(())
        })?;
        Ok(json!({ "edge_a": kind_a.map(|x| x.0), "edge_b": kind_b.map(|x| x.0) }))
    }

    /// Kéo cạnh tấm: grow `side` (cabinet frame) by `delta` mm. Constrained: an anchored
    /// edge keeps its relation and only its gap changes; free: the part's offset changes.
    pub(crate) fn resize_panel_side(&mut self, id: ObjectId, side: Side, delta: f64, constrained: bool) -> Result<(), CoreError> {
        if !delta.is_finite() {
            return Err(bad("resize", "delta must be a number"));
        }
        let (cab, key) = self.part_ref(id).ok_or_else(|| bad("part", "cabinet parts only"))?;
        let layout = self.doc.cabinet_layout(cab).ok_or(CoreError::NotFound { id: cab })?;
        let rot = layout.parts.iter().find(|p| p.key == key).map(|p| p.rotation_deg).unwrap_or([0.0; 3]);
        let edge = edge_along(rot, side.dir(), 0.9);
        self.edit_cabinet_checked(cab, "Kéo cạnh tấm", |c| {
            let m = c.mods.entry(key.clone()).or_default();
            if let (true, Some(e)) = (constrained, edge) {
                if let Some(a) = m.anchors.iter_mut().find(|a| a.edge == e) {
                    a.offset -= delta;
                    return Ok(());
                }
            }
            if !constrained {
                if let Some(e) = edge {
                    m.anchors.retain(|a| a.edge != e);
                }
            }
            m.offsets[side.index()] -= delta;
            Ok(())
        })
    }
}
