//! Sản phẩm ngoài tủ hộp (D19+): giường, bàn, vách ốp … dùng chung hạ tầng của tủ
//! (tham số W/H/D, part key, PartMod, template, undo). Một `Cabinet` có
//! `rules.product = Some(..)` được dựng bằng generator của sản phẩm thay cho thùng tủ.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Product {
    Bed(BedSpec),
    Desk(DeskSpec),
}

impl Product {
    /// Mã loại (báo giá `quote:piece:<CODE>`).
    pub fn code(&self) -> &'static str {
        match self {
            Product::Bed(_) => "BED",
            Product::Desk(_) => "DESK",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Product::Bed(_) => "Giường",
            Product::Desk(_) => "Bàn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HeadboardStyle {
    #[default]
    Flat,
    /// Bọc nệm: tấm đệm phía trước đầu giường.
    Upholstered,
    /// Nan đứng.
    Slatted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SlatKind {
    /// Dát tấm liền.
    Board,
    /// Dát nan.
    #[default]
    Slats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedStorage {
    #[default]
    None,
    /// Hộc kéo 2 bên.
    #[serde(rename = "DRAWERS_2_SIDES")]
    DrawersTwoSides,
    /// Hộc kéo phía đuôi.
    DrawersFoot,
    /// Nâng hơi (ben hơi, dát tấm lật).
    GasLift,
}

/// Giường. Lọt nệm = W (rộng tủ) × D (sâu tủ); cao tủ = cao đầu giường.
/// Đầu giường ở phía sau (z = 0), đuôi ở phía trước.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BedSpec {
    /// Cao mặt dát (350–450).
    pub frame_h: f64,
    /// Dày đầu giường (0 = theo ván).
    pub headboard_t: f64,
    pub headboard_style: HeadboardStyle,
    /// Cao vai / đuôi (200–300).
    pub side_rail_h: f64,
    /// Vai cao hơn mặt dát (giữ nệm).
    pub lip: f64,
    pub slats: SlatKind,
    /// Số nan (12–18).
    pub slat_count: u32,
    pub slat_t: f64,
    /// Đà giữa (tự bật khi rộng ≥ 1400).
    pub center_beam: bool,
    /// 4 / 6 chân.
    pub legs: u32,
    pub storage: BedStorage,
    /// Số hộc mỗi bên (2–4).
    pub drawer_count: u32,
}

impl Default for BedSpec {
    fn default() -> Self {
        Self {
            frame_h: 400.0,
            headboard_t: 0.0,
            headboard_style: HeadboardStyle::Flat,
            side_rail_h: 250.0,
            lip: 50.0,
            slats: SlatKind::Slats,
            slat_count: 14,
            slat_t: 12.0,
            center_beam: true,
            legs: 6,
            storage: BedStorage::None,
            drawer_count: 2,
        }
    }
}

/// Đỡ mặt bàn một bên.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeskSupport {
    /// Chân tấm.
    #[default]
    Panel,
    /// Hộc tủ ngăn kéo.
    DrawerUnit,
    /// Chân sắt (2 chân).
    Leg,
}

/// Bàn học / làm việc / trang điểm: W × H × D của bàn (H = cao mặt bàn).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeskSpec {
    /// Dày mặt bàn (17–40).
    pub top_t: f64,
    /// Mặt bàn nhô hai bên (0–50).
    pub top_overhang: f64,
    pub support_left: DeskSupport,
    pub support_right: DeskSupport,
    /// Rộng hộc tủ (LOCK, không đổi theo rộng bàn).
    pub unit_w: f64,
    /// Số ngăn kéo mỗi hộc.
    pub unit_drawers: u32,
    /// Yếm.
    pub modesty: bool,
    pub modesty_h: f64,
    /// Yếm lùi từ mép sau.
    pub modesty_setback: f64,
    /// Hộc bàn phím.
    pub keyboard_tray: bool,
    /// Kệ trên (0 = không): cao, số kệ, sâu.
    pub hutch_h: f64,
    pub hutch_shelves: u32,
    pub hutch_d: f64,
    /// Khoét luồn dây Ø (0 = không), tâm cách mép phải / mép sau.
    pub cable_d: f64,
    pub cable_x: f64,
    pub cable_y: f64,
    /// Gương (bàn trang điểm): rộng × cao.
    pub mirror_w: f64,
    pub mirror_h: f64,
}

impl Default for DeskSpec {
    fn default() -> Self {
        Self {
            top_t: 25.0,
            top_overhang: 0.0,
            support_left: DeskSupport::Panel,
            support_right: DeskSupport::DrawerUnit,
            unit_w: 400.0,
            unit_drawers: 3,
            modesty: true,
            modesty_h: 300.0,
            modesty_setback: 50.0,
            keyboard_tray: false,
            hutch_h: 0.0,
            hutch_shelves: 2,
            hutch_d: 250.0,
            cable_d: 60.0,
            cable_x: 150.0,
            cable_y: 60.0,
            mirror_w: 0.0,
            mirror_h: 800.0,
        }
    }
}
