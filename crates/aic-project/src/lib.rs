//! Project model: the document (scene + domain objects + parameters), the
//! undoable command set and the versioned project file format.

mod command;
mod document;
mod error;
mod file;
mod history;

pub use command::{Command, Snapshot};
pub use document::{default_prices, CabinetTemplate, ChangeSet, Document, ProjectMeta, ProjectSettings, RulePreset, RunDef, ScrewRule, STRUCTURAL_PARAMS};
pub use error::CoreError;
pub use file::{migrate, ProjectFile, FORMAT, VERSION};
pub use history::History;
