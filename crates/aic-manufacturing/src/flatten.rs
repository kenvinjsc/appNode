use crate::features::{DerivedFeature, FeatureOrigin};
use aic_domain::{DrillPurpose, EdgeBand, GrainDirection, MachiningFeature, MaterialId, ObjectId, Panel, PanelRole};
use aic_math::Polygon2D;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub drills: usize,
    pub edge_drills: usize,
    pub pockets: usize,
    pub grooves: usize,
    pub contours: usize,
    pub edge_bands: Vec<String>,
}

/// A panel projected into its local manufacturing coordinate system (XY, mm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlatPanel {
    pub id: ObjectId,
    pub name: String,
    pub role: PanelRole,
    pub material_id: MaterialId,
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    pub grain: GrainDirection,
    pub outer: Polygon2D,
    pub inner: Vec<Polygon2D>,
    pub features: Vec<DerivedFeature>,
    pub edge_bands: Vec<EdgeBand>,
    pub summary: FeatureSummary,
}

/// Flatten a panel: outer contour, inner contours and all features in local XY.
pub fn flatten(panel: &Panel, derived: Vec<DerivedFeature>) -> FlatPanel {
    let mut features: Vec<DerivedFeature> = panel
        .features
        .iter()
        .enumerate()
        .map(|(i, f)| DerivedFeature { feature: f.clone(), origin: FeatureOrigin::User { index: i } })
        .collect();
    features.extend(derived);

    let mut outer = Polygon2D::rect(0.0, 0.0, panel.width_mm, panel.height_mm);
    let mut inner = Vec::new();
    let mut summary = FeatureSummary::default();
    for f in &features {
        match &f.feature {
            MachiningFeature::Drill(_) => summary.drills += 1,
            MachiningFeature::EdgeDrill(_) => summary.edge_drills += 1,
            MachiningFeature::Pocket(_) => summary.pockets += 1,
            MachiningFeature::Groove(_) => summary.grooves += 1,
            MachiningFeature::Contour(c) => {
                summary.contours += 1;
                if c.inner {
                    inner.push(c.polygon.clone().ensure_ccw());
                } else {
                    outer = c.polygon.clone().ensure_ccw();
                }
            }
        }
    }
    summary.edge_bands = panel.edge_bands.iter().map(|b| format!("{:?}", b.edge).to_uppercase()).collect();
    FlatPanel {
        id: panel.id,
        name: panel.name.clone(),
        role: panel.role,
        material_id: panel.material_id.clone(),
        width: panel.width_mm,
        height: panel.height_mm,
        thickness: panel.thickness_mm,
        grain: panel.grain_direction,
        outer,
        inner,
        features,
        edge_bands: panel.edge_bands.clone(),
        summary,
    }
}

impl FlatPanel {
    pub fn count_purpose(&self, p: DrillPurpose) -> usize {
        self.features
            .iter()
            .filter(|f| match &f.feature {
                MachiningFeature::Drill(d) => d.purpose == p,
                MachiningFeature::EdgeDrill(d) => d.purpose == p,
                _ => false,
            })
            .count()
    }
}
