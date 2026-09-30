//! Generator sản phẩm ngoài tủ hộp (giường …). Toạ độ khung tủ: x trái → phải,
//! y sàn → trên, z sau → trước (mặt trước ở z lớn nhất).

use super::*;
use crate::product::{BedSpec, BedStorage, HeadboardStyle, Product, SlatKind};

pub(super) fn build(cx: &mut Ctx, p: &Product) {
    match p {
        Product::Bed(b) => bed(cx, b),
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
