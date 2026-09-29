use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable identifier of a domain object. Stable for the lifetime of a project
/// (and therefore within a session), also used as render/selection id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub u64);

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// In the MVP every scene node carries exactly one domain object and shares its id.
pub type NodeId = ObjectId;

/// Material catalogue code, e.g. `MDF18-WHITE`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MaterialId(pub String);

impl MaterialId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for MaterialId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdAllocator {
    next: u64,
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl IdAllocator {
    pub fn alloc(&mut self) -> ObjectId {
        let id = ObjectId(self.next);
        self.next += 1;
        id
    }

    /// Make sure future ids never collide with `id` (used when restoring/loading).
    pub fn reserve(&mut self, id: ObjectId) {
        if id.0 >= self.next {
            self.next = id.0 + 1;
        }
    }
}
