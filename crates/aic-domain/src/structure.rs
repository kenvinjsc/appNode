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

/// Hàng lỗ chốt tầng của kệ di động.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PinRow {
    /// Chỉ khoan 4 lỗ đúng vị trí kệ.
    #[default]
    AtShelf,
    /// Hàng lỗ hệ 32 suốt khoang (kệ bắt vào lỗ gần nhất).
    #[serde(rename = "ROW_32")]
    Row32,
}

/// Vị trí tay nắm trên cánh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HandlePos {
    /// Theo loại tủ: bếp dưới / ngăn kéo → trên, bếp trên → dưới, còn lại → giữa.
    #[default]
    Auto,
    Center,
    Top,
    Bottom,
}

macro_rules! shop_rules {
    ($( $field:ident : $ty:ty = $default:expr ),* $(,)?) => {
        /// Chuẩn xưởng: các giá trị sản xuất trước đây viết cứng trong generator.
        /// Mặc định = giá trị cũ, nên tủ lưu trước đó giữ nguyên hình.
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct ShopRules {
            $( pub $field: $ty, )*
        }
        impl Default for ShopRules {
            fn default() -> Self {
                Self { $( $field: $default, )* }
            }
        }
    };
}

shop_rules! {
    // Kệ & chốt tầng
    shelf_clear: f64 = 0.5,
    pin_row: PinRow = PinRow::AtShelf,
    pin_edge: f64 = 37.0,
    pin_edge_back: f64 = 37.0,
    pin_below: f64 = 5.0,
    pin_d: f64 = 5.0,
    pin_depth: f64 = 10.0,
    pin_pitch: f64 = 32.0,
    pin_start: f64 = 64.0,
    pin_end: f64 = 64.0,
    pin_snap: bool = true,
    // Cánh, bản lề, tay nắm
    hinge_table: String = "900=2, 1600=3, 4".into(),
    hinge_edge: f64 = 22.5,
    hinge_end: f64 = 100.0,
    cup_d: f64 = 35.0,
    cup_depth: f64 = 13.0,
    handle_len: f64 = 160.0,
    handle_edge: f64 = 40.0,
    handle_pos: HandlePos = HandlePos::Auto,
    handle_from_end: f64 = 60.0,
    slide_overlap: f64 = 30.0,
    // Ngăn kéo
    box_top_gap: f64 = 40.0,
    box_bottom_gap: f64 = 15.0,
    box_min: f64 = 60.0,
    box_max: f64 = 250.0,
    slide_margin: f64 = 10.0,
}

impl ShopRules {
    /// Số bản lề cho cánh dài `along` theo bảng `900=2, 1600=3, 4`
    /// (≤ 900 → 2, ≤ 1600 → 3, còn lại → 4).
    pub fn hinge_count(&self, along: f64) -> u32 {
        let mut fallback = 2u32;
        let mut steps: Vec<(f64, u32)> = Vec::new();
        for tok in self.hinge_table.split([',', ';']).map(str::trim).filter(|t| !t.is_empty()) {
            match tok.split_once('=') {
                Some((h, n)) => {
                    if let (Ok(h), Ok(n)) = (h.trim().parse::<f64>(), n.trim().parse::<u32>()) {
                        steps.push((h, n.clamp(1, 10)));
                    }
                }
                None => {
                    if let Ok(n) = tok.parse::<u32>() {
                        fallback = n.clamp(1, 10);
                    }
                }
            }
        }
        steps.sort_by(|a, b| a.0.total_cmp(&b.0));
        steps.iter().find(|(h, _)| along <= *h).map(|s| s.1).unwrap_or(fallback)
    }

    /// Tâm các chén bản lề dọc cạnh dài `along`.
    pub fn hinge_positions(&self, along: f64) -> Vec<f64> {
        let n = self.hinge_count(along).max(1);
        let inset = self.hinge_end.min(along / 4.0);
        if n == 1 {
            return vec![along / 2.0];
        }
        (0..n).map(|i| inset + (along - 2.0 * inset) * i as f64 / (n - 1) as f64).collect()
    }

    /// Cao độ các lỗ hàng chốt 32 trong khoảng [lo, hi] (tọa độ khoang).
    pub fn pin_grid(&self, lo: f64, hi: f64) -> Vec<f64> {
        let p = self.pin_pitch.max(8.0);
        let (a, b) = (lo + self.pin_start, hi - self.pin_end);
        if b < a {
            return Vec::new();
        }
        let n = ((b - a) / p).floor() as usize;
        (0..=n.min(500)).map(|i| a + i as f64 * p).collect()
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
    /// Chuẩn xưởng (kệ, chốt, bản lề, tay nắm, ngăn kéo).
    #[serde(default)]
    pub shop: ShopRules,
}

fn plinth_setback() -> f64 {
    50.0
}

impl Default for StructureRules {
    fn default() -> Self {
        Self { back: BackRule::default(), top_rails: TopRails::default(), plinth_setback: plinth_setback(), shop: ShopRules::default() }
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

    #[test]
    fn hinge_table_and_pin_grid() {
        let mut s = ShopRules::default();
        assert_eq!((s.hinge_count(800.0), s.hinge_count(1200.0), s.hinge_count(2000.0)), (2, 3, 4));
        s.hinge_table = "900=2,1600=3,2000=4,5".into();
        assert_eq!(s.hinge_count(2300.0), 5);
        assert_eq!(s.hinge_positions(700.0), vec![100.0, 600.0]);
        let g = s.pin_grid(0.0, 400.0);
        assert_eq!(g.first(), Some(&64.0));
        assert!(g.windows(2).all(|w| (w[1] - w[0] - 32.0).abs() < 1e-9));
        assert!(*g.last().unwrap() <= 336.0);
    }
}
