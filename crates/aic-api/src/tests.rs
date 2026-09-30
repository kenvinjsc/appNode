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
    assert!(fit("Chốt gỗ").unwrap() > 0.0);
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

#[test]
fn parametric_f_template_preset_mirror_array() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let root = z.result["zones"][0]["id"].as_u64().unwrap();
    call(&mut e, json!({"cmd": "zone_add_panels", "cabinet": cab, "zones": [root], "kind": "DIVIDER", "count": 1, "lock": "EVEN", "value": 0}));
    call(&mut e, json!({"cmd": "set_bay", "cabinet": cab, "zone": root, "index": 0, "mode": "LOCK", "value": 600}));
    call(&mut e, json!({"cmd": "set_bay", "cabinet": cab, "zone": root, "index": 2, "mode": "LOCK", "value": 400}));
    call(&mut e, json!({"cmd": "set_bay", "cabinet": cab, "zone": root, "index": 1, "mode": "AUTO"}));
    let r = call(&mut e, json!({"cmd": "save_template", "cabinet": cab, "name": "Tủ áo 3 khoang"}));
    assert!(r.ok, "{:?}", r.error);
    let list = call(&mut e, json!({"cmd": "get_templates"}));
    assert_eq!(list.result["templates"][0]["name"], "Tủ áo 3 khoang");
    assert!(list.result["presets"].as_array().unwrap().iter().any(|p| p["name"] == "AIC Wardrobe Standard"));

    // Insert 2000 × 2200: locked bays keep their mm, structure intact, one undo step.
    let before = e.doc.objects.len();
    let r = call(&mut e, json!({"cmd": "insert_template", "name": "Tủ áo 3 khoang", "width": 2000, "height": 2200, "room": "PN2"}));
    assert!(r.ok, "{:?}", r.error);
    let id: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    assert_eq!(e.doc.param_value(id, "width"), Some(2000.0));
    let zi = call(&mut e, json!({"cmd": "get_zones", "cabinet": id}));
    let root2 = zi.result["zones"][0]["id"].as_u64().unwrap();
    let b = bays(&mut e, id, root2);
    assert_eq!(b.len(), 3);
    assert_eq!((b[0].0, b[2].0), (600.0, 400.0));
    assert!((b[1].0 - (2000.0 - 4.0 * 17.2 - 1000.0)).abs() < 1e-6, "{b:?}");
    assert_eq!(zi.result["room"], "PN2");
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(e.doc.objects.len(), before, "one undo removes the inserted cabinet");
    // Too narrow for 600 + 400 locked bays: refused, nothing created.
    let r = call(&mut e, json!({"cmd": "insert_template", "name": "Tủ áo 3 khoang", "width": 900}));
    assert_eq!(r.error.unwrap().code, "CONSTRAINT_VIOLATED");
    assert_eq!(e.doc.objects.len(), before);

    // Rule preset.
    let r = call(&mut e, json!({"cmd": "apply_rule_preset", "ids": [cab], "name": "AIC Wardrobe Standard"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.param_value(cab, "shelf_setback"), Some(30.0));
    assert_eq!(e.doc.param_value(cab, "door_gap"), Some(2.0));

    // Mirror: bays reverse (400 | AUTO | 600).
    call(&mut e, json!({"cmd": "mirror_cabinet", "id": cab}));
    let b = bays(&mut e, cab, root);
    assert_eq!((b[0].0, b[2].0), (400.0, 600.0));

    // Array: 2 more cabinets to the right.
    let n = e.doc.objects.values().filter(|o| o.as_cabinet().is_some()).count();
    let r = call(&mut e, json!({"cmd": "array_cabinet", "id": cab, "count": 2, "axis": 0, "gap": 0}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.objects.values().filter(|o| o.as_cabinet().is_some()).count(), n + 2);
}

#[test]
fn parametric_d_dynamic_anchor_follows_moved_side() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"shelves": 1, "doors": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let id_of = |pfx: &str| -> ObjectId { serde_json::from_value(kids.iter().find(|k| k["name"].as_str().unwrap().starts_with(pfx)).unwrap()["id"].clone()).unwrap() };
    let (shelf, right) = (id_of("KệDiĐộng"), id_of("HồiPhải"));
    let boxes = |e: &Engine| {
        let l = e.doc.cabinet_layout(cab).unwrap();
        let get = |pfx: &str| aic_domain::layout::part_aabb(l.parts.iter().find(|p| p.name.starts_with(pfx)).unwrap());
        (get("KệDiĐộng"), get("HồiPhải"))
    };

    // Shelf.Right → HồiPhải.Inner, 0 mm.
    let r = call(&mut e, json!({"cmd": "set_part_mod", "id": shelf, "patch": {"add_anchor": {"edge": "RIGHT", "target": right, "face": "INNER", "offset": 0}}}));
    assert!(r.ok, "{:?}", r.error);
    let ((_, smx), (rmn, _)) = boxes(&e);
    assert!((smx[0] - rmn[0]).abs() < 1e-6, "shelf touches the right side's inner face");
    let props = call(&mut e, json!({"cmd": "get_properties", "id": shelf}));
    assert!(props.result.to_string().contains("\"target_name\":\"HồiPhải\""));

    // Move the right side 30 mm inward: the shelf follows (shorter by 30).
    let w0 = e.doc.panel(shelf).unwrap().width_mm;
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": right, "name": "off_right", "value": "30"}));
    assert!(r.ok, "{:?}", r.error);
    let ((_, smx), (rmn, _)) = boxes(&e);
    assert!((smx[0] - rmn[0]).abs() < 1e-6);
    assert!((e.doc.panel(shelf).unwrap().width_mm - (w0 - 30.0)).abs() < 1e-6);
    // Offset keeps a gap; removing the anchor restores the zone width.
    call(&mut e, json!({"cmd": "set_part_mod", "id": shelf, "patch": {"add_anchor": {"edge": "RIGHT", "target": right, "offset": 2}}}));
    let ((_, smx), (rmn, _)) = boxes(&e);
    assert!((rmn[0] - smx[0] - 2.0).abs() < 1e-6);
    call(&mut e, json!({"cmd": "set_part_mod", "id": shelf, "patch": {"remove_anchor": 0}}));
    assert!(e.doc.panel(shelf).unwrap().width_mm > w0 - 1.0, "back to the zone size");
}

#[test]
fn drawer_heights_and_door_side_gaps() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"drawers": 3, "doors": 0, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let fronts = |e: &mut Engine| -> (u64, Vec<f64>) {
        let z = call(e, json!({"cmd": "get_zones", "cabinet": cab}));
        let fb = z.result["front_bays"].as_array().unwrap().clone();
        (fb[0]["uid"].as_u64().unwrap(), fb.iter().map(|b| b["size"].as_f64().unwrap()).collect())
    };
    let (uid, h) = fronts(&mut e);
    assert_eq!(h.len(), 3);
    // Bottom drawer LOCK 150; the others share the rest.
    let r = call(&mut e, json!({"cmd": "set_drawer_height", "cabinet": cab, "uid": uid, "index": 0, "mode": "LOCK", "value": 150}));
    assert!(r.ok, "{:?}", r.error);
    let (_, h2) = fronts(&mut e);
    assert!((h2[0] - 150.0).abs() < 1e-6 && (h2[1] - h2[2]).abs() < 1e-6, "{h2:?}");
    // Taller cabinet: locked drawer keeps 150.
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "height", "value": "900"}));
    let (_, h3) = fronts(&mut e);
    assert!((h3[0] - 150.0).abs() < 1e-6 && h3[1] > h2[1]);
    // Drag the divider between drawer 2 and 3: drawer 2 = 250.
    let r = call(&mut e, json!({"cmd": "move_drawer_divider", "cabinet": cab, "uid": uid, "index": 1, "before": 250}));
    assert!(r.ok, "{:?}", r.error);
    let (_, h4) = fronts(&mut e);
    assert!((h4[1] - 250.0).abs() < 1e-6 && (h4[0] - 150.0).abs() < 1e-6, "{h4:?}");
    // Box follows its front: 3 drawer boxes still generated.
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let n = tree.result.to_string().matches("ThànhTrái").count();
    assert_eq!(n, 3);

    // Door side gaps on a wardrobe door.
    let wr = created_cabinet(&mut e);
    let t = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let door: ObjectId = serde_json::from_value(
        t.result["roots"].as_array().unwrap().iter().find(|r| r["id"] == json!(wr)).unwrap()["children"].as_array().unwrap().iter().find(|k| k["name"].as_str().unwrap().starts_with("CửaĐôi")).unwrap()["id"].clone(),
    )
    .unwrap();
    let w0 = e.doc.panel(door).unwrap().width_mm;
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": door, "name": "door_gap_left", "value": "10"}));
    assert!(r.ok, "{:?}", r.error);
    assert!(e.doc.panel(door).unwrap().width_mm < w0, "a wider left reveal narrows the doors");
}

#[test]
fn relations_overlay_inset_gap_flush_and_edge_drag() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"shelves": 1, "doors": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let id_of = |pfx: &str| -> ObjectId { serde_json::from_value(kids.iter().find(|k| k["name"].as_str().unwrap().starts_with(pfx)).unwrap()["id"].clone()).unwrap() };
    let (bottom, left, shelf) = (id_of("Đáy"), id_of("HồiTrái"), id_of("KệDiĐộng"));
    let bx = |e: &Engine, pfx: &str| {
        let l = e.doc.cabinet_layout(cab).unwrap();
        aic_domain::layout::part_aabb(l.parts.iter().find(|p| p.name.starts_with(pfx)).unwrap())
    };
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;

    // Phủ: the bottom runs under the left side, the side sits on the bottom.
    let r = call(&mut e, json!({"cmd": "set_relation", "a": bottom, "b": left, "kind": "OVERLAY"}));
    assert!(r.ok, "{:?}", r.error);
    let (b, l) = (bx(&e, "Đáy"), bx(&e, "HồiTrái"));
    assert!(near(b.0[0], l.0[0]), "bottom reaches the side's outer face");
    assert!(near(l.0[1], b.1[1]), "side stands on the bottom");
    // Lọt.
    call(&mut e, json!({"cmd": "set_relation", "a": bottom, "b": left, "kind": "INSET"}));
    let (b, l) = (bx(&e, "Đáy"), bx(&e, "HồiTrái"));
    assert!(near(b.0[0], l.1[0]) && near(l.0[1], b.0[1]));
    // Khe 3 mm.
    call(&mut e, json!({"cmd": "set_relation", "a": bottom, "b": left, "kind": "GAP", "gap": 3}));
    let (b, l) = (bx(&e, "Đáy"), bx(&e, "HồiTrái"));
    assert!(near(b.0[0] - l.1[0], 3.0));
    // Bằng mặt: the shelf's front edge flush with the side's front edge.
    let r = call(&mut e, json!({"cmd": "set_relation", "a": shelf, "b": left, "kind": "FLUSH"}));
    assert!(r.ok, "{:?}", r.error);
    let (s, l) = (bx(&e, "KệDiĐộng"), bx(&e, "HồiTrái"));
    assert!(near(s.1[2], l.1[2]), "front flush {} vs {}", s.1[2], l.1[2]);

    // Kéo cạnh: free drag of the shelf's right side +20 → 20 mm longer.
    let w0 = e.doc.panel(shelf).unwrap().width_mm;
    let r = call(&mut e, json!({"cmd": "resize_panel_side", "id": shelf, "side": "RIGHT", "delta": 20}));
    assert!(r.ok, "{:?}", r.error);
    assert!(near(e.doc.panel(shelf).unwrap().width_mm, w0 + 20.0));
    // Constrained: the right edge anchored to the right side keeps the anchor, gap changes.
    let right = id_of("HồiPhải");
    call(&mut e, json!({"cmd": "set_part_mod", "id": shelf, "patch": {"add_anchor": {"edge": "RIGHT", "target": right, "offset": 10}}}));
    call(&mut e, json!({"cmd": "resize_panel_side", "id": shelf, "side": "RIGHT", "delta": 4, "constrained": true}));
    let (s, rr) = (bx(&e, "KệDiĐộng"), bx(&e, "HồiPhải"));
    assert!(near(rr.0[0] - s.1[0], 6.0), "gap 10 − 4 = 6");
    // Undo restores in one step each; NONE removes the bottom/side relation.
    let r = call(&mut e, json!({"cmd": "set_relation", "a": bottom, "b": left, "kind": "NONE"}));
    assert!(r.ok);
}

#[test]
fn contour_arc_and_free_polygon_on_a_part() {
    let mut e = Engine::new();
    let _cab = created_cabinet(&mut e);
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let door: ObjectId = serde_json::from_value(kids.iter().find(|k| k["name"].as_str().unwrap().starts_with("CửaĐôi")).unwrap()["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [door], "op": {"kind": "EDGE_ARC", "edge": "TOP", "sagitta": -40}}));
    assert!(r.ok, "{:?}", r.error);
    // A free hole (triangle) inside the door and a region cut from the bottom corner.
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [door], "op": {"kind": "POLYGON", "mode": "HOLE", "points": [[100, 1000], [250, 1000], [175, 1150]]}}));
    assert!(r.ok, "{:?}", r.error);
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [door], "op": {"kind": "POLYGON", "mode": "SUBTRACT", "points": [[-10, -10], [120, -10], [-10, 120]]}}));
    assert!(r.ok, "{:?}", r.error);
    // A hole outside the panel is refused; a bow-tie outline too.
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [door], "op": {"kind": "POLYGON", "mode": "HOLE", "points": [[-100, 0], [50, 0], [0, 50]]}}));
    assert!(!r.ok);
    let r = call(&mut e, json!({"cmd": "shape_tool", "ids": [door], "op": {"kind": "POLYGON", "mode": "OUTLINE", "points": [[0, 0], [300, 300], [300, 0], [0, 300]]}}));
    assert!(!r.ok);
    // Geometry and the flat pattern (CNC) build.
    let m = call(&mut e, json!({"cmd": "get_render_objects", "ids": [door]}));
    assert!(m.ok, "{:?}", m.error);
    let f = call(&mut e, json!({"cmd": "get_manufacturing", "id": door}));
    assert!(f.ok, "{:?}", f.error);
    assert!(f.result["inner"].as_array().is_some_and(|v| !v.is_empty()), "hole in the flat pattern");
}

#[test]
fn row_of_cabinets_follows_a_width_change() {
    let mut e = Engine::new();
    let id = |r: &Response| -> ObjectId { serde_json::from_value(r.result["id"].clone()).unwrap() };
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "position": [1000, 0, 0]}));
    let a = id(&r);
    let b = id(&call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "after": a})));
    let c = id(&call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "after": b})));
    // A wall cabinet above A and a loose cabinet with a gap: not part of the row.
    let wall = id(&call(&mut e, json!({"cmd": "create_cabinet", "kind": "WALL", "position": [1000, 1450, 0]})));
    let loose = id(&call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "position": [3500, 0, 0]})));
    let x = |e: &Engine, i: ObjectId| e.doc.param_value(i, "x").unwrap();
    let (xb, xc, xw, xl) = (x(&e, b), x(&e, c), x(&e, wall), x(&e, loose));

    // Keep left: A grows 200 → B and C pushed right, the others stay.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": a, "name": "width", "value": "1000"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!((x(&e, b), x(&e, c)), (xb + 200.0, xc + 200.0));
    assert_eq!((x(&e, wall), x(&e, loose)), (xw, xl));
    // One undo puts everything back.
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!((x(&e, b), x(&e, c)), (xb, xc));

    // Keep right on B: B grows 100 to the left → A pulled left, C stays.
    call(&mut e, json!({"cmd": "set_parameter", "id": b, "name": "anchor_w", "value": "END"}));
    let xa = x(&e, a);
    call(&mut e, json!({"cmd": "set_parameter", "id": b, "name": "width", "value": "900"}));
    assert_eq!((x(&e, a), x(&e, b), x(&e, c)), (xa - 100.0, xb - 100.0, xc));
    // Shrinking closes the row the same way (C follows when anchored left).
    call(&mut e, json!({"cmd": "set_parameter", "id": b, "name": "anchor_w", "value": "START"}));
    call(&mut e, json!({"cmd": "set_parameter", "id": b, "name": "width", "value": "600"}));
    assert_eq!(x(&e, c), xc - 300.0);
}

#[test]
fn structure_tabs_back_rails_plinth_and_group_presets() {
    let mut e = Engine::new();
    let lib = std::env::temp_dir().join(format!("aic-lib-test-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&lib);
    e.set_library_path(Some(lib.clone()));
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"doors": 0, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let s = call(&mut e, json!({"cmd": "get_structure", "cabinet": cab}));
    assert!(s.ok, "{:?}", s.error);
    let titles: Vec<&str> = s.result["tabs"].as_array().unwrap().iter().map(|t| t["title"].as_str().unwrap()).collect();
    assert!(titles.contains(&"Hậu") && titles.contains(&"Thanh giằng (trên)"));
    let count = |e: &Engine, pfx: &str| e.doc.cabinet_layout(cab).unwrap().parts.iter().filter(|p| p.name.starts_with(pfx)).count();

    // Hậu: rãnh 9, dày 6, lùi 17, chia dọc mỗi tấm ≤ 400 → 2 tấm (800 − 2×17.2 + 2×9 = 783.6).
    for (k, v) in [("back_groove", "9"), ("back_thickness", "6"), ("back_offset", "17"), ("back_split", "on"), ("back_split_formula", "400")] {
        let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": k, "value": v}));
        assert!(r.ok, "{k}: {:?}", r.error);
    }
    assert_eq!(count(&e, "Hậu"), 2);
    // Khe hở 1: the back enters 8 of the 9 mm groove.
    let w0: f64 = e.doc.cabinet_layout(cab).unwrap().parts.iter().filter(|p| p.name.starts_with("Hậu")).map(|p| p.size[0]).sum();
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "back_clearance", "value": "1"}));
    let w1: f64 = e.doc.cabinet_layout(cab).unwrap().parts.iter().filter(|p| p.name.starts_with("Hậu")).map(|p| p.size[0]).sum();
    assert!((w0 - w1 - 2.0).abs() < 1e-6);

    // Thanh giằng trên: 2 front rails standing (33), 1 extra.
    for (k, v) in [("rt_front_count", "2"), ("rt_front_size", "33"), ("rt_front_horizontal", "off"), ("rt_extra_count", "1")] {
        assert!(call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": k, "value": v})).ok);
    }
    assert_eq!(count(&e, "GiằngTrước"), 2);
    assert_eq!(count(&e, "GiằngGiữa"), 1);

    // Mẫu tab: save "Hậu 9-6-17", apply to a second cabinet in one undo step.
    let r = call(&mut e, json!({"cmd": "save_group_preset", "cabinet": cab, "group": "back", "name": "Hậu 9-6-17"}));
    assert!(r.ok, "{:?}", r.error);
    assert!(std::fs::read_to_string(&lib).unwrap().contains("Hậu 9-6-17"), "stored in the library file");
    let r2 = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"doors": 0, "shelves": 0}}));
    let cab2: ObjectId = serde_json::from_value(r2.result["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "apply_group_preset", "ids": [cab2], "group": "back", "name": "Hậu 9-6-17"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.param_value(cab2, "back_offset"), Some(17.0));
    assert_eq!(e.doc.cabinet_layout(cab2).unwrap().parts.iter().filter(|p| p.name.starts_with("Hậu")).count(), 2);
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(e.doc.param_value(cab2, "back_offset"), Some(0.0));

    // A fresh engine on the same library file sees the preset (reuse in another project).
    let mut e2 = Engine::new();
    e2.set_library_path(Some(lib.clone()));
    let r3 = call(&mut e2, json!({"cmd": "create_cabinet", "kind": "BASE"}));
    let c3: ObjectId = serde_json::from_value(r3.result["id"].clone()).unwrap();
    let s = call(&mut e2, json!({"cmd": "get_structure", "cabinet": c3}));
    assert!(s.result.to_string().contains("Hậu 9-6-17"));
    let _ = std::fs::remove_file(&lib);
}

#[test]
fn zone_preset_saved_and_applied_to_another_cabinet() {
    let mut e = Engine::new();
    let a = created_cabinet(&mut e);
    let za = call(&mut e, json!({"cmd": "get_zones", "cabinet": a}));
    let root = za.result["zones"][0]["id"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "save_zone_preset", "cabinet": a, "zone": root, "name": "Tủ áo 2 khoang"}));
    assert!(r.ok, "{:?}", r.error);
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"doors": 0, "shelves": 0, "width": 1200, "height": 2000}}));
    let b: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let zb = call(&mut e, json!({"cmd": "get_zones", "cabinet": b}));
    let rb = zb.result["zones"][0]["id"].as_u64().unwrap();
    let before = e.doc.cabinet_layout(b).unwrap().parts.len();
    let r = call(&mut e, json!({"cmd": "apply_zone_preset", "cabinet": b, "zones": [rb], "name": "Tủ áo 2 khoang"}));
    assert!(r.ok, "{:?}", r.error);
    let parts = e.doc.cabinet_layout(b).unwrap().parts;
    assert!(parts.len() > before + 5, "divider, shelves, doors, rail copied");
    assert!(parts.iter().any(|p| p.name.starts_with("HôngGiữa")));
    let info = call(&mut e, json!({"cmd": "get_templates"}));
    assert!(info.result["zones"][0]["summary"].as_str().unwrap().contains("tấm chia"));
}

#[test]
fn resize_stretch_proportional_or_edge_bay() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let root = z.result["zones"][0]["id"].as_u64().unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let divider: ObjectId = serde_json::from_value(kids.iter().find(|k| k["name"] == "HôngGiữa_01").unwrap()["id"].clone()).unwrap();
    call(&mut e, json!({"cmd": "move_split_panel", "id": divider, "before": 600}));
    let b = bays(&mut e, cab, root);
    let (a0, a1) = (b[0].0, b[1].0);
    assert!((a0 - 600.0).abs() < 1e-6);

    // Chỉ khoang sát cạnh kéo (right edge): the right bay takes the +200.
    let r = call(&mut e, json!({"cmd": "resize_cabinet", "id": cab, "name": "width", "value": 1800, "stretch": "EDGE", "edge": "END"}));
    assert!(r.ok, "{:?}", r.error);
    let b = bays(&mut e, cab, root);
    assert!((b[0].0 - a0).abs() < 1e-6 && (b[1].0 - (a1 + 200.0)).abs() < 1e-6, "{b:?}");
    // One undo restores size and bay modes.
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(e.doc.param_value(cab, "width"), Some(1600.0));

    // Dãn đều tất cả: both bays keep their ratio.
    call(&mut e, json!({"cmd": "resize_cabinet", "id": cab, "name": "width", "value": 1800, "stretch": "PROPORTIONAL"}));
    let b = bays(&mut e, cab, root);
    let k = (a0 + a1 + 200.0) / (a0 + a1);
    assert!((b[0].0 - a0 * k).abs() < 1e-6 && (b[1].0 - a1 * k).abs() < 1e-6, "{b:?}");

    // Height, edge = top: only the top shelf cell grows; the lower cells keep their size.
    let zi = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let shelf_zone = zi.result["positions"].as_array().unwrap().iter().find(|p| p["zone"].as_u64() != Some(root)).unwrap()["zone"].as_u64().unwrap();
    let s0 = bays(&mut e, cab, shelf_zone);
    let r = call(&mut e, json!({"cmd": "resize_cabinet", "id": cab, "name": "height", "value": 2500, "stretch": "EDGE", "edge": "END"}));
    assert!(r.ok, "{:?}", r.error);
    let s1 = bays(&mut e, cab, shelf_zone);
    let n = s0.len();
    for i in 0..n - 1 {
        assert!((s1[i].0 - s0[i].0).abs() < 1e-6, "cell {i}");
    }
    assert!((s1[n - 1].0 - s0[n - 1].0 - 100.0).abs() < 1e-6);
}

#[test]
fn move_objects_by_vector_one_undo() {
    let mut e = Engine::new();
    let a: ObjectId = serde_json::from_value(call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "position": [0, 0, 0]})).result["id"].clone()).unwrap();
    let b: ObjectId = serde_json::from_value(call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "position": [1000, 0, 0]})).result["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "move_objects", "ids": [a, b], "delta": [250, 0, -100]}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!((e.doc.param_value(a, "x"), e.doc.param_value(b, "x"), e.doc.param_value(b, "z")), (Some(250.0), Some(1250.0), Some(-100.0)));
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!((e.doc.param_value(a, "x"), e.doc.param_value(b, "x")), (Some(0.0), Some(1000.0)));
}

#[test]
fn split_zone_by_formula_top_down_and_virtual() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"doors": 0, "shelves": 0, "width": 1000, "height": 2000}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let root = z.result["zones"][0]["id"].as_u64().unwrap();
    let before = e.doc.cabinet_layout(cab).unwrap().parts.len();
    // Chia ngang 500 từ trên xuống, tạo kệ cố định.
    let r = call(&mut e, json!({"cmd": "split_zone", "cabinet": cab, "zone": root, "kind": "SHELF_FIXED", "formula": "500", "from_end": true}));
    assert!(r.ok, "{:?}", r.error);
    let b = bays(&mut e, cab, root);
    assert_eq!(b.len(), 2);
    assert!((b[1].0 - 500.0).abs() < 1e-6, "top bay locked 500: {b:?}");
    assert_eq!(e.doc.cabinet_layout(cab).unwrap().parts.len(), before + 1);
    // Chia ảo khoang dưới thành 3 cột, không tạo tấm.
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let lower = z.result["bays"].as_array().unwrap().iter().find(|x| x["zone"] == root && x["index"] == 0).unwrap()["child"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "split_zone", "cabinet": cab, "zone": lower, "kind": "VIRTUAL_V", "formula": "/3"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.cabinet_layout(cab).unwrap().parts.len(), before + 1, "virtual split makes no part");
    let b = bays(&mut e, cab, lower);
    assert_eq!(b.len(), 3);
    assert!((b[0].0 - b[2].0).abs() < 1e-6);
    // Không đủ chỗ → từ chối; công thức sai → lỗi formula.
    let c0 = b_child(&mut e, cab, lower, 0);
    let r = call(&mut e, json!({"cmd": "split_zone", "cabinet": cab, "zone": c0, "kind": "SHELF_FIXED", "formula": "5000"}));
    assert!(!r.ok);
    let r = call(&mut e, json!({"cmd": "split_zone", "cabinet": cab, "zone": c0, "kind": "SHELF_FIXED", "formula": "abc"}));
    assert!(!r.ok);
    // Một bước undo cho mỗi lần chia.
    call(&mut e, json!({"cmd": "undo"}));
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(e.doc.cabinet_layout(cab).unwrap().parts.len(), before);
}

fn b_child(e: &mut Engine, cab: ObjectId, zone: u64, index: u64) -> u64 {
    let z = call(e, json!({"cmd": "get_zones", "cabinet": cab}));
    z.result["bays"].as_array().unwrap().iter().find(|x| x["zone"] == zone && x["index"] == index).unwrap()["child"].as_u64().unwrap()
}

#[test]
fn shop_standard_fields_saved_and_applied() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "overrides": {"width": 1000, "height": 2000}}));
    let a: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let pins = |e: &Engine, cab| e.doc.cabinet_layout(cab).unwrap().parts.iter().filter_map(|p| match &p.kind { aic_domain::PartKind::Panel { features, .. } => Some(features), _ => None }).map(|fs| fs.iter().filter(|f| matches!(f, aic_domain::MachiningFeature::Drill(d) if d.purpose == aic_domain::DrillPurpose::ShelfPin)).count()).sum::<usize>();
    let before = pins(&e, a);
    assert!(before > 0);
    // Hàng lỗ 32 → nhiều lỗ hơn hẳn; bản lề 5 cái cho cánh cao.
    for (k, v) in [("s_pin_row", "row_32"), ("s_hinge_table", "900=2,1600=3,1800=4,5"), ("s_handle_pos", "TOP")] {
        let r = call(&mut e, json!({"cmd": "set_parameter", "id": a, "name": k, "value": v}));
        assert!(r.ok, "{k}: {:?}", r.error);
    }
    assert!(pins(&e, a) > before * 3, "32-mm rows");
    let hinges = e.doc.cabinet_layout(a).unwrap().fittings.hinges;
    assert!(hinges.is_multiple_of(5) && hinges > 0, "{hinges}");
    let s = call(&mut e, json!({"cmd": "get_structure", "cabinet": a}));
    let shelves = s.result["tabs"].as_array().unwrap().iter().find(|t| t["key"] == "shelves").unwrap().clone();
    assert!(shelves["fields"].as_array().unwrap().iter().any(|f| f["key"] == "s_pin_row" && f["value"] == "ROW_32"));
    // Lưu Chuẩn xưởng (mọi tab) rồi áp lên tủ khác: một bước undo.
    let r = call(&mut e, json!({"cmd": "save_group_preset", "cabinet": a, "group": "all", "name": "Xưởng A"}));
    assert!(r.ok, "{:?}", r.error);
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "overrides": {"width": 800, "height": 2000}}));
    let b: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let b_before = pins(&e, b);
    let r = call(&mut e, json!({"cmd": "apply_group_preset", "ids": [b], "group": "all", "name": "Xưởng A"}));
    assert!(r.ok, "{:?}", r.error);
    assert!(pins(&e, b) > b_before * 3);
    assert_eq!(e.doc.param_value(b, "width"), Some(800.0), "size is not part of the standard");
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(pins(&e, b), b_before);
    // Giá trị sai bị từ chối.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": a, "name": "s_pin_row", "value": "abc"}));
    assert!(!r.ok);
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": a, "name": "s_pin_d", "value": "-3"}));
    assert!(!r.ok);
}

#[test]
fn shelves_snap_to_32mm_pin_row() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "overrides": {"width": 800, "height": 2000, "doors": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let shelf_pos = |e: &mut Engine| {
        let z = call(e, json!({"cmd": "get_zones", "cabinet": cab}));
        z.result["positions"].as_array().unwrap().iter().filter(|p| p["axis"] == 1).map(|p| p["from_start"].as_f64().unwrap()).collect::<Vec<_>>()
    };
    let before = shelf_pos(&mut e);
    assert!(!before.is_empty());
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "s_pin_row", "value": "ROW_32"}));
    let after = shelf_pos(&mut e);
    // base = lỗ đầu 64 + lỗ dưới mặt kệ 5; mọi kệ nằm trên bội 32.
    for st in &after {
        let k = (st - 69.0) / 32.0;
        assert!((k - k.round()).abs() < 1e-6, "shelf at {st} not on the 32 grid");
    }
    for (a, b) in before.iter().zip(after.iter()) {
        assert!((a - b).abs() <= 16.0 + 1e-6);
    }
    // Kéo kệ tới vị trí lẻ → vẫn bắt lỗ.
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let shelf: ObjectId = serde_json::from_value(kids.iter().find(|k| k["name"].as_str().unwrap().starts_with("KệDiĐộng")).unwrap()["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "move_split_panel", "id": shelf, "before": 413}));
    assert!(r.ok, "{:?}", r.error);
    for st in shelf_pos(&mut e) {
        let k = (st - 69.0) / 32.0;
        assert!((k - k.round()).abs() < 1e-6, "{st}");
    }
}

#[test]
fn handle_types_hinge_plate_and_slide_types() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800, "doors": 2}}));
    let door_cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "DRAWER", "overrides": {"width": 600, "drawers": 2}}));
    let drw: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let names = |e: &Engine, c| e.doc.cabinet_layout(c).unwrap().parts.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    let count = |e: &Engine, c, purpose: aic_domain::DrillPurpose| {
        e.doc.cabinet_layout(c).unwrap().parts.iter().filter_map(|p| match &p.kind { aic_domain::PartKind::Panel { features, .. } => Some(features.clone()), _ => None }).flatten().filter(|f| matches!(f, aic_domain::MachiningFeature::Drill(d) if d.purpose == purpose)).count()
    };
    let set = |e: &mut Engine, id, k: &str, v: &str| {
        let r = call(e, json!({"cmd": "set_parameter", "id": id, "name": k, "value": v}));
        assert!(r.ok, "{k}={v}: {:?}", r.error);
    };
    // Tay nắm bếp dưới (AUTO) nằm nửa trên cánh.
    let l = e.doc.cabinet_layout(door_cab).unwrap();
    let h = l.parts.iter().find(|p| p.name.starts_with("TayNắm")).unwrap().translation[1];
    let d = l.parts.iter().find(|p| p.name.starts_with("CửaĐôi") || p.name.starts_with("CửaĐơn")).unwrap();
    assert!(h > d.translation[1] + d.size[1] / 2.0, "handle on the upper half of a base door");
    // Đế bản lề + khoan lỗ tay nắm.
    assert_eq!(count(&e, door_cab, aic_domain::DrillPurpose::HingeScrew), 0);
    set(&mut e, door_cab, "s_hinge_plate", "on");
    set(&mut e, door_cab, "s_handle_drill", "on");
    let hinges = e.doc.cabinet_layout(door_cab).unwrap().fittings.hinges as usize;
    assert_eq!(count(&e, door_cab, aic_domain::DrillPurpose::HingeScrew), hinges * 2);
    assert_eq!(count(&e, door_cab, aic_domain::DrillPurpose::Handle), 4, "2 doors × 2 holes");
    // Nhấn mở: không còn tay nắm, báo giá có push-open.
    set(&mut e, door_cab, "s_handle_type", "push_open");
    assert!(!names(&e, door_cab).iter().any(|n| n.starts_with("TayNắm")));
    assert_eq!(e.doc.cabinet_layout(door_cab).unwrap().fittings.push_latches, 2);
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(c.result["fittings"].as_array().unwrap().iter().any(|l| l["name"] == "Nhấn mở (push-open)"), "{}", c.result["fittings"]);
    // Ray: tandem bỏ thành gỗ; ray âm dùng mã RAYAM.
    assert!(names(&e, drw).iter().any(|n| n.starts_with("ThànhTrái")));
    set(&mut e, drw, "s_slide_type", "TANDEM");
    let n = names(&e, drw);
    assert!(!n.iter().any(|n| n.starts_with("ThànhTrái")) && n.iter().any(|n| n.starts_with("Tandem")));
    set(&mut e, drw, "s_slide_type", "UNDERMOUNT");
    assert!(names(&e, drw).iter().any(|n| n.starts_with("RayÂm")));
    assert!(!e.doc.cabinet_layout(drw).unwrap().fittings.undermount.is_empty());
}

#[test]
fn joint_types_generate_real_holes_and_costing() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800, "doors": 0, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let fit = |e: &mut Engine, name: &str| {
        let c = call(e, json!({"cmd": "get_costing"}));
        c.result["fittings"].as_array().unwrap().iter().find(|l| l["name"].as_str().unwrap().starts_with(name)).map(|l| l["qty"].as_f64().unwrap()).unwrap_or(0.0)
    };
    assert!(fit(&mut e, "Chốt gỗ") > 0.0);
    assert_eq!(fit(&mut e, "Cam"), 0.0);
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "s_joint_type", "value": "CAM_DOWEL"}));
    assert!(r.ok, "{:?}", r.error);
    let cams = fit(&mut e, "Cam");
    assert!(cams >= 4.0, "2 cams per joint end: {cams}");
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "s_joint_type", "value": "SCREW"}));
    assert_eq!(fit(&mut e, "Cam"), 0.0);
    assert!(fit(&mut e, "Vít liên kết") > 0.0);
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "s_joint_type", "value": "BRACKET"}));
    assert!(fit(&mut e, "Ke góc") > 0.0);
    assert_eq!(fit(&mut e, "Chốt gỗ"), 0.0);
    call(&mut e, json!({"cmd": "undo"}));
    assert!(fit(&mut e, "Vít liên kết") > 0.0);
}

#[test]
fn base_types_plinth3_legs_hanging() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 900}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let names = |e: &Engine| e.doc.cabinet_layout(cab).unwrap().parts.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    let set = |e: &mut Engine, k: &str, v: &str| {
        let r = call(e, json!({"cmd": "set_parameter", "id": cab, "name": k, "value": v}));
        assert!(r.ok, "{k}={v}: {:?}", r.error);
    };
    set(&mut e, "plinth_height", "100");
    assert!(names(&e).contains(&"ChânTủ".to_string()), "AUTO keeps the old front plinth");
    set(&mut e, "bottom_style", "OVERLAY");
    set(&mut e, "base_type", "PLINTH_3");
    let n = names(&e);
    assert!(n.contains(&"ChânHôngTrái".to_string()) && n.contains(&"ChânHôngPhải".to_string()), "{n:?}");
    set(&mut e, "base_type", "LEGS_PLINTH");
    let l = e.doc.cabinet_layout(cab).unwrap();
    assert_eq!(l.fittings.legs, 6, "900 wide → 6 legs");
    assert!(names(&e).contains(&"ChânTủ".to_string()));
    set(&mut e, "base_type", "HANGING");
    set(&mut e, "hang_rail", "on");
    let l = e.doc.cabinet_layout(cab).unwrap();
    assert_eq!((l.fittings.legs, l.fittings.hangers), (0, 2));
    assert!(names(&e).contains(&"ThanhTreoTường".to_string()) && !names(&e).contains(&"ChânTủ".to_string()));
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(c.result["fittings"].as_array().unwrap().iter().any(|l| l["name"] == "Ke treo tủ"));
}

#[test]
fn run_countertop_follows_cabinets_one_undo() {
    let mut e = Engine::new();
    let mut ids = Vec::new();
    for (x, w) in [(0.0, 800.0), (800.0, 600.0), (1400.0, 900.0)] {
        let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "position": [x, 0, 0], "overrides": {"width": w, "plinth_height": 100}}));
        ids.push(serde_json::from_value::<ObjectId>(r.result["id"].clone()).unwrap());
    }
    let r = call(&mut e, json!({"cmd": "create_run", "ids": ids, "rules": {"countertop": true, "continuous_plinth": true, "filler_right": 50}}));
    assert!(r.ok, "{:?}", r.error);
    let part = |e: &Engine, name: &str| e.doc.objects.values().filter_map(|o| o.as_panel()).find(|p| p.name == name).map(|p| (p.width_mm, p.height_mm, p.thickness_mm));
    assert_eq!(part(&e, "MặtĐá"), Some((2350.0, 620.0, 20.0)));
    assert_eq!(part(&e, "LenChânDãy").map(|p| p.0), Some(2350.0));
    assert!(part(&e, "TấmLấpPhải").is_some());
    // Tủ trong dãy bỏ len riêng.
    assert!(!e.doc.cabinet_layout(ids[0]).unwrap().parts.iter().any(|p| p.name == "ChânTủ"));
    // Đổi rộng tủ giữa (đẩy dãy): mặt đá tự dài ra, một bước undo.
    let r = call(&mut e, json!({"cmd": "resize_cabinet", "id": ids[1], "name": "width", "value": 700, "stretch": "KEEP"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(part(&e, "MặtĐá").map(|p| p.0), Some(2450.0));
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(part(&e, "MặtĐá").map(|p| p.0), Some(2350.0));
    assert_eq!(e.doc.param_value(ids[1], "width"), Some(600.0));
    // Sửa luật: bỏ mặt đá; xóa dãy trả lại len riêng.
    let r = call(&mut e, json!({"cmd": "update_run", "name": "Dãy 1", "rules": {"countertop": false, "continuous_plinth": true}}));
    assert!(r.ok, "{:?}", r.error);
    assert!(part(&e, "MặtĐá").is_none());
    call(&mut e, json!({"cmd": "delete_run", "name": "Dãy 1"}));
    assert!(part(&e, "LenChânDãy").is_none());
    assert!(e.doc.cabinet_layout(ids[0]).unwrap().parts.iter().any(|p| p.name == "ChânTủ"));
}

#[test]
fn blind_corner_cabinet_left_and_right() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_corner", "hand": "LEFT", "width": 1100, "door_width": 450}));
    assert!(r.ok, "{:?}", r.error);
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let l = e.doc.cabinet_layout(cab).unwrap();
    let blind = l.parts.iter().find(|p| p.name.starts_with("TấmMù")).expect("blind panel");
    let door = l.parts.iter().find(|p| p.name.starts_with("CửaĐơn")).expect("door");
    assert!(blind.translation[0] < door.translation[0], "blind on the left");
    let gap = door.translation[0] - (blind.translation[0] + blind.size[0]);
    assert!(gap > 0.5 && gap < 5.0, "blind and door leave a door gap: {gap}");
    assert!((door.size[0] - 450.0).abs() < 5.0, "door ≈ 450: {}", door.size[0]);
    assert!(l.fittings.hinges == 2 && l.fittings.handles == 1, "only the door has hinges / handle");
    call(&mut e, json!({"cmd": "undo"}));
    assert!(!e.doc.objects.contains_key(&cab), "one undo removes the corner cabinet");
    let r = call(&mut e, json!({"cmd": "create_corner", "hand": "RIGHT", "width": 1000}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let l = e.doc.cabinet_layout(cab).unwrap();
    let blind = l.parts.iter().find(|p| p.name.starts_with("TấmMù")).unwrap();
    let door = l.parts.iter().find(|p| p.name.starts_with("CửaĐơn")).unwrap();
    assert!(blind.translation[0] > door.translation[0]);
    let r = call(&mut e, json!({"cmd": "create_corner", "hand": "LEFT", "width": 600, "door_width": 500}));
    assert!(!r.ok);
}

#[test]
fn edge_bands_per_panel_group() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800, "doors": 2}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let set = |e: &mut Engine, k: &str, v: &str| {
        let r = call(e, json!({"cmd": "set_parameter", "id": cab, "name": k, "value": v}));
        assert!(r.ok, "{k}={v}: {:?}", r.error);
    };
    set(&mut e, "back_thickness", "9");
    set(&mut e, "edge_g_front_mode", "ALL");
    set(&mut e, "edge_g_front_code", "ABS-2");
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    let rows = c.result["cut_list"].as_array().unwrap().clone();
    let door = rows.iter().find(|r| r["name"].as_str().unwrap().starts_with("CửaĐôi")).unwrap();
    assert!((door["length"].as_f64().unwrap() - door["cut_length"].as_f64().unwrap() - 4.0).abs() < 1e-6, "ABS 2mm on both ends");
    assert_eq!(door["edges"].as_array().unwrap().len(), 4);
    let back = rows.iter().find(|r| r["role"] == "Back").unwrap();
    assert!(back["edges"].as_array().unwrap().is_empty(), "9 mm back is not banded (by role)");
    assert!(c.result["edges"].as_array().unwrap().iter().any(|l| l["key"].as_str().unwrap().contains("ABS-2")));
    // Về luật chung.
    set(&mut e, "edge_g_front_mode", "INHERIT");
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(!c.result["edges"].as_array().unwrap().iter().any(|l| l["key"].as_str().unwrap().contains("ABS-2")));
}

#[test]
fn quote_linear_facade_and_cut_groups() {
    let mut e = Engine::new();
    for w in [800.0, 600.0, 900.0] {
        call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "room": "Bếp", "overrides": {"width": w}}));
    }
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "room": "PN1", "overrides": {"width": 1800, "height": 2400}}));
    let wr: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    let q = &c.result["quote"];
    let bep: f64 = q["rows"].as_array().unwrap().iter().filter(|r| r["room"] == "Bếp").map(|r| r["amount"].as_f64().unwrap()).sum();
    assert!((bep - 2.3 * 4_500_000.0).abs() < 1.0, "{bep}");
    let tu = q["rows"].as_array().unwrap().iter().find(|r| r["room"] == "PN1").unwrap();
    assert_eq!(tu["mode"], "FACADE_M2");
    assert!((tu["amount"].as_f64().unwrap() - 4.32 * 3_200_000.0).abs() < 1.0);
    assert!((q["total"].as_f64().unwrap() - q["subtotal"].as_f64().unwrap() * 1.08).abs() < 1.0, "VAT 8%");
    // Đổi đơn giá mét dài + chuyển tủ áo sang bóc chi tiết.
    call(&mut e, json!({"cmd": "set_price", "key": "quote:linear:BASE", "value": 5_000_000}));
    call(&mut e, json!({"cmd": "set_parameter", "id": wr, "name": "pricing", "value": "DETAIL"}));
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    let q = &c.result["quote"];
    let bep: f64 = q["rows"].as_array().unwrap().iter().filter(|r| r["room"] == "Bếp").map(|r| r["amount"].as_f64().unwrap()).sum();
    assert!((bep - 2.3 * 5_000_000.0).abs() < 1.0);
    let tu = q["rows"].as_array().unwrap().iter().find(|r| r["room"] == "PN1").unwrap();
    assert_eq!(tu["mode"], "DETAIL");
    // Danh sách cắt gộp: 3 tủ bếp cùng cao/sâu → hồi gộp (6 tấm).
    let groups = c.result["cut_groups"].as_array().unwrap();
    let sides = groups.iter().filter(|g| g["name"].as_str().unwrap().starts_with("Hồi")).map(|g| g["qty"].as_u64().unwrap()).max().unwrap();
    assert!(sides >= 6, "{sides}");
    assert!(c.result["cut_list"][0]["code"].as_str().unwrap().contains("Bếp") || c.result["cut_list"][0]["code"].as_str().unwrap().contains("PN1"));
}
