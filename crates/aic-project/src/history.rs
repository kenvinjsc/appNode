use crate::{Command, CoreError, Document};

/// Undo/redo stacks of *inverse commands* (no whole-project snapshots).
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<(String, Command)>,
    redo: Vec<(String, Command)>,
    pub limit: usize,
}

impl History {
    pub fn new() -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), limit: 500 }
    }

    pub fn execute(&mut self, doc: &mut Document, cmd: Command) -> Result<(), CoreError> {
        let label = cmd.label().to_string();
        let inverse = cmd.execute(doc)?;
        self.undo.push((label, inverse));
        if self.undo.len() > self.limit.max(1) {
            self.undo.remove(0);
        }
        self.redo.clear();
        Ok(())
    }

    /// Position to group the commands executed after it (see `squash`, `rollback`).
    pub fn mark(&self) -> usize {
        self.undo.len()
    }

    /// Merge every step since `mark` into one undo step (multi-edit = one Ctrl+Z).
    pub fn squash(&mut self, mark: usize, label: &str) {
        if self.undo.len() <= mark + 1 {
            return;
        }
        let steps: Vec<Command> = self.undo.drain(mark..).map(|(_, inv)| inv).rev().collect();
        self.undo.push((label.to_string(), Command::Batch { label: label.to_string(), commands: steps }));
    }

    /// Undo every step since `mark` without keeping them for redo (failed group).
    pub fn rollback(&mut self, doc: &mut Document, mark: usize) {
        while self.undo.len() > mark {
            if let Some((_, inv)) = self.undo.pop() {
                let _ = inv.execute(doc);
            }
        }
    }

    pub fn undo(&mut self, doc: &mut Document) -> Result<String, CoreError> {
        let (label, inv) = self.undo.pop().ok_or(CoreError::NothingTo { action: "undo".into() })?;
        match inv.clone().execute(doc) {
            Ok(redo) => {
                self.redo.push((label.clone(), redo));
                Ok(label)
            }
            Err(e) => {
                self.undo.push((label, inv));
                Err(e)
            }
        }
    }

    pub fn redo(&mut self, doc: &mut Document) -> Result<String, CoreError> {
        let (label, cmd) = self.redo.pop().ok_or(CoreError::NothingTo { action: "redo".into() })?;
        match cmd.clone().execute(doc) {
            Ok(undo) => {
                self.undo.push((label.clone(), undo));
                Ok(label)
            }
            Err(e) => {
                self.redo.push((label, cmd));
                Err(e)
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|u| u.0.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|u| u.0.as_str())
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
