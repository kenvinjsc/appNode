//! 2D polygon operations backed by Clipper2 (offset, boolean, cleanup).
//! Never mesh booleans for 2D CNC work.

use aic_math::{Point2, Polygon2D};
use clipper2::{EndType, JoinType, Paths};

fn to_paths(polys: &[Polygon2D]) -> Paths {
    let v: Vec<Vec<(f64, f64)>> = polys.iter().map(|p| p.points.iter().map(|q| (q.x, q.y)).collect()).collect();
    v.into()
}

fn from_paths(p: Paths) -> Vec<Polygon2D> {
    p.iter()
        .map(|path| Polygon2D::new(path.iter().map(|pt| Point2::new(pt.x(), pt.y())).collect()))
        .filter(|p| p.points.len() >= 3)
        .collect()
}

/// Offset polygons by `delta` (positive grows). Miter joins keep rectangles rectangular.
pub fn offset(polys: &[Polygon2D], delta: f64) -> Vec<Polygon2D> {
    if polys.is_empty() {
        return Vec::new();
    }
    from_paths(to_paths(polys).inflate(delta, JoinType::Miter, EndType::Polygon, 4.0))
}

/// Tool-radius compensated path for cutting *outside* a part contour (kerf compensation).
pub fn outside_toolpath(contour: &Polygon2D, tool_diameter: f64) -> Vec<Polygon2D> {
    offset(std::slice::from_ref(contour), tool_diameter / 2.0)
}

/// Tool centre boundary for clearing *inside* a pocket.
pub fn inside_toolpath(contour: &Polygon2D, tool_diameter: f64) -> Vec<Polygon2D> {
    offset(std::slice::from_ref(contour), -tool_diameter / 2.0)
}

pub fn union(a: &[Polygon2D], b: &[Polygon2D]) -> Vec<Polygon2D> {
    from_paths(clipper2::union(to_paths(a), to_paths(b), clipper2::FillRule::NonZero).unwrap_or_default())
}

pub fn difference(a: &[Polygon2D], b: &[Polygon2D]) -> Vec<Polygon2D> {
    from_paths(clipper2::difference(to_paths(a), to_paths(b), clipper2::FillRule::NonZero).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kerf_offset_of_rect() {
        let r = Polygon2D::rect(0.0, 0.0, 600.0, 500.0);
        let o = outside_toolpath(&r, 12.0);
        assert_eq!(o.len(), 1);
        assert!((o[0].area() - 612.0 * 512.0).abs() < 1.0);
        let i = inside_toolpath(&r, 12.0);
        assert!((i[0].area() - 588.0 * 488.0).abs() < 1.0);
    }

    #[test]
    fn booleans() {
        let a = Polygon2D::rect(0.0, 0.0, 10.0, 10.0);
        let b = Polygon2D::rect(5.0, 0.0, 10.0, 10.0);
        let u: f64 = union(&[a.clone()], &[b.clone()]).iter().map(|p| p.area()).sum();
        let d: f64 = difference(&[a], &[b]).iter().map(|p| p.area()).sum();
        assert!((u - 150.0).abs() < 1e-3);
        assert!((d - 50.0).abs() < 1e-3);
    }
}
