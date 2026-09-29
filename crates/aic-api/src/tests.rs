use super::*;
use serde_json::json;

fn call(e: &mut Engine, v: Value) -> Response {
    let r: Request = serde_json::from_value(v).unwrap();
    e.dispatch(r)
}

fn created_cabinet(e: &mut Engine) -> ObjectId {
    let r = call(e, json!({"cmd": "create_cabinet", "kind": "WARDROBE"}));
    assert!(r.ok, "{:?}", r.error);
    serde_json::from_value(r.result["id"].clone()).unwrap()
}

#[test]
fn end_to_end_flow() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);

    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let children = tree.result["roots"][0]["children"].as_array().unwrap().len();
    assert!(children > 15, "wardrobe TủQA: carcass + divider + shelves + doors + handles + rail");
    assert_eq!(tree.result["roots"][0]["name"], "TủQA01");

    // Render all, then only changed geometry after a width edit.
    let all = call(&mut e, json!({"cmd": "get_render_objects"}));
    let objects = all.result["objects"].as_array().unwrap();
    assert_eq!(objects.len(), children);
    let known: Vec<String> = all.result["meshes"].as_object().unwrap().keys().cloned().collect();
    assert!(known.len() < objects.len(), "identical parts share geometry");

    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "800"}));
    assert!(r.ok);
    let geo = r.events.iter().find_map(|ev| match ev { CoreEvent::GeometryChanged { ids } => Some(ids.clone()), _ => None }).unwrap();
    assert!(geo.len() >= 5 && geo.len() < children, "only affected parts: {geo:?}");
    let upd = call(&mut e, json!({"cmd": "get_render_objects", "ids": geo, "known_keys": known}));
    assert!(!upd.result["meshes"].as_object().unwrap().is_empty());

    // Error codes are stable for UI localisation.
    let bad = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "20"}));
    assert!(!bad.ok);
    let err = bad.error.unwrap();
    assert_eq!(err.code, "CONSTRAINT_VIOLATED");
    assert_eq!(err.details["constraint"], "WIDTH_LESS_THAN_SIDES");

    // Undo/redo.
    assert!(call(&mut e, json!({"cmd": "undo"})).ok);
    assert_eq!(e.doc.param_value(cab, "width"), Some(1600.0));
    assert!(call(&mut e, json!({"cmd": "redo"})).ok);

    // Properties.
    let props = call(&mut e, json!({"cmd": "get_properties", "id": cab}));
    assert_eq!(props.result["kind"], "CABINET");

    // Relations + manufacturing.
    let rel = call(&mut e, json!({"cmd": "get_relations"}));
    let rels = rel.result["relations"].as_array().unwrap();
    assert!(rels.iter().any(|r| r["contact"] == "TOUCH"));
    let side = e.doc.objects.values().find_map(|o| o.as_panel().filter(|p| p.role == aic_domain::PanelRole::LeftSide)).unwrap().id;
    let m = call(&mut e, json!({"cmd": "get_manufacturing", "id": side}));
    assert!(m.result["summary"]["drills"].as_u64().unwrap() >= 8, "{}", m.result["summary"]);
    let door = e.doc.objects.values().find_map(|o| o.as_panel().filter(|p| p.role == aic_domain::PanelRole::Door)).unwrap().id;
    let md = call(&mut e, json!({"cmd": "get_manufacturing", "id": door}));
    assert_eq!(md.result["summary"]["drills"].as_u64().unwrap(), 4, "4 hinge cups on a door > 1600 mm");

    // Parts, nesting, CNC.
    let parts = call(&mut e, json!({"cmd": "get_parts"}));
    assert!(parts.result["parts"].as_array().unwrap().len() >= 12);
    let nest = call(&mut e, json!({"cmd": "run_nesting", "material": "MDF17-WHITE"}));
    assert!(nest.ok, "{:?}", nest.error);
    let job = &nest.result["jobs"][0];
    assert!(job["result"]["unplaced"].as_array().unwrap().is_empty());
    let cnc = call(&mut e, json!({"cmd": "generate_cnc", "material": "MDF17-WHITE", "sheet_id": 0}));
    assert!(cnc.ok, "{:?}", cnc.error);
    assert!(cnc.result["program"]["gcode"].as_str().unwrap().contains("M30"));

    // Save / load roundtrip keeps objects.
    let saved = call(&mut e, json!({"cmd": "save_project"}));
    let mut e2 = Engine::new();
    let loaded = call(&mut e2, json!({"cmd": "load_project", "project": saved.result}));
    assert!(loaded.ok, "{:?}", loaded.error);
    assert!(loaded.events.contains(&CoreEvent::ProjectLoaded));
    assert_eq!(e2.doc.objects, e.doc.objects);
}

#[test]
fn snap_and_duplicate() {
    let mut e = Engine::new();
    let a = created_cabinet(&mut e);
    let d = call(&mut e, json!({"cmd": "duplicate_objects", "ids": [a]}));
    let b: ObjectId = serde_json::from_value(d.result["created"][0].clone()).unwrap();
    // Move b towards a, 7 mm short of touching: snap closes the gap.
    let bx = e.doc.param_value(b, "x").unwrap();
    let s = call(&mut e, json!({"cmd": "snap", "id": b, "delta": [-(bx - 1600.0) + 7.0, 0.0, 0.0]}));
    assert!((s.result["delta"][0].as_f64().unwrap() + (bx - 1600.0)).abs() < 1e-6, "{}", s.result);
    let del = call(&mut e, json!({"cmd": "delete_objects", "ids": [b]}));
    assert!(del.events.iter().any(|ev| matches!(ev, CoreEvent::SelectionInvalidated { .. })));
}

#[test]
fn bad_request_is_reported() {
    let mut e = Engine::new();
    let out = e.dispatch_json(r#"{"cmd":"nope"}"#);
    assert!(out.contains("\"ok\":false"));
}

#[test]
fn zone_workflow_tao_tam_chinh_tam() {
    let mut e = Engine::new();
    // P1: kitchen base unit next to another one.
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "room": "Bếp"}));
    let a: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "after": a}));
    let b: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    assert_eq!(e.doc.param_value(b, "x"), Some(800.0), "placed right of BếpDưới01");
    assert_eq!(e.doc.object(b).unwrap().name(), "BếpDưới02");

    // Clear the preset content of A, then pin the root zone.
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": a}));
    let root = z.result["zones"][0]["id"].as_u64().unwrap();
    for att in z.result["attachments"].as_array().unwrap() {
        call(&mut e, json!({"cmd": "zone_remove", "cabinet": a, "uid": att["uid"]}));
    }
    let shelves: Vec<u64> = z.result["positions"].as_array().unwrap().iter().map(|p| p["uid"].as_u64().unwrap()).collect();
    for u in shelves {
        assert!(call(&mut e, json!({"cmd": "zone_remove", "cabinet": a, "uid": u})).ok);
    }
    // P3: 1 adjustable shelf at 50 %.
    let r = call(&mut e, json!({"cmd": "zone_add_panels", "cabinet": a, "zones": [root], "kind": "SHELF_ADJUSTABLE", "lock": "RATIO", "value": 50}));
    assert!(r.ok, "{:?}", r.error);
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": a}));
    let pos = &z.result["positions"][0];
    assert!((pos["from_start"].as_f64().unwrap() - pos["from_end"].as_f64().unwrap()).abs() < 1e-9);
    // P10: edit "Cách dưới" of the shelf → ratio & cách trên follow.
    let shelf = e.doc.objects.values().filter_map(|o| o.as_panel()).find(|p| p.role == aic_domain::PanelRole::Shelf && e.doc.cabinet_of(p.id) == Some(a)).unwrap().id;
    assert!(call(&mut e, json!({"cmd": "set_parameter", "id": shelf, "name": "pos_start", "value": "300"})).ok);
    let props = call(&mut e, json!({"cmd": "get_properties", "id": shelf}));
    let fields: Vec<&Value> = props.result["groups"].as_array().unwrap().iter().flat_map(|g| g["fields"].as_array().unwrap().iter()).collect();
    let start = fields.iter().find(|f| f["key"] == "pos_start").unwrap();
    assert_eq!(start["value"], json!(300.0));
    assert_eq!(start["locked"], json!(true));
    let width = fields.iter().find(|f| f["key"] == "width").unwrap();
    assert_eq!(width["editable"], json!(false), "Dài/Rộng are computed");
    // P6: double overlay doors; P7: drawers; P9: oval rail.
    assert!(call(&mut e, json!({"cmd": "zone_add_doors", "cabinet": a, "zones": [root], "kind": "DOUBLE", "cols": 2})).ok);
    let doors = e.doc.objects.values().filter_map(|o| o.as_panel()).filter(|p| p.role == aic_domain::PanelRole::Door && e.doc.cabinet_of(p.id) == Some(a)).count();
    assert_eq!(doors, 2);
    let door = e.doc.objects.values().filter_map(|o| o.as_panel()).find(|p| p.role == aic_domain::PanelRole::Door && e.doc.cabinet_of(p.id) == Some(a)).unwrap().id;
    assert!(call(&mut e, json!({"cmd": "set_parameter", "id": door, "name": "door_mount", "value": "INSET"})).ok);
    // P12: stretch the door +20 at the top (co giãn).
    let h0 = e.doc.panel(door).unwrap().height_mm;
    assert!(call(&mut e, json!({"cmd": "set_part_mod", "id": door, "patch": {"extend_delta": [0, 0, 0, 20]}})).ok);
    assert!((e.doc.panel(door).unwrap().height_mm - h0 - 20.0).abs() < 1e-9);
    // Delete the shelf → zone merges; undo brings it back with the same id.
    assert!(call(&mut e, json!({"cmd": "delete_objects", "ids": [shelf]})).ok);
    assert!(e.doc.panel(shelf).is_none());
    assert!(call(&mut e, json!({"cmd": "undo"})).ok);
    assert!(e.doc.panel(shelf).is_some());
    // Deleting a carcass part marks it deleted in the definition.
    let bottom = e.doc.objects.values().filter_map(|o| o.as_panel()).find(|p| p.role == aic_domain::PanelRole::Bottom && e.doc.cabinet_of(p.id) == Some(b)).unwrap().id;
    assert!(call(&mut e, json!({"cmd": "delete_objects", "ids": [bottom]})).ok);
    assert!(e.doc.panel(bottom).is_none());
    let r = call(&mut e, json!({"cmd": "zone_add_link", "cabinet": b, "zones": [z.result["zones"][0]["id"]], "kind": "OVAL_RAIL"}));
    assert!(r.ok, "{:?}", r.error);
}

#[test]
fn costing_report_wardrobe() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);
    let r = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(r.ok, "{:?}", r.error);
    let c = &r.result;
    let fit = |name: &str| c["fittings"].as_array().unwrap().iter().find(|l| l["name"].as_str().unwrap().starts_with(name)).map(|l| l["qty"].as_f64().unwrap());
    assert_eq!(fit("Bản lề"), Some(16.0), "4 doors × 4 hinges");
    assert_eq!(fit("Chốt tầng"), Some(16.0), "4 adjustable shelves × 4 pins");
    assert_eq!(fit("Chén oval"), Some(2.0));
    assert!(fit("Thanh Oval").is_some());
    assert!(fit("Cam & Dowel").unwrap() > 0.0);
    // Panels grouped by material and thickness; the 8.6 back is never banded.
    let panels = c["panels"].as_array().unwrap();
    assert!(panels.iter().any(|l| l["name"].as_str().unwrap().contains("8.6")));
    let back = c["cut_list"].as_array().unwrap().iter().find(|r| r["name"] == "Hậu").unwrap();
    assert!(back["edges"].as_array().unwrap().is_empty());
    let side = c["cut_list"].as_array().unwrap().iter().find(|r| r["name"] == "HồiTrái").unwrap();
    assert!(!side["edges"].as_array().unwrap().is_empty(), "exposed front edge of a side is banded");
    assert!(c["edges"][0]["qty"].as_f64().unwrap() > 10.0);
    assert!(c["totals"]["total"].as_f64().unwrap() > 0.0);
    // Unit price edit is undoable.
    assert!(call(&mut e, json!({"cmd": "set_price", "key": "panel:MDF17-WHITE|17.2", "value": 250000})).ok);
    let r2 = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(r2.result["totals"]["panels"].as_f64().unwrap() > 0.0);
    assert!(call(&mut e, json!({"cmd": "undo"})).ok);
    assert_eq!(c["cabinets"][0]["name"], "TủQA01");
    let _ = cab;
}

#[test]
fn floors_and_rooms_place_cabinets_in_their_own_area() {
    let mut e = Engine::new();
    let mk = |e: &mut Engine, floor: &str, room: &str| -> ObjectId {
        let r = call(e, json!({"cmd": "create_cabinet", "kind": "BASE", "floor": floor, "room": room}));
        serde_json::from_value(r.result["id"].clone()).unwrap()
    };
    let x = |e: &Engine, id: ObjectId| e.doc.param_value(id, "x").unwrap();
    let a = mk(&mut e, "Tầng 1", "Bếp");
    let b = mk(&mut e, "Tầng 1", "Bếp");
    assert_eq!(x(&e, b), x(&e, a) + 800.0, "same room continues the row");
    let c = mk(&mut e, "Tầng 1", "Khách");
    assert_eq!(x(&e, c), x(&e, b) + 800.0 + 1500.0, "new room on the floor: own area");
    let d = mk(&mut e, "Tầng 2", "PN1");
    assert_eq!(x(&e, d), x(&e, c) + 800.0 + 3000.0, "new floor: own area");
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let node = tree.result["roots"].as_array().unwrap().iter().find(|n| n["id"] == json!(d)).unwrap().clone();
    assert_eq!(node["floor"], "Tầng 2");
    assert_eq!(node["room"], "PN1");
    // Moving a cabinet to another floor is an undoable parameter change.
    call(&mut e, json!({"cmd": "set_parameter", "id": d, "name": "floor", "value": "Tầng 3"}));
    assert_eq!(e.doc.object(d).unwrap().as_cabinet().unwrap().floor, "Tầng 3");
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(e.doc.object(d).unwrap().as_cabinet().unwrap().floor, "Tầng 2");
}

#[test]
fn shape_tools_corner_cut_merge() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let by = |pfx: &str| -> Vec<ObjectId> {
        kids.iter().filter(|n| n["name"].as_str().unwrap().starts_with(pfx)).map(|n| serde_json::from_value(n["id"].clone()).unwrap()).collect()
    };
    let doors = by("CửaĐôi");
    assert!(doors.len() >= 2);
    let outer = |e: &Engine, id: ObjectId| {
        let p = e.doc.panel(id).unwrap();
        p.features.iter().chain(p.gen_features.iter()).any(|f| matches!(f, aic_domain::MachiningFeature::Contour(c) if !c.inner))
    };

    // 10. Bo góc on a door: outline stored in the cabinet's part mods; mesh builds; undo restores.
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [doors[0]], "op": {"kind": "CORNERS", "corners": ["TOP_LEFT", "TOP_RIGHT"], "size": 40}}));
    assert!(r.ok, "{:?}", r.error);
    assert!(outer(&e, doors[0]));
    let m = call(&mut e, json!({"cmd": "get_render_objects", "ids": [doors[0]]}));
    assert!(m.ok, "{:?}", m.error);
    let props = call(&mut e, json!({"cmd": "get_properties", "id": doors[0]}));
    assert!(props.result.to_string().contains("10. Bo/Vác góc"), "tool listed on the part");
    call(&mut e, json!({"cmd": "undo"}));
    assert!(!outer(&e, doors[0]));
    // Too large a radius is a clear error.
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [doors[0]], "op": {"kind": "CORNERS", "corners": ["TOP_LEFT"], "size": 5000}}));
    assert_eq!(r.error.unwrap().code, "INVALID_PARAMETER");

    // 09. Cắt theo tấm: the divider does not cross a door → error; the top crosses the sides → notch.
    let left = by("HồiTrái")[0];
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [doors[0]], "op": {"kind": "CUT_BY_PANEL", "cutter": left, "clearance": 0}}));
    assert!(!r.ok);

    // 06. Hợp tấm: the two leaves of a double door do not touch; stretched so they overlap they merge.
    let r = call(&mut e, json!({"cmd": "merge_panels", "ids": [doors[0], doors[1]]}));
    assert!(!r.ok, "gap between leaves");
    let before = e.doc.panel(doors[0]).unwrap().width_mm;
    let w1 = e.doc.panel(doors[1]).unwrap().width_mm;
    call(&mut e, json!({"cmd": "set_part_mod", "id": doors[0], "patch": {"extend_delta": [0, 10, 0, 0]}}));
    call(&mut e, json!({"cmd": "set_part_mod", "id": doors[0], "patch": {"extend_delta": [10, 0, 0, 0]}}));
    let r = call(&mut e, json!({"cmd": "merge_panels", "ids": [doors[0], doors[1]]}));
    if r.ok {
        let w = e.doc.panel(doors[0]).unwrap().width_mm;
        assert!(w > before + w1 - 1.0, "merged width {w}");
        assert!(e.doc.panel(doors[1]).is_none(), "second leaf deleted");
    } else {
        // Leaves hinge on opposite sides: stretching the left edge moved it away — merge the other way.
        let r = call(&mut e, json!({"cmd": "merge_panels", "ids": [doors[1], doors[0]]}));
        assert!(r.ok, "{:?}", r.error);
    }
    let _ = cab;
}

#[test]
fn chia_tam_splits_a_part_into_pieces() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);
    let names = |e: &mut Engine| -> Vec<(ObjectId, String)> {
        let t = call(e, json!({"cmd": "get_scene_tree"}));
        t.result["roots"][0]["children"].as_array().unwrap().iter().map(|n| (serde_json::from_value(n["id"].clone()).unwrap(), n["name"].as_str().unwrap().to_string())).collect()
    };
    let back = names(&mut e).into_iter().find(|(_, n)| n == "Hậu").unwrap().0;
    let h = e.doc.panel(back).unwrap().height_mm;
    let r = call(&mut e, json!({"cmd": "set_part_mod", "id": back, "patch": {"split": {"axis": "Y", "count": 3, "gap": 2}}}));
    assert!(r.ok, "{:?}", r.error);
    let pieces: Vec<ObjectId> = names(&mut e).into_iter().filter(|(_, n)| n.starts_with("Hậu.")).map(|(i, _)| i).collect();
    assert_eq!(pieces.len(), 3);
    let ph = e.doc.panel(pieces[0]).unwrap().height_mm;
    assert!((ph * 3.0 + 4.0 - h).abs() < 1e-6, "{ph}");
    assert_eq!(pieces[0], back, "first piece keeps the id");
    // Thickness back to the generated value with an explicit null.
    call(&mut e, json!({"cmd": "set_part_mod", "id": back, "patch": {"thickness": 17.2}}));
    call(&mut e, json!({"cmd": "set_part_mod", "id": back, "patch": {"thickness": null}}));
    assert!((e.doc.panel(back).unwrap().thickness_mm - 8.6).abs() < 1e-9);
    // Remove the split.
    call(&mut e, json!({"cmd": "set_part_mod", "id": back, "patch": {"split": null}}));
    assert_eq!(names(&mut e).iter().filter(|(_, n)| n.starts_with("Hậu")).count(), 1);
    assert!((e.doc.panel(back).unwrap().height_mm - h).abs() < 1e-9);
    let _ = cab;
}

fn changed_ids(r: &Response) -> std::collections::BTreeSet<ObjectId> {
    let mut s = std::collections::BTreeSet::new();
    for e in &r.events {
        match e {
            CoreEvent::ObjectChanged { ids } | CoreEvent::GeometryChanged { ids } | CoreEvent::TransformChanged { ids } => s.extend(ids.iter().copied()),
            _ => {}
        }
    }
    s
}

fn bays(e: &mut Engine, cab: ObjectId, zone: u64) -> Vec<(f64, String)> {
    let z = call(e, json!({"cmd": "get_zones", "cabinet": cab}));
    let mut v: Vec<(u64, f64, String)> = z.result["bays"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["zone"].as_u64() == Some(zone))
        .map(|b| (b["index"].as_u64().unwrap(), b["size"].as_f64().unwrap(), b["mode"].as_str().unwrap_or("-").to_string()))
        .collect();
    v.sort_by_key(|x| x.0);
    v.into_iter().map(|x| (x.1, x.2)).collect()
}

#[test]
fn parametric_a_resize_keeps_locked_bays_and_anchor() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e); // TủQA 1600: root split by one divider
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let root = z.result["zones"][0]["id"].as_u64().unwrap();
    // Third bay: another divider in the root split.
    let r = call(&mut e, json!({"cmd": "zone_add_panels", "cabinet": cab, "zones": [root], "kind": "DIVIDER", "count": 1, "lock": "EVEN", "value": 0}));
    assert!(r.ok, "{:?}", r.error);
    for (i, mode, v) in [(0, "LOCK", 600.0), (2, "LOCK", 400.0)] {
        let r = call(&mut e, json!({"cmd": "set_bay", "cabinet": cab, "zone": root, "index": i, "mode": mode, "value": v}));
        assert!(r.ok, "{:?}", r.error);
    }
    call(&mut e, json!({"cmd": "set_bay", "cabinet": cab, "zone": root, "index": 1, "mode": "AUTO"}));
    let b = bays(&mut e, cab, root);
    assert_eq!((b[0].0, b[2].0), (600.0, 400.0));
    assert_eq!(b[1].1, "AUTO");
    let auto_before = b[1].0;

    // Keep center, 1600 → 1800.
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "anchor_w", "value": "CENTER"}));
    let x0 = e.doc.param_value(cab, "x").unwrap();
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "1800"}));
    assert!(r.ok, "{:?}", r.error);
    let b = bays(&mut e, cab, root);
    assert_eq!((b[0].0, b[2].0), (600.0, 400.0), "locked bays keep their size");
    assert!((b[1].0 - auto_before - 200.0).abs() < 1e-6, "AUTO bay takes the whole change");
    assert!((e.doc.param_value(cab, "x").unwrap() - (x0 - 100.0)).abs() < 1e-6, "center anchor");
    // One undo step restores size and position.
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(e.doc.param_value(cab, "width"), Some(1600.0));
    assert!((e.doc.param_value(cab, "x").unwrap() - x0).abs() < 1e-6);
    // Too narrow for the locked bays → refused, nothing changes.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "1000"}));
    assert_eq!(r.error.unwrap().code, "CONSTRAINT_VIOLATED");
    assert_eq!(e.doc.param_value(cab, "width"), Some(1600.0));
}

#[test]
fn parametric_b_c_drag_divider_and_shelf() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let root = z.result["zones"][0]["id"].as_u64().unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let id_of = |n: &str| -> ObjectId { serde_json::from_value(kids.iter().find(|k| k["name"] == n).unwrap()["id"].clone()).unwrap() };
    let total = kids.len();

    // B. Divider: left bay 774.2 → 900.
    let divider = id_of("HôngGiữa_01");
    let r = call(&mut e, json!({"cmd": "move_split_panel", "id": divider, "before": 900}));
    assert!(r.ok, "{:?}", r.error);
    let b = bays(&mut e, cab, root);
    assert!((b[0].0 - 900.0).abs() < 1e-6 && (b[0].0 + b[1].0 - 2.0 * 774.2).abs() < 1e-6, "{b:?}");
    let changed = changed_ids(&r);
    assert!(changed.contains(&divider));
    assert!(changed.len() < total, "only affected parts change ({} of {total})", changed.len());

    // C. Shelf in the left bay: bay below → 1250; only its split changes.
    let shelf = id_of("KệDiĐộng_02");
    let zi = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let pos = zi.result["positions"].as_array().unwrap().iter().find(|p| p["zone"].as_u64() != Some(root)).unwrap().clone();
    let shelf_zone = pos["zone"].as_u64().unwrap();
    let before = bays(&mut e, cab, shelf_zone);
    let idx = 1; // KệDiĐộng_02 closes bay 1
    let target = before[idx].0 + 60.0;
    let r = call(&mut e, json!({"cmd": "move_split_panel", "id": shelf, "before": target}));
    assert!(r.ok, "{:?}", r.error);
    let after = bays(&mut e, cab, shelf_zone);
    assert!((after[idx].0 - target).abs() < 1e-6, "{after:?}");
    assert_eq!(bays(&mut e, cab, root), b, "the divider split is untouched");
    let changed = changed_ids(&r);
    // The shelf's own bay neighbours (left side, divider) get new pin holes; the other bay does not change.
    assert!(!changed.contains(&id_of("HồiPhải")) && !changed.contains(&id_of("Nóc")), "no unrelated part changes");
    assert!(changed.contains(&shelf));
    // Dragging past the neighbour is refused.
    let r = call(&mut e, json!({"cmd": "move_split_panel", "id": shelf, "before": 5000}));
    assert!(!r.ok);
}

#[test]
fn parametric_e_multi_edit_shelves() {
    let mut e = Engine::new();
    let _cab = created_cabinet(&mut e);
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let shelves: Vec<ObjectId> = kids.iter().filter(|k| k["name"].as_str().unwrap().starts_with("KệDiĐộng")).map(|k| serde_json::from_value(k["id"].clone()).unwrap()).collect();
    let id_of = |n: &str| -> ObjectId { serde_json::from_value(kids.iter().find(|k| k["name"] == n).unwrap()["id"].clone()).unwrap() };
    assert!(shelves.len() >= 4);
    let right = id_of("HồiPhải");

    // Mixed display: widths equal, thickness equal; set one shelf thicker → mixed.
    call(&mut e, json!({"cmd": "set_parameter", "id": shelves[0], "name": "thickness", "value": "25"}));
    let m = call(&mut e, json!({"cmd": "get_properties_multi", "ids": shelves}));
    assert!(m.ok, "{:?}", m.error);
    let s = m.result.to_string();
    assert!(s.contains("\"mixed\":true"), "thickness differs → mixed");

    // One multi-edit = one undo step; only the shelves' split changes.
    let r = call(&mut e, json!({"cmd": "set_parameter_multi", "ids": shelves, "name": "thickness", "value": "18"}));
    assert!(r.ok, "{:?}", r.error);
    assert!(!changed_ids(&r).contains(&right));
    for id in &shelves {
        assert!((e.doc.panel(*id).unwrap().thickness_mm - 18.0).abs() < 1e-9);
    }
    let r = call(&mut e, json!({"cmd": "set_parameter_multi", "ids": shelves, "name": "off_front", "value": "30"}));
    assert!(r.ok, "{:?}", r.error);
    let d0 = e.doc.panel(shelves[1]).unwrap().height_mm; // shelf local Y = depth
    call(&mut e, json!({"cmd": "undo"}));
    let d1 = e.doc.panel(shelves[1]).unwrap().height_mm;
    assert!((d1 - d0 - 30.0).abs() < 1e-6, "front offset shortened the shelf by 30 ({d1} vs {d0})");
    call(&mut e, json!({"cmd": "undo"}));
    for id in &shelves[1..] {
        assert!((e.doc.panel(*id).unwrap().thickness_mm - 17.2).abs() < 1e-9, "one undo restores every shelf");
    }
    // All or nothing: an invalid value on the group changes nothing.
    let r = call(&mut e, json!({"cmd": "set_parameter_multi", "ids": shelves, "name": "thickness", "value": "500"}));
    assert!(!r.ok);
    assert!((e.doc.panel(shelves[1]).unwrap().thickness_mm - 17.2).abs() < 1e-9);
}
