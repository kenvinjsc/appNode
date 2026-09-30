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
    /// Hậu ốp bắt vít: the back covers the whole rear, screwed onto the carcass edges
    /// (sides / top / bottom are shortened by the back thickness).
    #[serde(default)]
    pub overlay: bool,
    /// Hậu chia theo kệ cố định (lapped back only): one back per section, full-width
    /// fixed shelves run through to the rear edge.
    #[serde(default)]
    pub split_at_fixed: bool,
    /// Khoét hậu: openings for sockets, pipes, vents.
    #[serde(default)]
    pub cutouts: Vec<BackCutout>,
}

/// Kiểu lỗ khoét hậu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CutoutKind {
    /// Ổ điện (chữ nhật bo góc).
    #[default]
    Socket,
    /// Ống nước (tròn, đường kính = `w`).
    Pipe,
    /// Thoát nhiệt (khe chữ nhật).
    Vent,
}

/// Neo ngang của lỗ khoét: đo từ mép trái / tâm / mép phải lòng tủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HAnchor {
    Left,
    #[default]
    Center,
    Right,
}

/// Lỗ khoét trên hậu, tọa độ theo lòng tủ (không theo tấm hậu) nên đổi cỡ tủ vẫn đúng chỗ:
/// tâm lỗ cách neo ngang `x` (Left: sang phải, Right: sang trái, Center: + sang phải) và cách
/// mặt trên đáy `y`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BackCutout {
    #[serde(default)]
    pub kind: CutoutKind,
    #[serde(default)]
    pub anchor: HAnchor,
    #[serde(default)]
    pub x: f64,
    pub y: f64,
    pub w: f64,
    #[serde(default)]
    pub h: f64,
    /// Bo góc (Socket / Vent).
    #[serde(default)]
    pub r: f64,
}

impl BackCutout {
    /// Size (w, h) and corner radius of the opening.
    pub fn shape(&self) -> (f64, f64, f64) {
        match self.kind {
            CutoutKind::Pipe => (self.w, self.w, self.w / 2.0),
            _ => {
                let h = if self.h > 0.0 { self.h } else { self.w };
                (self.w, h, self.r.clamp(0.0, self.w.min(h) / 2.0))
            }
        }
    }
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

/// Loại tay nắm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HandleType {
    /// Tay nắm thanh (2 lỗ theo bước lỗ).
    #[default]
    Bar,
    /// Núm (1 lỗ).
    Knob,
    /// Nhấn mở (push-open): không tay nắm, 1 bộ / cánh ≤ 1200, 2 bộ nếu cao hơn.
    PushOpen,
    /// Không tay nắm (tay nắm âm / cánh vát).
    None,
}

/// Loại ray ngăn kéo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SlideType {
    /// Ray bi 3 tầng: hở hông theo ngăn kéo (mặc định 13).
    #[default]
    Ball,
    /// Ray âm giảm chấn: hở hông 5, hộc ngắn hơn ray 10, đáy nâng 12.
    Undermount,
    /// Hộp kim loại (tandem box): chỉ cắt đáy + hậu hộc.
    Tandem,
}

/// Kiểu liên kết thùng (hồi ↔ nóc / đáy / kệ cố định / vách).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JointType {
    /// Chỉ chốt gỗ (hành vi cũ).
    #[default]
    Dowel,
    /// Cam (minifix) ở hai đầu + chốt gỗ ở giữa.
    CamDowel,
    /// Vít xuyên: lỗ Ø5 xuyên mặt + lỗ mồi Ø3 trên cạnh.
    Screw,
    /// Ke góc: không khoan, đếm ke.
    Bracket,
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
    handle_type: HandleType = HandleType::Bar,
    handle_pos: HandlePos = HandlePos::Auto,
    handle_pitch: f64 = 128.0,
    handle_drill: bool = false,
    hinge_plate: bool = false,
    handle_from_end: f64 = 60.0,
    slide_overlap: f64 = 30.0,
    // Ngăn kéo
    box_top_gap: f64 = 40.0,
    box_bottom_gap: f64 = 15.0,
    box_min: f64 = 60.0,
    box_max: f64 = 250.0,
    slide_margin: f64 = 10.0,
    slide_type: SlideType = SlideType::Ball,
    // Liên kết thùng
    joint_type: JointType = JointType::Dowel,
    dowel_d: f64 = 8.0,
    dowel_face_depth: f64 = 12.0,
    dowel_edge_depth: f64 = 25.0,
    joint_end: f64 = 50.0,
    joint_pitch: f64 = 300.0,
    cam_d: f64 = 15.0,
    cam_depth: f64 = 12.5,
    cam_offset: f64 = 34.0,
    bolt_d: f64 = 8.0,
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

/// Chân tủ / cách đặt tủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BaseType {
    /// Như cũ: tủ áo / bếp dưới / ngăn kéo có len chân trước khi `plinth_height` > 0.
    #[default]
    Auto,
    /// Không chân.
    None,
    /// Len chân trước.
    Plinth,
    /// Len chân 3 mặt (trước + 2 hông), cho tủ đầu dãy / đứng độc lập.
    Plinth3,
    /// Chân nhựa tăng chỉnh.
    Legs,
    /// Chân nhựa + len chân kẹp phía trước.
    LegsPlinth,
    /// Tủ treo: ke treo (+ thanh treo tường).
    Hanging,
}

/// Cách báo giá một tủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PricingMode {
    /// Theo loại tủ: bếp (dưới / trên / ngăn kéo) → mét dài, còn lại → m² mặt đứng.
    #[default]
    Auto,
    /// Bóc chi tiết: vật tư × (1 + hao hụt) × (1 + công).
    Detail,
    /// Mét dài (rộng tủ).
    LinearM,
    /// m² mặt đứng (rộng × cao).
    FacadeM2,
}

/// Tủ góc chéo: hai cạnh áp tường dài = rộng tủ, sâu tay = sâu tủ, mặt cánh xiên 45°.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiagonalCorner {
    /// Số kệ cố định 5 cạnh.
    pub shelves: u32,
    /// Bản lề phía trái (nhìn từ trước mặt cánh), ngược lại phải.
    pub hinge_left: bool,
    /// Kệ lùi so với mặt cánh.
    pub shelf_setback: f64,
}

impl Default for DiagonalCorner {
    fn default() -> Self {
        Self { shelves: 1, hinge_left: true, shelf_setback: 20.0 }
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
    /// Chân tủ.
    #[serde(default)]
    pub base_type: BaseType,
    /// Len hông lùi vào so với mặt ngoài hồi (len 3 mặt).
    #[serde(default)]
    pub plinth_side_setback: f64,
    /// Số chân (0 = tự động theo rộng: ≤ 600 → 4, ≤ 1200 → 6, còn lại 8).
    #[serde(default)]
    pub leg_count: u32,
    /// Tủ treo: thanh treo tường 17 × 60 sau hậu.
    #[serde(default)]
    pub hang_rail: bool,
    /// Cách báo giá tủ.
    #[serde(default)]
    pub pricing: PricingMode,
    /// Tủ góc chéo (None = tủ hộp thường).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal: Option<DiagonalCorner>,
    /// Chuẩn xưởng (kệ, chốt, bản lề, tay nắm, ngăn kéo).
    #[serde(default)]
    pub shop: ShopRules,
    /// Phào & ốp (D13).
    #[serde(default)]
    pub trim: TrimRules,
}

/// Mặt có phào nóc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CorniceSides {
    #[default]
    None,
    Front,
    FrontLeft,
    FrontRight,
    #[serde(rename = "3_SIDES")]
    ThreeSides,
}

impl CorniceSides {
    /// (trước, trái, phải)
    pub fn sides(self) -> (bool, bool, bool) {
        match self {
            CorniceSides::None => (false, false, false),
            CorniceSides::Front => (true, false, false),
            CorniceSides::FrontLeft => (true, true, false),
            CorniceSides::FrontRight => (true, false, true),
            CorniceSides::ThreeSides => (true, true, true),
        }
    }
}

/// Tab "Phào & ốp": phào nóc, phào chân, ốp hông, nẹp.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrimRules {
    /// Phào nóc: các mặt có phào.
    pub cornice: CorniceSides,
    /// Cao phào nóc (20–200).
    pub cornice_h: f64,
    /// Nhô ra khỏi mặt tủ.
    pub cornice_overhang: f64,
    /// Góc nối vát 45° (false = nối bằng).
    pub cornice_miter: bool,
    /// Phào chân (thanh dưới đáy, cùng các mặt với phào nóc; cao 0 = không).
    pub skirting_h: f64,
    /// Ốp hông trái / phải.
    pub end_left: bool,
    pub end_right: bool,
    /// Dày ốp hông (0 = theo ván thùng).
    pub end_t: f64,
    /// Ốp hông nhô trước (0–20).
    pub end_front: f64,
    /// Ốp hông chạm sàn.
    pub end_to_floor: bool,
    /// Nẹp che khe (rộng 30–60, 0 = không) bên trái / phải, sát mặt trước.
    pub scribe_left: f64,
    pub scribe_right: f64,
}

impl Default for TrimRules {
    fn default() -> Self {
        Self {
            cornice: CorniceSides::None,
            cornice_h: 60.0,
            cornice_overhang: 20.0,
            cornice_miter: true,
            skirting_h: 0.0,
            end_left: false,
            end_right: false,
            end_t: 0.0,
            end_front: 0.0,
            end_to_floor: true,
            scribe_left: 0.0,
            scribe_right: 0.0,
        }
    }
}

fn plinth_setback() -> f64 {
    50.0
}

impl Default for StructureRules {
    fn default() -> Self {
        Self { back: BackRule::default(), top_rails: TopRails::default(), plinth_setback: plinth_setback(), base_type: BaseType::Auto, plinth_side_setback: 0.0, leg_count: 0, hang_rail: false, pricing: PricingMode::Auto, diagonal: None, shop: ShopRules::default(), trim: TrimRules::default() }
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
