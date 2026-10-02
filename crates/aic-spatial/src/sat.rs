use aic_math::{Obb, Vector3};

/// Signed separation between two oriented boxes along the best SAT axis.
/// `distance > 0` is a gap, `< 0` a penetration depth; `axis` points from A to B.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Separation {
    pub distance: f64,
    pub axis: Vector3<f64>,
}

/// Separating-axis test over the 15 candidate axes of two boxes. For boxes whose
/// closest features are faces (the normal case for furniture panels) the result
/// is the exact gap / penetration depth.
pub fn obb_separation(a: &Obb, b: &Obb) -> Separation {
    let aa = a.axes();
    let ba = b.axes();
    let mut axes: Vec<Vector3<f64>> = Vec::with_capacity(15);
    axes.extend_from_slice(&aa);
    axes.extend_from_slice(&ba);
    for x in &aa {
        for y in &ba {
            let c = x.cross(y);
            let n = c.norm();
            if n > 1e-6 {
                axes.push(c / n);
            }
        }
    }
    let d = b.center() - a.center();
    let mut best = Separation { distance: f64::NEG_INFINITY, axis: Vector3::x() };
    for axis in axes {
        let axis = if axis.dot(&d) < 0.0 { -axis } else { axis };
        let (amin, amax) = a.project(&axis);
        let (bmin, bmax) = b.project(&axis);
        let gap = (bmin - amax).max(amin - bmax);
        // Strictly greater: face axes (listed first) win ties.
        if gap > best.distance + 1e-9 {
            best = Separation { distance: gap, axis };
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use aic_math::Transform3D;

    #[test]
    fn gap_touch_penetration() {
        let a = Obb::new([18.0, 500.0, 600.0], Transform3D::IDENTITY);
        let touch = Obb::new([18.0, 500.0, 600.0], Transform3D::from_translation(18.0, 0.0, 0.0));
        let gap = Obb::new([18.0, 500.0, 600.0], Transform3D::from_translation(20.0, 0.0, 0.0));
        let pen = Obb::new([18.0, 500.0, 600.0], Transform3D::from_translation(8.0, 0.0, 0.0));
        assert!(obb_separation(&a, &touch).distance.abs() < 1e-9);
        assert!((obb_separation(&a, &gap).distance - 2.0).abs() < 1e-9);
        assert!((obb_separation(&a, &pen).distance + 10.0).abs() < 1e-9);
        assert!((obb_separation(&a, &gap).axis.x - 1.0).abs() < 1e-9);
    }
}
