use crate::ObjectId;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DomainError {
    #[error("object {0} not found")]
    NotFound(ObjectId),
    #[error("object {0} is locked")]
    Locked(ObjectId),
    #[error("invalid reparent of {child} under {parent}")]
    InvalidReparent { child: ObjectId, parent: ObjectId },
    #[error("invalid transform")]
    InvalidTransform,
    #[error("invalid parameter {name}: {reason}")]
    InvalidParameter { name: String, reason: String },
}
