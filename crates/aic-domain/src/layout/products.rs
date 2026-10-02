//! Generator sản phẩm ngoài tủ hộp (giường …). Toạ độ khung tủ: x trái → phải,
//! y sàn → trên, z sau → trước (mặt trước ở z lớn nhất).

use super::*;
use crate::product::{BedSpec, BedStorage, CladdingSpec, DeskSpec, DeskSupport, HeadboardStyle, JointType, Product, SlatKind};

pub(super) fn build(cx: &mut Ctx, p: &Product) {
    match p {
        Product::Bed(b) => bed(cx, b),
        Product::Desk(d) => desk(cx, d),
        Product::Cladding(c) => cladding(cx, c),
    }
}

/// Giường: lọt nệm W × L; vai (thành) hai bên dài suốt L + dày đầu + dày đuôi,
/// đầu giường và đuôi lọt giữa hai vai.
fn bed(cx: &mut Ctx, b: &BedSpec) {
    let v = cx.v;
    let (w, l, t) = (v.width, v.depth, v.thickness);
    let ht = if b.headboard_t > 0.0 { b.headboard_t } else { t };
    let hh = v.height.max(b.frame_h + 100.0);
    let wt = w + 2.0 * t;
    let lt = l + ht + t;
    let fh = b.frame_h.clamp(200.0, 700.0);
    let rail_top = fh + b.lip.max(0.0);
    let st = b.slat_t.max(6.0);
    let ledge = 40.0;
    let y_ledge = fh - st - ledge;
    let legs_h = 100.0_f64.min(y_ledge - 20.0).max(0.0);
    let drawers = matches!(b.storage, BedStorage::DrawersTwoSides | BedStorage::DrawersFoot);
    // Hộc kéo nằm dưới thanh đỡ dát; vai / đuôi phía có hộc chỉ còn phần trên hộc.
    let drawer_top = y_ledge - 10.0;
    let drawer_bot = legs_h + 10.0;
    let rail_h = b.side_rail_h.clamp(100.0, rail_top);
    let side_rail_y = |has: bool| if has { drawer_top } else { rail_top - rail_h };
    let sides = b.storage == BedStorage::DrawersTwoSides;
    let foot = b.storage == BedStorage::DrawersFoot;

    // Đầu giường.
    cx.panel("b:head".into(), "ĐầuGiường".into(), PanelRole::Generic, MaterialSlot::Front, GrainDirection::AlongHeight, [w, hh, ht], [t, 0.0, 0.0], [0.0; 3]);
    match b.headboard_style {
        HeadboardStyle::Upholstered => {
            let (pw, ph) = (w - 40.0, (hh - rail_top - 80.0).max(100.0));
            cx.panel("b:head_pad".into(), "ĐệmĐầuGiường".into(), PanelRole::Generic, MaterialSlot::Front, GrainDirection::AlongWidth, [pw, ph, 30.0], [t + 20.0, rail_top + 40.0, ht], [0.0; 3]);
        }
        HeadboardStyle::Slatted => {
            let n = ((w / 120.0).floor() as u32).max(3);
            let step = w / n as f64;
            for i in 0..n {
                cx.panel(format!("b:head_slat{i}"), format!("NanĐầuGiường_{:02}", i + 1), PanelRole::Generic, MaterialSlot::Front, GrainDirection::AlongHeight, [60.0, hh - rail_top - 60.0, 20.0], [t + i as f64 * step + (step - 60.0) / 2.0, rail_top + 30.0, ht], [0.0; 3]);
            }
        }
        HeadboardStyle::Flat => {}
    }
    // Vai trái / phải (ROT_SIDE: dài theo -z từ mặt trước).
    for (key, name, x, role) in [("b:rail_l", "VaiTrái", 0.0, PanelRole::LeftSide), ("b:rail_r", "VaiPhải", w + t, PanelRole::RightSide)] {
        let y = side_rail_y(sides);
        cx.panel(key.into(), name.into(), role, MaterialSlot::Front, GrainDirection::AlongWidth, [lt, rail_top - y, t], [x, y, lt], ROT_SIDE);
    }
    // Đuôi giường.
    let fy = side_rail_y(foot);
    cx.panel("b:foot".into(), "ĐuôiGiường".into(), PanelRole::Generic, MaterialSlot::Front, GrainDirection::AlongWidth, [w, rail_top - fy, t], [t, fy, ht + l], [0.0; 3]);
    // Thanh đỡ dát trong hai vai.
    for (key, name, x) in [("b:ledge_l", "ThanhĐỡDátTrái", t), ("b:ledge_r", "ThanhĐỡDátPhải", w + t - 20.0)] {
        cx.panel(key.into(), name.into(), PanelRole::Rail, MaterialSlot::Carcass, GrainDirection::AlongWidth, [l, ledge, 20.0], [x, y_ledge, ht + l], ROT_SIDE);
    }
    // Đà giữa.
    let beam = b.center_beam && w >= 1400.0;
    if beam {
        cx.panel("b:beam".into(), "ĐàGiữa".into(), PanelRole::Rail, MaterialSlot::Carcass, GrainDirection::AlongWidth, [l, 100.0, t], [t + (w - t) / 2.0, fh - st - 100.0, ht + l], ROT_SIDE);
    }
    // Dát.
    match b.slats {
        SlatKind::Board => {
            // Dát tấm: chia đôi theo đà giữa.
            let pieces = if beam { 2 } else { 1 };
            let pw = w / pieces as f64;
            for i in 0..pieces {
                cx.panel(format!("b:deck{i}"), format!("DátTấm_{:02}", i + 1), PanelRole::Shelf, MaterialSlot::Carcass, GrainDirection::AlongHeight, [pw, l, st], [t + i as f64 * pw, fh - st, ht + l], ROT_HORIZONTAL);
            }
        }
        SlatKind::Slats => {
            let n = b.slat_count.clamp(6, 30);
            let sw = 70.0;
            let step = l / n as f64;
            for i in 0..n {
                let zf = ht + l - i as f64 * step - (step - sw) / 2.0;
                cx.panel(format!("b:slat{i}"), format!("NanDát_{:02}", i + 1), PanelRole::Shelf, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, sw, st], [t, fh - st, zf], ROT_HORIZONTAL);
            }
        }
    }
    // Chân: 4 góc (+2 giữa dưới đà khi 6 chân).
    let dia = 50.0;
    let mut spots = vec![(t, ht), (w + t - dia, ht), (t, lt - dia - t), (w + t - dia, lt - dia - t)];
    if b.legs >= 6 && !drawers {
        // 6 chân: thêm 2 chân giữa dưới đà (có đà) hoặc giữa hai vai.
        let zm = ht + l / 2.0;
        if beam {
            spots.push((t + (w - dia) / 2.0, ht + l / 4.0));
            spots.push((t + (w - dia) / 2.0, ht + 3.0 * l / 4.0));
        } else {
            spots.push((t, zm));
            spots.push((w + t - dia, zm));
        }
    }
    for (k, (x, z)) in spots.into_iter().enumerate() {
        cx.hardware(format!("b:leg{k}"), format!("ChânGiường_{:02}", k + 1), HardwareKind::Leg, "LEG-BED", [dia, legs_h, dia], [x, 0.0, z]);
        cx.out.fittings.legs += 1;
    }
    if b.storage == BedStorage::GasLift {
        cx.hardware("b:gas_l".into(), "BenHơi_01".into(), HardwareKind::Rail, "GAS-LIFT-800N", [30.0, 400.0, 30.0], [t + 30.0, y_ledge - 400.0, ht + l / 2.0]);
        cx.hardware("b:gas_r".into(), "BenHơi_02".into(), HardwareKind::Rail, "GAS-LIFT-800N", [30.0, 400.0, 30.0], [w + t - 60.0, y_ledge - 400.0, ht + l / 2.0]);
        cx.out.fittings.gas_lifts += 2;
    }
    if !drawers {
        return;
    }
    // Hộc kéo: chia đều chiều dài (2 bên) hoặc chiều rộng (đuôi).
    let n = b.drawer_count.clamp(1, 4);
    let dh = drawer_top - drawer_bot;
    let ft = cx.v.door_thickness;
    let gap = 3.0;
    let bt = 15.0;
    if sides {
        // Mặt hộc thẳng mặt ngoài vai; hộc sâu theo ray, không quá nửa rộng giường.
        let avail = (w / 2.0 - 40.0).max(250.0);
        let sl = STD_SLIDES.iter().copied().filter(|x| *x <= avail - ft).fold(STD_SLIDES[0], f64::max);
        let span = l - 2.0 * t;
        let fw = (span - (n - 1) as f64 * gap) / n as f64;
        for (side, left) in [("l", true), ("r", false)] {
            // Vách đỡ ray phía trong.
            let xi = if left { ft + sl + 10.0 } else { wt - ft - sl - 10.0 - t };
            cx.panel(format!("b:dw_wall_{side}"), format!("VáchHộc{}", if left { "Trái" } else { "Phải" }), PanelRole::Divider, MaterialSlot::Carcass, GrainDirection::AlongWidth, [l, drawer_top - legs_h, t], [xi, legs_h, ht + l], ROT_SIDE);
            for k in 0..n {
                let z1 = ht + l - t - k as f64 * (fw + gap);
                let z0 = z1 - fw;
                let tag = format!("{side}{k}");
                let x_front = if left { 0.0 } else { wt - ft };
                cx.panel(format!("b:dw_front_{tag}"), format!("MặtHộcGiường_{}{:02}", side.to_uppercase(), k + 1), PanelRole::DrawerFront, MaterialSlot::Front, GrainDirection::AlongWidth, [fw, dh, ft], [x_front, drawer_bot, z1], ROT_SIDE);
                let xb = if left { ft } else { wt - ft - sl };
                let hb = dh - 30.0;
                cx.panel(format!("b:dw_s1_{tag}"), format!("ThànhHộcGiường_{}{:02}", side.to_uppercase(), k + 1), PanelRole::DrawerSide, MaterialSlot::Carcass, GrainDirection::AlongWidth, [sl, hb, bt], [xb, drawer_bot + 10.0, z0 + 12.5], [0.0; 3]);
                cx.panel(format!("b:dw_s2_{tag}"), format!("ThànhHộcGiường_{}{:02}b", side.to_uppercase(), k + 1), PanelRole::DrawerSide, MaterialSlot::Carcass, GrainDirection::AlongWidth, [sl, hb, bt], [xb, drawer_bot + 10.0, z1 - 12.5 - bt], [0.0; 3]);
                let xback = if left { ft + sl - bt } else { wt - ft - sl };
                cx.panel(format!("b:dw_back_{tag}"), format!("HậuHộcGiường_{}{:02}", side.to_uppercase(), k + 1), PanelRole::DrawerBack, MaterialSlot::Carcass, GrainDirection::AlongWidth, [fw - 25.0 - 2.0 * bt, hb, bt], [xback, drawer_bot + 10.0, z1 - 12.5 - bt], ROT_SIDE);
                cx.panel(format!("b:dw_bot_{tag}"), format!("ĐáyHộcGiường_{}{:02}", side.to_uppercase(), k + 1), PanelRole::DrawerBottom, MaterialSlot::Back, GrainDirection::AlongWidth, [sl, fw - 25.0, 8.0], [xb, drawer_bot + 10.0, z1 - 12.5], ROT_HORIZONTAL);
                *cx.out.fittings.slides.entry(sl as u32).or_insert(0) += 1;
            }
        }
    } else {
        // Hộc đuôi: mặt hộc thẳng mặt ngoài đuôi (z = lt).
        let avail = (l / 2.0).max(250.0);
        let sl = STD_SLIDES.iter().copied().filter(|x| *x <= avail - ft).fold(STD_SLIDES[0], f64::max);
        let fw = (w - (n - 1) as f64 * gap) / n as f64;
        for k in 0..n {
            let x0 = t + k as f64 * (fw + gap);
            let tag = format!("f{k}");
            cx.panel(format!("b:dw_front_{tag}"), format!("MặtHộcGiường_Đ{:02}", k + 1), PanelRole::DrawerFront, MaterialSlot::Front, GrainDirection::AlongWidth, [fw, dh, ft], [x0, drawer_bot, lt - ft], [0.0; 3]);
            let hb = dh - 30.0;
            let zf = lt - ft;
            for (j, xs) in [(1, x0 + 12.5), (2, x0 + fw - 12.5 - bt)] {
                cx.panel(format!("b:dw_s{j}_{tag}"), format!("ThànhHộcGiường_Đ{:02}{}", k + 1, if j == 2 { "b" } else { "" }), PanelRole::DrawerSide, MaterialSlot::Carcass, GrainDirection::AlongWidth, [sl, hb, bt], [xs, drawer_bot + 10.0, zf], ROT_SIDE);
            }
            cx.panel(format!("b:dw_back_{tag}"), format!("HậuHộcGiường_Đ{:02}", k + 1), PanelRole::DrawerBack, MaterialSlot::Carcass, GrainDirection::AlongWidth, [fw - 25.0 - 2.0 * bt, hb, bt], [x0 + 12.5 + bt, drawer_bot + 10.0, zf - sl], [0.0; 3]);
            cx.panel(format!("b:dw_bot_{tag}"), format!("ĐáyHộcGiường_Đ{:02}", k + 1), PanelRole::DrawerBottom, MaterialSlot::Back, GrainDirection::AlongWidth, [fw - 25.0, sl, 8.0], [x0 + 12.5, drawer_bot + 10.0, zf], ROT_HORIZONTAL);
            *cx.out.fittings.slides.entry(sl as u32).or_insert(0) += 1;
        }
    }
}

/// Bàn: mặt bàn (dày, nhô hai bên), đỡ trái / phải (chân tấm, hộc tủ ngăn kéo, chân sắt),
/// yếm, hộc bàn phím, kệ trên, khoét luồn dây, gương.
fn desk(cx: &mut Ctx, s: &DeskSpec) {
    let v = cx.v;
    let (w, h, d, t) = (v.width, v.height, v.depth, v.thickness);
    let tt = s.top_t.clamp(12.0, 60.0);
    let o = s.top_overhang.clamp(0.0, 100.0);
    let under = h - tt;
    let top = cx.panel("k:top".into(), "MặtBàn".into(), PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w, d, tt], [0.0, under, d], ROT_HORIZONTAL);
    if s.cable_d > 0.0 {
        // Tâm lỗ cách mép phải / mép sau; local y của tấm nằm ngang đo từ mép trước.
        let (lx, ly) = (w - s.cable_x, d - s.cable_y);
        let r = s.cable_d / 2.0;
        if lx - r > 5.0 && lx + r < w - 5.0 && ly - r > 5.0 && ly + r < d - 5.0 {
            cx.add_features(top, vec![MachiningFeature::Contour(crate::ContourFeature { polygon: rounded_rect(lx, ly, s.cable_d, s.cable_d, r), inner: true, depth: tt })]);
        }
    }
    // Đỡ hai bên; trả về mặt trong (x) để yếm / hộc phím nằm giữa.
    let mut inner = [o, w - o];
    for (i, sup) in [s.support_left, s.support_right].into_iter().enumerate() {
        let left = i == 0;
        let side = if left { "Trái" } else { "Phải" };
        match sup {
            DeskSupport::Panel => {
                let x = if left { o } else { w - o - t };
                cx.panel(format!("k:leg_{i}"), format!("ChânTấm{side}"), PanelRole::LeftSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, under, t], [x, 0.0, d], ROT_SIDE);
                inner[i] = if left { x + t } else { x };
            }
            DeskSupport::Leg => {
                let x = if left { o + 10.0 } else { w - o - 60.0 };
                for (k, z) in [(0, 30.0), (1, d - 80.0)] {
                    cx.hardware(format!("k:iron_{i}{k}"), format!("ChânSắt{side}_{:02}", k + 1), HardwareKind::Leg, "DESK-LEG-50", [50.0, under, 50.0], [x, 0.0, z]);
                    cx.out.fittings.legs += 1;
                }
                inner[i] = if left { x + 50.0 } else { x };
            }
            DeskSupport::DrawerUnit => {
                let uw = s.unit_w.clamp(250.0, (w - 2.0 * o) / 2.0);
                let x0 = if left { o } else { w - o - uw };
                let kick = 50.0;
                let l = cx.panel(format!("k:unit_{i}_l"), format!("HồiHộc{side}_01"), PanelRole::LeftSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, under, t], [x0, 0.0, d], ROT_SIDE);
                let r = cx.panel(format!("k:unit_{i}_r"), format!("HồiHộc{side}_02"), PanelRole::RightSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [d, under, t], [x0 + uw - t, 0.0, d], ROT_SIDE);
                let b = cx.panel(format!("k:unit_{i}_b"), format!("ĐáyHộc{side}"), PanelRole::Bottom, MaterialSlot::Carcass, GrainDirection::AlongWidth, [uw - 2.0 * t, d - 20.0, t], [x0 + t, kick, d - 20.0], ROT_HORIZONTAL);
                cx.panel(format!("k:unit_{i}_kick"), format!("LenHộc{side}"), PanelRole::Plinth, MaterialSlot::Carcass, GrainDirection::AlongWidth, [uw - 2.0 * t, kick, t], [x0 + t, 0.0, d - 20.0 - t], [0.0; 3]);
                cx.panel(format!("k:unit_{i}_back"), format!("HậuHộc{side}"), PanelRole::Back, MaterialSlot::Back, GrainDirection::AlongHeight, [uw - 2.0 * t, under - kick - t, v.back_thickness], [x0 + t, kick + t, 0.0], [0.0; 3]);
                let bx = ZBox {
                    min: [x0 + t, kick + t, v.back_thickness],
                    size: [uw - 2.0 * t, under - kick - t, d - v.back_thickness],
                    nb: [Neighbor { t, outer: true, part: Some(l) }, Neighbor { t, outer: true, part: Some(r) }, Neighbor { t, outer: true, part: Some(b) }, Neighbor { t: tt, outer: true, part: Some(top) }],
                };
                let spec = DrawerSpec::new(9000 + i as u32, s.unit_drawers.clamp(1, 6), Mount::Overlay);
                drawers(cx, &spec, &bx);
                inner[i] = if left { x0 + uw } else { x0 };
            }
        }
    }
    let knee = inner[1] - inner[0];
    if s.modesty && knee > 50.0 {
        let mh = s.modesty_h.clamp(50.0, under);
        cx.panel("k:modesty".into(), "Yếm".into(), PanelRole::Back, MaterialSlot::Carcass, GrainDirection::AlongWidth, [knee, mh, t], [inner[0], under - mh, s.modesty_setback.clamp(0.0, d / 2.0)], [0.0; 3]);
    }
    if s.keyboard_tray && knee > 300.0 {
        let kw = knee.min(700.0) - 30.0;
        let sl = STD_SLIDES.iter().copied().filter(|x| *x <= d - 100.0).fold(STD_SLIDES[0], f64::max);
        cx.panel("k:tray".into(), "HộcBànPhím".into(), PanelRole::Shelf, MaterialSlot::Carcass, GrainDirection::AlongWidth, [kw, sl, t], [inner[0] + (knee - kw) / 2.0, under - 80.0, d], ROT_HORIZONTAL);
        *cx.out.fittings.slides.entry(sl as u32).or_insert(0) += 1;
    }
    if s.hutch_h > 0.0 {
        // Kệ trên: 2 hồi + nóc + kệ, đặt sát mép sau mặt bàn.
        let (hh, hd) = (s.hutch_h.clamp(200.0, 1500.0), s.hutch_d.clamp(150.0, d));
        cx.panel("k:hutch_l".into(), "HồiKệTrên_01".into(), PanelRole::LeftSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [hd, hh, t], [0.0, h, hd], ROT_SIDE);
        cx.panel("k:hutch_r".into(), "HồiKệTrên_02".into(), PanelRole::RightSide, MaterialSlot::Carcass, GrainDirection::AlongHeight, [hd, hh, t], [w - t, h, hd], ROT_SIDE);
        cx.panel("k:hutch_top".into(), "NócKệTrên".into(), PanelRole::Top, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w - 2.0 * t, hd, t], [t, h + hh - t, hd], ROT_HORIZONTAL);
        let n = s.hutch_shelves.min(6);
        let step = (hh - t) / (n + 1) as f64;
        for k in 0..n {
            cx.panel(format!("k:hutch_s{k}"), format!("KệTrên_{:02}", k + 1), PanelRole::ShelfFixed, MaterialSlot::Carcass, GrainDirection::AlongWidth, [w - 2.0 * t, hd, t], [t, h + (k + 1) as f64 * step - t / 2.0, hd], ROT_HORIZONTAL);
        }
    }
    if s.mirror_w > 0.0 {
        let mw = s.mirror_w.min(w);
        cx.panel("k:mirror".into(), "Gương".into(), PanelRole::Generic, MaterialSlot::Front, GrainDirection::AlongHeight, [mw, s.mirror_h.clamp(200.0, 1500.0), 5.0], [(w - mw) / 2.0, h + 50.0, 0.0], [0.0; 3]);
    }
}

/// Kích thước các ô theo công thức chia (khe `gap` giữa hai ô); công thức sai / trống = 1 ô.
fn cells(len: f64, formula: &str, gap: f64) -> (Vec<f64>, bool) {
    let bays = if formula.trim().is_empty() { Ok(vec![Bay::auto()]) } else { parse_split_formula(formula) };
    match bays {
        Ok(b) if !b.is_empty() => {
            let gaps = vec![gap; b.len() - 1];
            let (s, ok) = solve_bays(len, &gaps, &b);
            (s, ok)
        }
        _ => (vec![len], false),
    }
}

/// Vách ốp: lưới tấm (cột × hàng theo công thức, khe bóng / soi V), khung xương, lam gỗ,
/// khoét hộp điện trên tấm chứa tâm lỗ.
fn cladding(cx: &mut Ctx, c: &CladdingSpec) {
    let v = cx.v;
    let (w, h, t) = (v.width, v.height, v.thickness);
    let gap = if c.joint_type == JointType::ShadowGap { c.joint_gap.clamp(0.0, 30.0) } else { 0.0 };
    let fz = if c.frame { 20.0 } else { 0.0 };
    if c.frame {
        let n = ((w - 40.0) / 400.0).ceil().max(1.0) as u32;
        let step = (w - 40.0) / n as f64;
        for i in 0..=n {
            cx.panel(format!("w:frame{i}"), format!("KhungXương_{:02}", i + 1), PanelRole::Rail, MaterialSlot::Carcass, GrainDirection::AlongHeight, [40.0, h, 20.0], [i as f64 * step, 0.0, 0.0], [0.0; 3]);
        }
    }
    let holes: Vec<(f64, f64, f64, f64, f64)> = c
        .cutouts
        .iter()
        .map(|k| {
            let (cw, ch, r) = k.shape();
            let x = match k.anchor {
                crate::structure::HAnchor::Left => k.x,
                crate::structure::HAnchor::Center => w / 2.0 + k.x,
                crate::structure::HAnchor::Right => w - k.x,
            };
            (x, k.y, cw, ch, r)
        })
        .collect();
    let mut bz = fz;
    if c.boards {
        let (cols, ok1) = cells(w, &c.cols, gap);
        let (rows, ok2) = cells(h, &c.rows, gap);
        if !(ok1 && ok2) {
            cx.out.problems.push(0);
        }
        let v_groove = c.joint_type == JointType::VGroove;
        let mut n = 0;
        let mut y = 0.0;
        for (ri, rh) in rows.iter().enumerate() {
            let mut x = 0.0;
            for (ci, cw) in cols.iter().enumerate() {
                n += 1;
                let name = format!("TấmỐp_{n:02}{}", if v_groove { " (soi V)" } else { "" });
                let idx = cx.panel(format!("w:p{ri}_{ci}"), name, PanelRole::Generic, MaterialSlot::Front, GrainDirection::AlongHeight, [*cw, *rh, t], [x, y, fz], [0.0; 3]);
                let mut feats = Vec::new();
                for &(hx, hy, hw, hh, r) in &holes {
                    let (lx, ly) = (hx - x, hy - y);
                    if lx - hw / 2.0 >= 5.0 && lx + hw / 2.0 <= cw - 5.0 && ly - hh / 2.0 >= 5.0 && ly + hh / 2.0 <= rh - 5.0 {
                        feats.push(MachiningFeature::Contour(crate::ContourFeature { polygon: rounded_rect(lx, ly, hw, hh, r), inner: true, depth: t }));
                    }
                }
                cx.add_features(idx, feats);
                x += cw + gap;
            }
            y += rh + gap;
        }
        bz += t;
    }
    if c.batten {
        let (bw, bg, bt) = (c.batten_w.clamp(10.0, 200.0), c.batten_gap.clamp(0.0, 200.0), c.batten_t.clamp(8.0, 60.0));
        let (span, len) = if c.batten_vertical { (w, h) } else { (h, w) };
        // Số lam vừa khít: n lam + (n − 1) khe ≤ span, chia đều phần dư vào khe.
        let n = ((span + bg) / (bw + bg)).floor().max(1.0) as u32;
        let g = if n > 1 { (span - n as f64 * bw) / (n - 1) as f64 } else { 0.0 };
        for i in 0..n {
            let a = i as f64 * (bw + g);
            let (size, at) = if c.batten_vertical { ([bw, len, bt], [a, 0.0, bz]) } else { ([len, bw, bt], [0.0, a, bz]) };
            cx.panel(format!("w:batten{i}"), format!("Lam_{:02}", i + 1), PanelRole::Rail, MaterialSlot::Front, if c.batten_vertical { GrainDirection::AlongHeight } else { GrainDirection::AlongWidth }, size, at, [0.0; 3]);
        }
    }
}
