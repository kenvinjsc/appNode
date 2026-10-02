use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Simple closed polygon (implicitly closed; last point != first point).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Polygon2D {
    pub points: Vec<Point2>,
}

impl Polygon2D {
    pub fn new(points: Vec<Point2>) -> Self {
        Self { points }
    }

    pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self::new(vec![
            Point2::new(x, y),
            Point2::new(x + w, y),
            Point2::new(x + w, y + h),
            Point2::new(x, y + h),
        ])
    }

    /// Signed area (positive for counter-clockwise).
    pub fn signed_area(&self) -> f64 {
        let n = self.points.len();
        if n < 3 {
            return 0.0;
        }
        let mut a = 0.0;
        for i in 0..n {
            let p = self.points[i];
            let q = self.points[(i + 1) % n];
            a += p.x * q.y - q.x * p.y;
        }
        a * 0.5
    }

    pub fn area(&self) -> f64 {
        self.signed_area().abs()
    }

    pub fn bounds(&self) -> (Point2, Point2) {
        let mut min = Point2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &self.points {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        }
        (min, max)
    }

    pub fn ensure_ccw(mut self) -> Self {
        if self.signed_area() < 0.0 {
            self.points.reverse();
        }
        self
    }

    /// Ear-clipping triangulation of a simple polygon (any winding, may be concave).
    /// Returns index triples in counter-clockwise order.
    pub fn triangulate(&self) -> Vec<[usize; 3]> {
        let n = self.points.len();
        if n < 3 {
            return Vec::new();
        }
        let p = &self.points;
        let mut idx: Vec<usize> = (0..n).collect();
        if self.signed_area() < 0.0 {
            idx.reverse();
        }
        let cross = |a: Point2, b: Point2, c: Point2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        let inside = |q: Point2, a: Point2, b: Point2, c: Point2| cross(a, b, q) >= -1e-12 && cross(b, c, q) >= -1e-12 && cross(c, a, q) >= -1e-12;
        let mut out = Vec::with_capacity(n - 2);
        let mut guard = 0;
        while idx.len() > 3 && guard < n * n {
            guard += 1;
            let m = idx.len();
            let mut clipped = false;
            for i in 0..m {
                let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
                let (a, b, c) = (p[ia], p[ib], p[ic]);
                if cross(a, b, c) <= 1e-12 {
                    continue; // reflex or degenerate
                }
                let blocked = idx.iter().any(|&j| j != ia && j != ib && j != ic && inside(p[j], a, b, c) && {
                    let q = p[j];
                    // Points coincident with the ear's vertices do not block it.
                    !((q.x - a.x).abs() < 1e-9 && (q.y - a.y).abs() < 1e-9 || (q.x - c.x).abs() < 1e-9 && (q.y - c.y).abs() < 1e-9)
                });
                if blocked {
                    continue;
                }
                out.push([ia, ib, ic]);
                idx.remove(i);
                clipped = true;
                break;
            }
            if !clipped {
                // Degenerate input (collinear run): drop a vertex and continue.
                let m = idx.len();
                let i = (0..m).find(|&i| cross(p[idx[(i + m - 1) % m]], p[idx[i]], p[idx[(i + 1) % m]]).abs() <= 1e-12).unwrap_or(0);
                idx.remove(i);
            }
        }
        if idx.len() == 3 && cross(p[idx[0]], p[idx[1]], p[idx[2]]).abs() > 1e-12 {
            out.push([idx[0], idx[1], idx[2]]);
        }
        out
    }

    /// Sutherland–Hodgman clip of `self` against a *convex* clip polygon.
    pub fn clip_convex(&self, clip: &Polygon2D) -> Polygon2D {
        let clip = clip.clone().ensure_ccw();
        let mut output = self.points.clone();
        let n = clip.points.len();
        for i in 0..n {
            if output.is_empty() {
                break;
            }
            let a = clip.points[i];
            let b = clip.points[(i + 1) % n];
            let inside = |p: &Point2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= -1e-12;
            let input = std::mem::take(&mut output);
            let m = input.len();
            for j in 0..m {
                let cur = input[j];
                let prev = input[(j + m - 1) % m];
                let (ci, pi) = (inside(&cur), inside(&prev));
                if ci {
                    if !pi {
                        output.push(intersect(prev, cur, a, b));
                    }
                    output.push(cur);
                } else if pi {
                    output.push(intersect(prev, cur, a, b));
                }
            }
        }
        Polygon2D::new(output)
    }
}

fn intersect(p1: Point2, p2: Point2, a: Point2, b: Point2) -> Point2 {
    let (dx, dy) = (p2.x - p1.x, p2.y - p1.y);
    let (ex, ey) = (b.x - a.x, b.y - a.y);
    let denom = dx * ey - dy * ex;
    if denom.abs() < 1e-15 {
        return p2;
    }
    let t = ((a.x - p1.x) * ey - (a.y - p1.y) * ex) / denom;
    Point2::new(p1.x + t * dx, p1.y + t * dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_area_and_clip() {
        let a = Polygon2D::rect(0.0, 0.0, 10.0, 10.0);
        let b = Polygon2D::rect(5.0, 5.0, 10.0, 10.0);
        assert_eq!(a.area(), 100.0);
        assert!((a.clip_convex(&b).area() - 25.0).abs() < 1e-9);
    }
}
