use aic_domain::ObjectId;
use aic_math::{Aabb, Obb};

#[derive(Debug, Clone, Copy)]
pub struct SpatialItem {
    pub id: ObjectId,
    pub obb: Obb,
    pub aabb: Aabb,
}

impl SpatialItem {
    pub fn new(id: ObjectId, obb: Obb) -> Self {
        Self { id, obb, aabb: obb.aabb() }
    }
}

/// Sort-and-sweep index along X. O(n log n + k) candidate generation, which
/// comfortably handles thousands of parts.
#[derive(Debug, Clone, Default)]
pub struct SpatialIndex {
    items: Vec<SpatialItem>,
    /// Indices into `items` sorted by aabb.min.x.
    order: Vec<usize>,
}

impl SpatialIndex {
    pub fn build(items: Vec<SpatialItem>) -> Self {
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by(|a, b| items[*a].aabb.min[0].total_cmp(&items[*b].aabb.min[0]));
        Self { items, order }
    }

    pub fn items(&self) -> &[SpatialItem] {
        &self.items
    }

    pub fn get(&self, id: ObjectId) -> Option<&SpatialItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Pairs (i, j) of item indices whose AABBs inflated by `margin` overlap.
    pub fn candidate_pairs(&self, margin: f64) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (k, &i) in self.order.iter().enumerate() {
            let a = self.items[i].aabb.inflate(margin);
            for &j in &self.order[k + 1..] {
                let b = &self.items[j].aabb;
                if b.min[0] > a.max[0] {
                    break;
                }
                if a.overlaps(b) {
                    out.push((i.min(j), i.max(j)));
                }
            }
        }
        out
    }

    pub fn query_aabb(&self, q: &Aabb) -> Vec<ObjectId> {
        self.items.iter().filter(|i| i.aabb.overlaps(q)).map(|i| i.id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aic_math::Transform3D;

    #[test]
    fn sweep_finds_only_neighbours() {
        let items: Vec<SpatialItem> = (0..100)
            .map(|i| SpatialItem::new(ObjectId(i), Obb::new([10.0, 10.0, 10.0], Transform3D::from_translation(i as f64 * 20.0, 0.0, 0.0))))
            .collect();
        let idx = SpatialIndex::build(items);
        assert!(idx.candidate_pairs(1.0).is_empty());
        assert_eq!(idx.candidate_pairs(10.0).len(), 99);
    }
}
