//! Scene graph: hierarchy, local/world transforms with dirty propagation,
//! visibility and locking. One node per domain object in the MVP.

use crate::{DomainError, NodeId, ObjectId};
use aic_math::Transform3D;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub local_transform: Transform3D,
    pub object_id: ObjectId,
    pub visible: bool,
    pub locked: bool,
}

impl SceneNode {
    pub fn new(id: ObjectId, local_transform: Transform3D) -> Self {
        Self { id, parent: None, children: Vec::new(), local_transform, object_id: id, visible: true, locked: false }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scene {
    nodes: BTreeMap<NodeId, SceneNode>,
    roots: Vec<NodeId>,
    #[serde(skip)]
    world: HashMap<NodeId, Transform3D>,
    /// Roots of subtrees whose world transforms are stale.
    #[serde(skip)]
    dirty: BTreeSet<NodeId>,
}

impl PartialEq for Scene {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes && self.roots == other.roots
    }
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn node(&self, id: NodeId) -> Result<&SceneNode, DomainError> {
        self.nodes.get(&id).ok_or(DomainError::NotFound(id))
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut SceneNode, DomainError> {
        self.nodes.get_mut(&id).ok_or(DomainError::NotFound(id))
    }

    pub fn nodes(&self) -> impl Iterator<Item = &SceneNode> {
        self.nodes.values()
    }

    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(&id).and_then(|n| n.parent)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.nodes.get(&id).map(|n| n.children.as_slice()).unwrap_or(&[])
    }

    fn siblings_mut(&mut self, parent: Option<NodeId>) -> Result<&mut Vec<NodeId>, DomainError> {
        match parent {
            Some(p) => Ok(&mut self.node_mut(p)?.children),
            None => Ok(&mut self.roots),
        }
    }

    pub fn index_in_parent(&self, id: NodeId) -> Option<usize> {
        let parent = self.nodes.get(&id)?.parent;
        let list = match parent {
            Some(p) => &self.nodes.get(&p)?.children,
            None => &self.roots,
        };
        list.iter().position(|c| *c == id)
    }

    /// Insert a node (its `children` must be empty; children are inserted separately).
    pub fn insert(&mut self, mut node: SceneNode, parent: Option<NodeId>, index: Option<usize>) -> Result<(), DomainError> {
        if let Some(p) = parent {
            self.node(p)?;
        }
        let id = node.id;
        node.parent = parent;
        node.children.clear();
        self.nodes.insert(id, node);
        let list = self.siblings_mut(parent)?;
        let at = index.unwrap_or(list.len()).min(list.len());
        list.insert(at, id);
        self.mark_dirty(id);
        Ok(())
    }

    /// Pre-order list of `id` and all its descendants.
    pub fn subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            if let Some(node) = self.nodes.get(&n) {
                out.push(n);
                for c in node.children.iter().rev() {
                    stack.push(*c);
                }
            }
        }
        out
    }

    /// Remove `id` and its subtree. Returns the removed nodes in pre-order.
    pub fn remove_subtree(&mut self, id: NodeId) -> Result<Vec<SceneNode>, DomainError> {
        let parent = self.node(id)?.parent;
        self.siblings_mut(parent)?.retain(|c| *c != id);
        let ids = self.subtree(id);
        let mut removed = Vec::with_capacity(ids.len());
        for n in ids {
            if let Some(node) = self.nodes.remove(&n) {
                self.world.remove(&n);
                self.dirty.remove(&n);
                removed.push(node);
            }
        }
        Ok(removed)
    }

    /// Re-insert nodes previously returned by [`remove_subtree`] (pre-order).
    pub fn restore_subtree(&mut self, nodes: Vec<SceneNode>, index: Option<usize>) -> Result<(), DomainError> {
        let Some(root) = nodes.first() else { return Ok(()) };
        let root_id = root.id;
        let parent = root.parent;
        if let Some(p) = parent {
            self.node(p)?;
        }
        for n in nodes {
            self.nodes.insert(n.id, n);
        }
        let list = self.siblings_mut(parent)?;
        let at = index.unwrap_or(list.len()).min(list.len());
        list.insert(at, root_id);
        self.mark_dirty(root_id);
        Ok(())
    }

    pub fn is_ancestor(&self, ancestor: NodeId, mut id: NodeId) -> bool {
        while let Some(p) = self.parent(id) {
            if p == ancestor {
                return true;
            }
            id = p;
        }
        false
    }

    /// Move `id` under `new_parent`. The world placement is preserved by
    /// recomputing the local transform relative to the new parent.
    pub fn reparent(&mut self, id: NodeId, new_parent: Option<NodeId>, index: Option<usize>) -> Result<(), DomainError> {
        self.node(id)?;
        if let Some(p) = new_parent {
            self.node(p)?;
            if p == id || self.is_ancestor(id, p) {
                return Err(DomainError::InvalidReparent { child: id, parent: p });
            }
        }
        self.update_world();
        let world = self.world(id);
        let old_parent = self.node(id)?.parent;
        self.siblings_mut(old_parent)?.retain(|c| *c != id);
        let parent_world = new_parent.map(|p| self.world(p)).unwrap_or_default();
        let node = self.node_mut(id)?;
        node.parent = new_parent;
        node.local_transform = parent_world.inverse().compose(&world);
        let list = self.siblings_mut(new_parent)?;
        let at = index.unwrap_or(list.len()).min(list.len());
        list.insert(at, id);
        self.mark_dirty(id);
        Ok(())
    }

    pub fn set_local_transform(&mut self, id: NodeId, t: Transform3D) -> Result<Transform3D, DomainError> {
        if !t.is_finite() {
            return Err(DomainError::InvalidTransform);
        }
        let node = self.node_mut(id)?;
        let old = std::mem::replace(&mut node.local_transform, t);
        if old != t {
            self.mark_dirty(id);
        }
        Ok(old)
    }

    pub fn set_visible(&mut self, id: NodeId, v: bool) -> Result<bool, DomainError> {
        let n = self.node_mut(id)?;
        Ok(std::mem::replace(&mut n.visible, v))
    }

    pub fn set_locked(&mut self, id: NodeId, v: bool) -> Result<bool, DomainError> {
        let n = self.node_mut(id)?;
        Ok(std::mem::replace(&mut n.locked, v))
    }

    /// Visible only if the node and all its ancestors are visible.
    pub fn is_effectively_visible(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(c) = cur {
            match self.nodes.get(&c) {
                Some(n) if n.visible => cur = n.parent,
                _ => return false,
            }
        }
        true
    }

    /// Locked if the node or any ancestor is locked.
    pub fn is_effectively_locked(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(c) = cur {
            match self.nodes.get(&c) {
                Some(n) if n.locked => return true,
                Some(n) => cur = n.parent,
                None => return false,
            }
        }
        false
    }

    pub fn mark_dirty(&mut self, id: NodeId) {
        self.dirty.insert(id);
    }

    pub fn mark_all_dirty(&mut self) {
        self.world.clear();
        self.dirty = self.roots.iter().copied().collect();
    }

    /// Recompute world transforms of dirty subtrees. Returns every node whose
    /// world transform was recomputed (i.e. needs a render-side matrix update).
    pub fn update_world(&mut self) -> Vec<NodeId> {
        if self.dirty.is_empty() {
            return Vec::new();
        }
        let dirty = std::mem::take(&mut self.dirty);
        // Skip nodes that have a dirty ancestor (they are covered by it).
        let tops: Vec<NodeId> = dirty
            .iter()
            .copied()
            .filter(|id| self.nodes.contains_key(id))
            .filter(|id| {
                let mut cur = self.parent(*id);
                while let Some(p) = cur {
                    if dirty.contains(&p) {
                        return false;
                    }
                    cur = self.parent(p);
                }
                true
            })
            .collect();
        let mut updated = Vec::new();
        for top in tops {
            let parent_world = match self.parent(top) {
                Some(p) => self.world_uncached(p),
                None => Transform3D::IDENTITY,
            };
            let mut stack = vec![(top, parent_world)];
            while let Some((id, pw)) = stack.pop() {
                let Some(node) = self.nodes.get(&id) else { continue };
                let w = pw.compose(&node.local_transform);
                for c in &node.children {
                    stack.push((*c, w));
                }
                self.world.insert(id, w);
                updated.push(id);
            }
        }
        updated
    }

    fn world_uncached(&self, id: NodeId) -> Transform3D {
        if let Some(w) = self.world.get(&id) {
            return *w;
        }
        let mut chain = Vec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            match self.nodes.get(&c) {
                Some(n) => {
                    chain.push(n.local_transform);
                    cur = n.parent;
                }
                None => break,
            }
        }
        chain.iter().rev().fold(Transform3D::IDENTITY, |acc, t| acc.compose(t))
    }

    /// World transform. Uses the cache when clean, falls back to walking the chain.
    pub fn world(&self, id: NodeId) -> Transform3D {
        if self.dirty.is_empty() {
            if let Some(w) = self.world.get(&id) {
                return *w;
            }
        }
        let mut chain = Vec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            match self.nodes.get(&c) {
                Some(n) => {
                    chain.push(n.local_transform);
                    cur = n.parent;
                }
                None => break,
            }
        }
        chain.iter().rev().fold(Transform3D::IDENTITY, |acc, t| acc.compose(t))
    }

    /// Nearest ancestor (excluding self) satisfying `pred`.
    pub fn find_ancestor(&self, id: NodeId, mut pred: impl FnMut(NodeId) -> bool) -> Option<NodeId> {
        let mut cur = self.parent(id);
        while let Some(p) = cur {
            if pred(p) {
                return Some(p);
            }
            cur = self.parent(p);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> ObjectId {
        ObjectId(n)
    }

    fn build() -> Scene {
        let mut s = Scene::new();
        s.insert(SceneNode::new(id(1), Transform3D::from_translation(100.0, 0.0, 0.0)), None, None).unwrap();
        s.insert(SceneNode::new(id(2), Transform3D::from_translation(10.0, 0.0, 0.0)), Some(id(1)), None).unwrap();
        s.insert(SceneNode::new(id(3), Transform3D::from_translation(0.0, 5.0, 0.0)), Some(id(2)), None).unwrap();
        s.update_world();
        s
    }

    #[test]
    fn world_transform_and_dirty_propagation() {
        let mut s = build();
        assert_eq!(s.world(id(3)).translation, [110.0, 5.0, 0.0]);
        s.set_local_transform(id(1), Transform3D::from_translation(0.0, 0.0, 0.0)).unwrap();
        let updated = s.update_world();
        assert_eq!(updated.len(), 3, "whole subtree recomputed");
        assert_eq!(s.world(id(3)).translation, [10.0, 5.0, 0.0]);
        // Changing a leaf only updates the leaf.
        s.set_local_transform(id(3), Transform3D::from_translation(0.0, 6.0, 0.0)).unwrap();
        assert_eq!(s.update_world(), vec![id(3)]);
    }

    #[test]
    fn reparent_keeps_world_and_rejects_cycles() {
        let mut s = build();
        assert!(s.reparent(id(1), Some(id(3)), None).is_err());
        s.reparent(id(3), None, None).unwrap();
        s.update_world();
        assert_eq!(s.world(id(3)).translation, [110.0, 5.0, 0.0]);
        assert_eq!(s.roots(), &[id(1), id(3)]);
    }

    #[test]
    fn remove_and_restore_subtree() {
        let mut s = build();
        let removed = s.remove_subtree(id(2)).unwrap();
        assert_eq!(removed.len(), 2);
        assert_eq!(s.len(), 1);
        s.restore_subtree(removed, Some(0)).unwrap();
        s.update_world();
        assert_eq!(s.len(), 3);
        assert_eq!(s.world(id(3)).translation, [110.0, 5.0, 0.0]);
    }

    #[test]
    fn visibility_and_lock_inherit() {
        let mut s = build();
        s.set_visible(id(1), false).unwrap();
        s.set_locked(id(2), true).unwrap();
        assert!(!s.is_effectively_visible(id(3)));
        assert!(s.is_effectively_locked(id(3)));
        assert!(!s.is_effectively_locked(id(1)));
    }
}
