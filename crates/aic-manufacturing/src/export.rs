//! Xuất file máy theo tấm (D26): DXF theo layer cho CAM ngoài, MPR (Homag WoodWOP) và CIX
//! (Biesse) cho máy khoan CNC có khoan cạnh. Dữ liệu vào là `FlatPanel` (đường bao + feature
//! toạ độ local mặt A); mặt B lật theo trục X (x' = rộng − x) khi `flip_for_b`.

use crate::flatten::FlatPanel;
use aic_domain::{EdgeSide, FaceSide, MachiningFeature};
use aic_math::Polygon2D;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Dxf,
    Mpr,
    Cix,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "DXF" => Some(Format::Dxf),
            "MPR" => Some(Format::Mpr),
            "CIX" => Some(Format::Cix),
            _ => None,
        }
    }
    pub fn ext(self) -> &'static str {
        match self {
            Format::Dxf => "dxf",
            Format::Mpr => "mpr",
            Format::Cix => "cix",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExportOptions {
    /// Mặt B: lật x để máy gia công sau khi lật tấm.
    pub flip_for_b: bool,
    /// Gốc toạ độ góc trái trên (y' = cao − y) thay vì trái dưới.
    pub origin_top: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self { flip_for_b: true, origin_top: false }
    }
}

/// Số gọn: 12.5, 15, 8 (không số 0 thừa).
pub fn n(v: f64) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        let s = format!("{r:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

struct Frame<'a> {
    fp: &'a FlatPanel,
    o: ExportOptions,
}

impl Frame<'_> {
    fn xy(&self, x: f64, y: f64, side: FaceSide) -> (f64, f64) {
        let x = if side == FaceSide::B && self.o.flip_for_b { self.fp.width - x } else { x };
        let y = if self.o.origin_top { self.fp.height - y } else { y };
        (x, y)
    }
}

/// Tâm lỗ khoan cạnh trên mặt tấm (x, y) + hướng khoan vào tấm.
fn edge_point(fp: &FlatPanel, edge: EdgeSide, offset: f64) -> ((f64, f64), (f64, f64)) {
    match edge {
        EdgeSide::Left => ((0.0, offset), (1.0, 0.0)),
        EdgeSide::Right => ((fp.width, offset), (-1.0, 0.0)),
        EdgeSide::Bottom => ((offset, 0.0), (0.0, 1.0)),
        EdgeSide::Top => ((offset, fp.height), (0.0, -1.0)),
    }
}

fn edge_name(e: EdgeSide) -> &'static str {
    match e {
        EdgeSide::Left => "L",
        EdgeSide::Right => "R",
        EdgeSide::Bottom => "B",
        EdgeSide::Top => "T",
    }
}

// ------------------------------------------------------------------ DXF

struct Dxf {
    out: String,
    layers: Vec<String>,
}

impl Dxf {
    fn new() -> Self {
        Self { out: String::new(), layers: Vec::new() }
    }
    fn layer(&mut self, l: &str) {
        if !self.layers.iter().any(|x| x == l) {
            self.layers.push(l.to_string());
        }
    }
    fn circle(&mut self, layer: &str, x: f64, y: f64, r: f64) {
        self.layer(layer);
        self.out += &format!("0\nCIRCLE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0\n40\n{}\n", n(x), n(y), n(r));
    }
    fn line(&mut self, layer: &str, a: (f64, f64), b: (f64, f64)) {
        self.layer(layer);
        self.out += &format!("0\nLINE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0\n", n(a.0), n(a.1), n(b.0), n(b.1));
    }
    fn poly(&mut self, layer: &str, pts: &[(f64, f64)]) {
        self.layer(layer);
        self.out += &format!("0\nPOLYLINE\n8\n{layer}\n66\n1\n70\n1\n");
        for (x, y) in pts {
            self.out += &format!("0\nVERTEX\n8\n{layer}\n10\n{}\n20\n{}\n30\n0\n", n(*x), n(*y));
        }
        self.out += "0\nSEQEND\n";
    }
    fn finish(self) -> String {
        let mut s = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n");
        s += &format!("70\n{}\n", self.layers.len());
        for l in &self.layers {
            s += &format!("0\nLAYER\n2\n{l}\n70\n0\n62\n7\n6\nCONTINUOUS\n");
        }
        s += "0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n";
        s += &self.out;
        s += "0\nENDSEC\n0\nEOF\n";
        s
    }
}

fn pts(f: &Frame, p: &Polygon2D, side: FaceSide) -> Vec<(f64, f64)> {
    p.points.iter().map(|q| f.xy(q.x, q.y, side)).collect()
}

/// DXF R12 theo layer: `CUT`, `CUT_INNER`, `DRILL_<Ø>_<sâu>` (mặt A), `DRILL_B_<Ø>_<sâu>`,
/// `HDRILL_<Ø>_<sâu>` (khoan cạnh: đường từ mép vào), `POCKET_<sâu>`, `GROOVE_<rộng>_<sâu>`.
pub fn dxf(fp: &FlatPanel, o: ExportOptions) -> String {
    let f = Frame { fp, o };
    let mut d = Dxf::new();
    d.poly("CUT", &pts(&f, &fp.outer, FaceSide::A));
    for h in &fp.inner {
        d.poly("CUT_INNER", &pts(&f, h, FaceSide::A));
    }
    for df in &fp.features {
        match &df.feature {
            MachiningFeature::Drill(dr) => {
                let (x, y) = f.xy(dr.x, dr.y, dr.side);
                let through = dr.depth >= fp.thickness - 1e-6;
                let layer = match (dr.side, through) {
                    (_, true) => format!("DRILL_{}_THRU", n(dr.diameter)),
                    (FaceSide::A, _) => format!("DRILL_{}_{}", n(dr.diameter), n(dr.depth)),
                    (FaceSide::B, _) => format!("DRILL_B_{}_{}", n(dr.diameter), n(dr.depth)),
                };
                d.circle(&layer, x, y, dr.diameter / 2.0);
            }
            MachiningFeature::EdgeDrill(e) => {
                let ((x, y), (dx, dy)) = edge_point(fp, e.edge, e.offset);
                let a = f.xy(x, y, FaceSide::A);
                let b = f.xy(x + dx * e.depth, y + dy * e.depth, FaceSide::A);
                d.line(&format!("HDRILL_{}_{}", n(e.diameter), n(e.depth)), a, b);
            }
            MachiningFeature::Pocket(p) => {
                let r = [(p.x, p.y), (p.x + p.width, p.y), (p.x + p.width, p.y + p.height), (p.x, p.y + p.height)];
                let side = if p.side == FaceSide::B { "_B" } else { "" };
                let v: Vec<(f64, f64)> = r.iter().map(|(x, y)| f.xy(*x, *y, p.side)).collect();
                d.poly(&format!("POCKET{side}_{}", n(p.depth)), &v);
            }
            MachiningFeature::Groove(g) => {
                let (x1, y1) = (g.x, g.y);
                let (x2, y2) = match g.direction {
                    aic_domain::Axis2::X => (g.x + g.length, g.y),
                    aic_domain::Axis2::Y => (g.x, g.y + g.length),
                };
                let side = if g.side == FaceSide::B { "_B" } else { "" };
                d.line(&format!("GROOVE{side}_{}_{}", n(g.width), n(g.depth)), f.xy(x1, y1, g.side), f.xy(x2, y2, g.side));
            }
            MachiningFeature::Contour(_) => {}
        }
    }
    d.finish()
}

// ------------------------------------------------------------------ MPR (Homag WoodWOP)

/// WoodWOP MPR: kích thước phôi, đường bao (`]1`), khoan đứng `<102 \BohrVert\` (mặt A / B),
/// khoan ngang `<103 \BohrHoriz\` theo cạnh, rãnh `<109 \Nuten\`, hốc `<112 \Tasche\`.
pub fn mpr(fp: &FlatPanel, o: ExportOptions) -> String {
    let f = Frame { fp, o };
    let mut s = String::new();
    s += "[H\nVERSION=\"4.0 Alpha\"\nOP=\"1\"\nMAT=\"HOMAG\"\nINCH=\"0\"\nVIEW=\"NOMIRROR\"\nANZ=\"1\"\n";
    s += &format!("_BSX={}\n_BSY={}\n_BSZ={}\n", n(fp.width), n(fp.height), n(fp.thickness));
    s += &format!("KOMMENTAR=\"{}\"\n\n[001\nL=\"{}\"\nB=\"{}\"\nD=\"{}\"\n\n", fp.name, n(fp.width), n(fp.height), n(fp.thickness));
    s += "]1\n";
    for (i, p) in fp.outer.points.iter().enumerate() {
        let (x, y) = f.xy(p.x, p.y, FaceSide::A);
        s += &format!("$E{i}\nKP\nX={}\nY={}\nZ=0\n", n(x), n(y));
    }
    s += "\n<100 \\WerkStck\\\nLA=\"L\"\nBR=\"B\"\nDI=\"D\"\n\n";
    for df in &fp.features {
        match &df.feature {
            MachiningFeature::Drill(dr) => {
                let (x, y) = f.xy(dr.x, dr.y, dr.side);
                let through = dr.depth >= fp.thickness - 1e-6;
                let bm = if through { "LSL" } else if dr.side == FaceSide::B { "LSU" } else { "LS" };
                s += &format!("<102 \\BohrVert\\\nXA=\"{}\"\nYA=\"{}\"\nBM=\"{bm}\"\nTI=\"{}\"\nDU=\"{}\"\nAN=\"1\"\n\n", n(x), n(y), n(dr.depth), n(dr.diameter));
            }
            MachiningFeature::EdgeDrill(e) => {
                let ((x, y), _) = edge_point(fp, e.edge, e.offset);
                let (x, y) = f.xy(x, y, FaceSide::A);
                let bm = match e.edge {
                    EdgeSide::Left => "XP",
                    EdgeSide::Right => "XM",
                    EdgeSide::Bottom => "YP",
                    EdgeSide::Top => "YM",
                };
                s += &format!("<103 \\BohrHoriz\\\nXA=\"{}\"\nYA=\"{}\"\nZA=\"{}\"\nBM=\"{bm}\"\nTI=\"{}\"\nDU=\"{}\"\nAN=\"1\"\n\n", n(x), n(y), n(e.z), n(e.depth), n(e.diameter));
            }
            MachiningFeature::Groove(g) => {
                let (xe, ye) = match g.direction {
                    aic_domain::Axis2::X => (g.x + g.length, g.y),
                    aic_domain::Axis2::Y => (g.x, g.y + g.length),
                };
                let (xa, ya) = f.xy(g.x, g.y, g.side);
                let (xe, ye) = f.xy(xe, ye, g.side);
                s += &format!("<109 \\Nuten\\\nXA=\"{}\"\nYA=\"{}\"\nXE=\"{}\"\nYE=\"{}\"\nNB=\"{}\"\nTI=\"{}\"\n\n", n(xa), n(ya), n(xe), n(ye), n(g.width), n(g.depth));
            }
            MachiningFeature::Pocket(p) => {
                let (x, y) = f.xy(p.x + p.width / 2.0, p.y + p.height / 2.0, p.side);
                s += &format!("<112 \\Tasche\\\nXA=\"{}\"\nYA=\"{}\"\nLA=\"{}\"\nBR=\"{}\"\nTI=\"{}\"\nR=\"{}\"\n\n", n(x), n(y), n(p.width), n(p.height), n(p.depth), n(p.corner_radius));
            }
            MachiningFeature::Contour(_) => {}
        }
    }
    s += "!\n";
    s
}

// ------------------------------------------------------------------ CIX (Biesse)

/// Biesse CIX: MAINDATA (LPX / LPY / LPZ), macro `BV` (khoan đứng, SIDE 0 = trên, 5 = dưới),
/// `BH` (khoan ngang, SIDE 1..4 theo cạnh), `ROUTG` rãnh, `POCK` hốc.
pub fn cix(fp: &FlatPanel, o: ExportOptions) -> String {
    let f = Frame { fp, o };
    let mut s = String::from("BEGIN ID CID3\n\tREL= 5.0\nEND ID\n\nBEGIN MAINDATA\n");
    s += &format!("\tLPX={}\n\tLPY={}\n\tLPZ={}\n\tORLST=\"1\"\n\tSIMMETRY=1\n\tCUSTSTR=\"{}\"\nEND MAINDATA\n\n", n(fp.width), n(fp.height), n(fp.thickness), fp.name);
    let param = |name: &str, v: String| format!("\tPARAM,NAME={name},VALUE={v}\n");
    for (i, df) in fp.features.iter().enumerate() {
        match &df.feature {
            MachiningFeature::Drill(dr) => {
                let (x, y) = f.xy(dr.x, dr.y, dr.side);
                let through = dr.depth >= fp.thickness - 1e-6;
                s += "BEGIN MACRO\n\tNAME=BV\n";
                s += &param("LAY", "\"DRILL\"".into());
                s += &param("ID", format!("\"B{i}\""));
                s += &param("SIDE", if dr.side == FaceSide::B { "5".into() } else { "0".into() });
                s += &param("X", n(x));
                s += &param("Y", n(y));
                s += &param("DP", n(dr.depth));
                s += &param("DIA", n(dr.diameter));
                s += &param("THR", if through { "YES".into() } else { "NO".into() });
                s += "END MACRO\n\n";
            }
            MachiningFeature::EdgeDrill(e) => {
                let ((x, y), _) = edge_point(fp, e.edge, e.offset);
                let (x, y) = f.xy(x, y, FaceSide::A);
                let side = match e.edge {
                    EdgeSide::Bottom => 1,
                    EdgeSide::Right => 2,
                    EdgeSide::Top => 3,
                    EdgeSide::Left => 4,
                };
                s += "BEGIN MACRO\n\tNAME=BH\n";
                s += &param("LAY", "\"HDRILL\"".into());
                s += &param("ID", format!("\"H{i}{}\"", edge_name(e.edge)));
                s += &param("SIDE", side.to_string());
                s += &param("X", n(x));
                s += &param("Y", n(y));
                s += &param("Z", n(e.z));
                s += &param("DP", n(e.depth));
                s += &param("DIA", n(e.diameter));
                s += "END MACRO\n\n";
            }
            MachiningFeature::Groove(g) => {
                let (xe, ye) = match g.direction {
                    aic_domain::Axis2::X => (g.x + g.length, g.y),
                    aic_domain::Axis2::Y => (g.x, g.y + g.length),
                };
                let (xa, ya) = f.xy(g.x, g.y, g.side);
                let (xe, ye) = f.xy(xe, ye, g.side);
                s += "BEGIN MACRO\n\tNAME=ROUTG\n";
                s += &param("SIDE", if g.side == FaceSide::B { "5".into() } else { "0".into() });
                s += &param("XI", n(xa));
                s += &param("YI", n(ya));
                s += &param("XF", n(xe));
                s += &param("YF", n(ye));
                s += &param("DP", n(g.depth));
                s += &param("TW", n(g.width));
                s += "END MACRO\n\n";
            }
            MachiningFeature::Pocket(p) => {
                let (x, y) = f.xy(p.x, p.y, p.side);
                s += "BEGIN MACRO\n\tNAME=POCK\n";
                s += &param("SIDE", if p.side == FaceSide::B { "5".into() } else { "0".into() });
                s += &param("X", n(x));
                s += &param("Y", n(y));
                s += &param("LX", n(p.width));
                s += &param("LY", n(p.height));
                s += &param("DP", n(p.depth));
                s += "END MACRO\n\n";
            }
            MachiningFeature::Contour(_) => {}
        }
    }
    s
}

pub fn export(fp: &FlatPanel, format: Format, o: ExportOptions) -> String {
    match format {
        Format::Dxf => dxf(fp, o),
        Format::Mpr => mpr(fp, o),
        Format::Cix => cix(fp, o),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_compact() {
        assert_eq!(n(12.5), "12.5");
        assert_eq!(n(15.0), "15");
        assert_eq!(n(8.0), "8");
        assert_eq!(n(0.125), "0.125");
    }
}
