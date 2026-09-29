use aic_domain::{DomainError, ObjectId};
use aic_parametric::ParamError;
use serde::Serialize;

/// Errors reported by the core. The UI maps `code()` to a localised message;
/// raw technical text is never shown to end users.
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoreError {
    #[error("object {id} not found")]
    NotFound { id: ObjectId },
    #[error("object {id} is locked")]
    Locked { id: ObjectId },
    #[error("invalid parameter {name}: {reason}")]
    InvalidParameter { name: String, reason: String },
    #[error("dependency cycle: {}", path.join(" -> "))]
    DependencyCycle { path: Vec<String> },
    #[error("constraint {constraint} violated: {message}")]
    ConstraintViolated { constraint: String, message: String },
    #[error("invalid transform")]
    InvalidTransform,
    #[error("invalid reparent")]
    InvalidReparent,
    #[error("geometry boolean failed: {reason}")]
    GeometryBooleanFailed { reason: String },
    #[error("invalid feature: {reason}")]
    InvalidFeature { reason: String },
    #[error("unknown material {material}")]
    UnknownMaterial { material: String },
    #[error("unsupported project version {version}")]
    UnsupportedVersion { version: u64 },
    #[error("invalid project file: {reason}")]
    InvalidProject { reason: String },
    #[error("nothing to {action}")]
    NothingTo { action: String },
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::NotFound { .. } => "NOT_FOUND",
            CoreError::Locked { .. } => "LOCKED",
            CoreError::InvalidParameter { .. } => "INVALID_PARAMETER",
            CoreError::DependencyCycle { .. } => "DEPENDENCY_CYCLE",
            CoreError::ConstraintViolated { .. } => "CONSTRAINT_VIOLATED",
            CoreError::InvalidTransform => "INVALID_TRANSFORM",
            CoreError::InvalidReparent => "INVALID_REPARENT",
            CoreError::GeometryBooleanFailed { .. } => "GEOMETRY_BOOLEAN_FAILED",
            CoreError::InvalidFeature { .. } => "INVALID_FEATURE",
            CoreError::UnknownMaterial { .. } => "UNKNOWN_MATERIAL",
            CoreError::UnsupportedVersion { .. } => "UNSUPPORTED_VERSION",
            CoreError::InvalidProject { .. } => "INVALID_PROJECT",
            CoreError::NothingTo { .. } => "NOTHING_TO",
        }
    }

    pub fn from_param(name: &str, e: ParamError) -> Self {
        match e {
            ParamError::Cycle { path } => CoreError::DependencyCycle { path },
            ParamError::Constraint { code, message } => CoreError::ConstraintViolated { constraint: code, message },
            ParamError::Parse(p) => CoreError::InvalidParameter { name: name.into(), reason: p.to_string() },
            ParamError::Unresolved(r) => CoreError::InvalidParameter { name: name.into(), reason: format!("unknown reference '{r}'") },
            ParamError::Eval { key, message } => CoreError::InvalidParameter { name: key, reason: message },
        }
    }
}

impl From<DomainError> for CoreError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::NotFound(id) => CoreError::NotFound { id },
            DomainError::Locked(id) => CoreError::Locked { id },
            DomainError::InvalidReparent { .. } => CoreError::InvalidReparent,
            DomainError::InvalidTransform => CoreError::InvalidTransform,
            DomainError::InvalidParameter { name, reason } => CoreError::InvalidParameter { name, reason },
        }
    }
}
