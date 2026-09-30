//! Thuộc tính kết cấu of a cabinet (tabs Hậu, Thanh giằng, Len chân …). Each value
//! is optional-with-default so cabinets saved before this module keep their geometry.

use serde::{Deserialize, Serialize};

/// Tab Hậu. Thickness (B), groove depth (C) and setback (I) are cabinet parameters
/// (`back_thickness`, `back_groove`, `back_offset`); these are the other options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BackRule {
    /// Khe hở: clearance between the back's edge and the bottom of the groove.
    #[serde(default)]
    pub clearance: f64,
    /// Hở trái / phải / trên / dưới: the back is reduced on that side.
    #[serde(default)]
    pub gaps: [f64; 4],
    /// Chia dọc: split the back into vertical pieces.
    #[serde(default)]
    pub split: bool,
    /// Công thức chia: max piece width in mm ("600"), or "n×" for a count ("3x").
    #[serde(default)]
    pub split_formula: String,
    /// Nóc trùm hậu: the top covers the back (runs to the rear edge). None = legacy.
    #[serde(default)]
    pub top_covers: Option<bool>,
    /// Đáy trùm hậu.
    #[serde(default)]
    pub bottom_covers: Option<bool>,
}

impl BackRule {
    /// Widths of the back pieces for a back `w` wide.
    pub fn pieces(&self, w: f64) -> Vec<f64> {
        if !self.split || w <= 0.0 {
            return vec![w];
        }
        let f = self.split_formula.trim().to_lowercase().replace(',', ".");
        let n = if let Some(c) = f.strip_suffix('x').or_else(|| f.strip_suffix('×')) {
            c.trim().parse::<f64>().ok().map(|c| c.round().max(1.0) as usize)
        } else {
            f.parse::<f64>().ok().filter(|m| *m > 1.0).map(|m| (w / m).ceil().max(1.0) as usize)
        }
        .unwrap_or(1)
        .min(50);
        vec![w / n as f64; n]
    }
}

/// One set of top rails (thanh bạ nóc).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RailSet {
    /// Số thanh.
    pub count: u32,
    /// Kích thước (rail width, mm).
    pub size: f64,
    /// Ngang: lying flat (true) or standing on edge (false).
    pub horizontal: bool,
    /// Front set: âm mặt (≤ 0 = set back from the front face). Back set: cách hậu.
    #[serde(default)]
    pub offset: f64,
}

/// Tab Thanh giằng (trên): used when the top style is RAILS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopRails {
    pub front: RailSet,
    pub back: RailSet,
    /// Thanh bổ sung, spread evenly between the front and back rails.
    pub extra: RailSet,
}

impl Default for TopRails {
    fn default() -> Self {
        // Legacy: one flat rail at the front and one at the back, width = `rail_width`
        // (size 0 = use the cabinet parameter).
        Self {
            front: RailSet { count: 1, size: 0.0, horizontal: true, offset: 0.0 },
            back: RailSet { count: 1, size: 0.0, horizontal: true, offset: 0.0 },
            extra: RailSet { count: 0, size: 50.0, horizontal: true, offset: 0.0 },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructureRules {
    #[serde(default)]
    pub back: BackRule,
    #[serde(default)]
    pub top_rails: TopRails,
    /// Len chân: set-back of the plinth board from the front (mm).
    #[serde(default = "plinth_setback")]
    pub plinth_setback: f64,
}

fn plinth_setback() -> f64 {
    50.0
}

impl Default for StructureRules {
    fn default() -> Self {
        Self { back: BackRule::default(), top_rails: TopRails::default(), plinth_setback: plinth_setback() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_pieces_from_formula() {
        let mut b = BackRule { split: true, split_formula: "600".into(), ..Default::default() };
        assert_eq!(b.pieces(1565.6).len(), 3);
        b.split_formula = "2x".into();
        assert_eq!(b.pieces(1000.0), vec![500.0, 500.0]);
        b.split = false;
        assert_eq!(b.pieces(1000.0), vec![1000.0]);
    }
}
