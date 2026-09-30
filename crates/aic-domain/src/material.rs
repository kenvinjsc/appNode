use crate::MaterialId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MaterialKind {
    Mdf,
    Plywood,
    Particleboard,
    Hdf,
    SolidWood,
    /// Đá (mặt bếp).
    Stone,
}

/// Logical board material. `color`/`texture` are *render properties only* and are
/// never used to decide anything about manufacturing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    pub kind: MaterialKind,
    pub thickness_mm: f64,
    /// Whether the board has a visible grain that constrains part rotation.
    pub has_grain: bool,
    pub sheet_width_mm: f64,
    pub sheet_height_mm: f64,
    pub color: String,
    #[serde(default)]
    pub texture: Option<String>,
}

fn mat(code: &str, name: &str, kind: MaterialKind, t: f64, grain: bool, color: &str) -> Material {
    Material {
        id: MaterialId::new(code),
        name: name.into(),
        kind,
        thickness_mm: t,
        has_grain: grain,
        sheet_width_mm: 2440.0,
        sheet_height_mm: 1220.0,
        color: color.into(),
        texture: None,
    }
}

pub fn default_materials() -> Vec<Material> {
    use MaterialKind::*;
    vec![
        mat("MDF17-WHITE", "MDF trắng 17", Mdf, 17.2, false, "#f2f0eb"),
        mat("MDF17-OAK", "MDF vân sồi 17", Mdf, 17.2, true, "#c8a27a"),
        mat("MDF17-WALNUT", "MDF óc chó 17", Mdf, 17.2, true, "#7a5237"),
        mat("MDF8-WHITE", "MDF hậu 8", Mdf, 8.6, false, "#ebe8e1"),
        mat("MDF18-WHITE", "MDF trắng 18", Mdf, 18.0, false, "#f2f0eb"),
        mat("MDF18-OAK", "MDF vân sồi 18", Mdf, 18.0, true, "#c8a27a"),
        mat("MDF25-GREY", "MDF xám 25", Mdf, 25.0, false, "#9ea3a8"),
        mat("PLY18-BIRCH", "Plywood bạch dương 18", Plywood, 18.0, true, "#e3c99a"),
        mat("PB18-WALNUT", "Ván dăm óc chó 18", Particleboard, 18.0, true, "#7a5237"),
        mat("PB25-WHITE", "Ván dăm trắng 25", Particleboard, 25.0, false, "#f5f5f2"),
        mat("HDF5-WHITE", "HDF hậu 5", Hdf, 5.0, false, "#e9e7e1"),
        mat("STONE20-WHITE", "Đá thạch anh trắng 20", Stone, 20.0, false, "#e8e6e1"),
        mat("STONE20-BLACK", "Đá granite đen 20", Stone, 20.0, false, "#2f3136"),
    ]
}
