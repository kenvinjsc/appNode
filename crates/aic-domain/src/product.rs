//! Sản phẩm ngoài tủ hộp (D19+): giường, bàn, vách ốp … dùng chung hạ tầng của tủ
//! (tham số W/H/D, part key, PartMod, template, undo). Một `Cabinet` có
//! `rules.product = Some(..)` được dựng bằng generator của sản phẩm thay cho thùng tủ.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Product {
    Bed(BedSpec),
}

impl Product {
    /// Mã loại (báo giá `quote:piece:<CODE>`).
    pub fn code(&self) -> &'static str {
        match self {
            Product::Bed(_) => "BED",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Product::Bed(_) => "Giường",
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
