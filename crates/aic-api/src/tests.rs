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
    // Khoét chậu trên mặt đá: một đường bao trong, theo mặt đá khi dãy đổi.
    let r = call(&mut e, json!({"cmd": "update_run", "name": "Dãy 1", "rules": {"countertop": true, "continuous_plinth": true, "filler_right": 50, "cutouts": [{"kind": "SINK", "x": 900, "width": 780, "depth": 430, "from_front": 80, "radius": 10}]}}));
    assert!(r.ok, "{:?}", r.error);
    let top = e.doc.objects.values().filter_map(|o| o.as_panel()).find(|p| p.name == "MặtĐá").unwrap();
    assert_eq!(top.features.iter().filter(|f| matches!(f, aic_domain::MachiningFeature::Contour(c) if c.inner)).count(), 1);
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

#[test]
fn inner_drawers_behind_doors_and_false_front() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "overrides": {"width": 1000, "height": 2000, "shelves": 0, "doors": 2}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let zs = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let root = zs.result["zones"].as_array().unwrap().iter().find(|z| z["leaf"] == true).unwrap()["id"].as_u64().unwrap();
    // Chia ảo phần dưới 400 rồi thêm 2 ngăn kéo trong vào khoang dưới (cánh vẫn phủ cả tủ).
    let r = call(&mut e, json!({"cmd": "split_zone", "cabinet": cab, "zone": root, "kind": "VIRTUAL_H", "formula": "400"}));
    assert!(r.ok, "{:?}", r.error);
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let low = z.result["bays"].as_array().unwrap().iter().find(|b| b["zone"] == root && b["index"] == 0).unwrap()["child"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "zone_add_drawers", "cabinet": cab, "zones": [low], "count": 2, "inner": true}));
    assert!(r.ok, "{:?}", r.error);
    let l = e.doc.cabinet_layout(cab).unwrap();
    let doors: Vec<_> = l.parts.iter().filter(|p| p.name.starts_with("CửaĐôi")).collect();
    let inner: Vec<_> = l.parts.iter().filter(|p| p.name.starts_with("MặtNgănTrong")).collect();
    assert_eq!(doors.len(), 2, "doors still cover the cabinet");
    assert_eq!(inner.len(), 2);
    let door_back = doors[0].translation[2];
    assert!(inner.iter().all(|p| p.translation[2] + p.size[2] < door_back - 20.0), "inner fronts sit behind the doors");
    assert!(!l.parts.iter().any(|p| p.name.starts_with("TayNắm [Bộ")), "inner drawers have no handle");
    assert!(l.parts.iter().any(|p| p.name.starts_with("ThànhTrái")), "inner drawers keep their box");
    // Mặt giả tủ chậu: chỉ mặt, không hộc / ray / tay nắm.
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800, "doors": 0, "shelves": 0}}));
    let sink: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let sroot = call(&mut e, json!({"cmd": "get_zones", "cabinet": sink})).result["zones"][0]["id"].as_u64().unwrap();
    call(&mut e, json!({"cmd": "zone_add_drawers", "cabinet": sink, "zones": [sroot], "count": 1, "false_front": true}));
    let l = e.doc.cabinet_layout(sink).unwrap();
    assert!(l.parts.iter().any(|p| p.name.starts_with("MặtGiả")));
    assert!(!l.parts.iter().any(|p| p.name.starts_with("ThànhTrái") || p.name.starts_with("RayBi") || p.name.starts_with("TayNắm")));
    assert!(l.fittings.slides.is_empty());
}

#[test]
fn material_sets_apply_to_a_room_one_undo() {
    let mut e = Engine::new();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "room": "Bếp", "overrides": {"width": 800, "doors": 2}}));
        ids.push(serde_json::from_value::<ObjectId>(r.result["id"].clone()).unwrap());
    }
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "room": "PN1"}));
    let wr: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let sets = call(&mut e, json!({"cmd": "get_material_sets"}));
    let name = sets.result["sets"][0]["set"]["name"].as_str().unwrap().to_string();
    assert!(name.contains("chống ẩm"));
    let r = call(&mut e, json!({"cmd": "apply_material_set", "room": "Bếp", "name": name}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(r.result["cabinets"], 2);
    let mat = |e: &Engine, cab: ObjectId, role: aic_domain::PanelRole| {
        e.doc.scene.subtree(cab).into_iter().filter_map(|c| e.doc.panel(c)).find(|p| p.role == role).map(|p| p.material_id.0.clone()).unwrap()
    };
    use aic_domain::PanelRole;
    assert_eq!(mat(&e, ids[0], PanelRole::LeftSide), "MFCMR18-WHITE");
    assert_eq!(mat(&e, ids[1], PanelRole::Door), "ACR18-WHITE");
    assert_eq!(mat(&e, ids[0], PanelRole::Back), "HDFMR8-WHITE");
    assert_ne!(mat(&e, wr, PanelRole::LeftSide), "MFCMR18-WHITE", "other room untouched");
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(c.result["edges"].as_array().unwrap().iter().any(|l| l["key"].as_str().unwrap().contains("ABS-1")));
    // Một undo trả lại cả phòng.
    call(&mut e, json!({"cmd": "undo"}));
    assert_ne!(mat(&e, ids[0], PanelRole::LeftSide), "MFCMR18-WHITE");
    assert_ne!(mat(&e, ids[1], PanelRole::Door), "ACR18-WHITE");
    // Lưu bộ từ tủ, áp lại.
    let r = call(&mut e, json!({"cmd": "save_material_set", "cabinet": wr, "name": "Bộ tủ áo"}));
    assert!(r.ok, "{:?}", r.error);
    let r = call(&mut e, json!({"cmd": "apply_material_set", "ids": [ids[0]], "name": "Bộ tủ áo"}));
    assert!(r.ok, "{:?}", r.error);
    let r = call(&mut e, json!({"cmd": "apply_material_set", "ids": [ids[0]], "name": "không có"}));
    assert!(!r.ok);
}

#[test]
fn diagonal_corner_cabinet_pentagon_panels_and_45_degree_door() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_corner", "hand": "LEFT", "kind": "DIAGONAL", "width": 900, "depth": 580}));
    assert!(r.ok, "{:?}", r.error);
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let l = e.doc.cabinet_layout(cab).unwrap();
    let part = |name: &str| l.parts.iter().find(|p| p.name.starts_with(name)).unwrap_or_else(|| panic!("{name}"));
    // Đáy / nóc / kệ 5 cạnh.
    for n in ["Đáy", "Nóc", "KệCốĐịnh"] {
        match &part(n).kind {
            aic_domain::PartKind::Panel { features, .. } => assert!(features.iter().any(|f| matches!(f, aic_domain::MachiningFeature::Contour(c) if !c.inner && c.polygon.points.len() == 5)), "{n}"),
            _ => panic!(),
        }
    }
    // Cánh chéo 45°, rộng = √2 × (900 − 580) − 2 khe.
    let door = part("CửaChéo");
    assert_eq!(door.rotation_deg, [0.0, 45.0, 0.0]);
    let gap = e.doc.param_value(cab, "door_gap").unwrap();
    assert!((door.size[0] - (2f64.sqrt() * 320.0 - 2.0 * gap)).abs() < 1e-6);
    assert!(l.fittings.hinges >= 2);
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("Hậu")).count(), 2, "two backs against the walls");
    // Bảng kết cấu có tab Tủ góc chéo; đổi số kệ.
    let s = call(&mut e, json!({"cmd": "get_structure", "cabinet": cab}));
    assert_eq!(s.result["tabs"][0]["key"], "diagonal");
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "dg_shelves", "value": "2"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.cabinet_layout(cab).unwrap().parts.iter().filter(|p| p.name.starts_with("KệCốĐịnh")).count(), 2);
    // Danh sách cắt có đường bao.
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(c.result["cut_list"].as_array().unwrap().iter().any(|r| r["name"] == "Đáy" && r["machining"].as_array().unwrap().iter().any(|m| m.as_str().unwrap().contains("Đường bao"))));
    // Sâu tay quá lớn → từ chối; một undo xóa tủ.
    let r = call(&mut e, json!({"cmd": "create_corner", "hand": "LEFT", "kind": "DIAGONAL", "width": 700, "depth": 600}));
    assert!(!r.ok);
    call(&mut e, json!({"cmd": "undo"}));
    call(&mut e, json!({"cmd": "undo"}));
    assert!(!e.doc.objects.contains_key(&cab));
}

#[test]
fn tool_features_are_parametric_and_one_undo() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800, "depth": 560}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let id = |n: &str| serde_json::from_value::<ObjectId>(kids.iter().find(|k| k["name"] == n).unwrap()["id"].clone()).unwrap();
    let (l, rgt) = (id("HồiTrái"), id("HồiPhải"));
    let r = call(&mut e, json!({"cmd": "tool_feature", "ids": [l, rgt], "tool": "03. Khấu góc tủ", "feature": {"type": "NOTCH", "corner": "TR", "width": 100, "depth": 100}}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(r.result["panels"], 2);
    let notch = |e: &Engine| {
        let layout = e.doc.cabinet_layout(cab).unwrap();
        let p = layout.parts.iter().find(|p| p.name == "HồiTrái").unwrap();
        let f = match &p.kind {
            aic_domain::PartKind::Panel { features, .. } => features.iter().find_map(|f| match f {
                aic_domain::MachiningFeature::Pocket(pk) if pk.width == 100.0 => Some(pk.clone()),
                _ => None,
            }),
            _ => None,
        };
        (p.size[0], f)
    };
    let (w0, f0) = notch(&e);
    assert!((f0.unwrap().x - (w0 - 100.0)).abs() < 1e-6);
    // Đổi sâu tủ → hồi rộng hơn, khấu vẫn ở góc trên phải.
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "depth", "value": "600"}));
    let (w1, f1) = notch(&e);
    assert!((w1 - w0 - 40.0).abs() < 1e-6);
    assert!((f1.unwrap().x - (w1 - 100.0)).abs() < 1e-6, "notch follows the corner");
    // 2 tấm = 1 undo (sau khi undo đổi sâu).
    call(&mut e, json!({"cmd": "undo"}));
    call(&mut e, json!({"cmd": "undo"}));
    assert!(notch(&e).1.is_none());
    // Rãnh LED cách mép cuối 50 theo chiều cao.
    let r = call(&mut e, json!({"cmd": "tool_feature", "ids": [l], "tool": "16. LED", "feature": {"type": "GROOVE_LINE", "direction": "Y", "offset": 50, "from_end": true, "width": 10, "depth": 8, "side": "A"}}));
    assert!(r.ok, "{:?}", r.error);
}

#[test]
fn kitchen_products_insert_with_one_undo() {
    let mut e = Engine::new();
    let info = call(&mut e, json!({"cmd": "get_products"}));
    assert_eq!(info.result["products"].as_array().unwrap().iter().filter(|p| p["room"] == "Bếp").count(), 5);
    let mut ids = Vec::new();
    for key in ["KITCHEN_BASE_800", "KITCHEN_WALL_800", "KITCHEN_CORNER_L", "KITCHEN_OVEN_TALL"] {
        let r = call(&mut e, json!({"cmd": "insert_product", "key": key}));
        assert!(r.ok, "{key}: {:?}", r.error);
        ids.push(serde_json::from_value::<ObjectId>(r.result["id"].clone()).unwrap());
    }
    let names = |e: &Engine, c: ObjectId| e.doc.cabinet_layout(c).unwrap().parts.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    let size = |e: &Engine, c: ObjectId| ["width", "height", "depth"].map(|n| e.doc.param_value(c, n).unwrap());
    // Bếp dưới: 800 × 810 × 560, kệ, cánh đôi, chân nhựa 6, vật liệu chống ẩm.
    assert_eq!(size(&e, ids[0]), [800.0, 810.0, 560.0]);
    let l = e.doc.cabinet_layout(ids[0]).unwrap();
    assert_eq!(l.fittings.legs, 6);
    assert_eq!(names(&e, ids[0]).iter().filter(|n| n.starts_with("CửaĐôi")).count(), 2);
    assert!(names(&e, ids[0]).iter().any(|n| n.starts_with("KệDiĐộng")));
    let cab0 = e.doc.objects.get(&ids[0]).unwrap().as_cabinet().unwrap().clone();
    assert_eq!(cab0.carcass_material.0, "MFCMR18-WHITE");
    assert_eq!(cab0.room, "Bếp");
    // Bếp trên: treo, tay nắm nửa dưới cánh.
    let l = e.doc.cabinet_layout(ids[1]).unwrap();
    assert_eq!(l.fittings.hangers, 2);
    let door = l.parts.iter().find(|p| p.name.starts_with("CửaĐôi")).unwrap();
    let handle = l.parts.iter().find(|p| p.name.starts_with("TayNắm")).unwrap();
    assert!(handle.translation[1] < door.translation[1] + door.size[1] / 2.0);
    // Góc L: tấm mù + cánh + kệ.
    assert!(names(&e, ids[2]).iter().any(|n| n.starts_with("TấmMù")));
    assert!(names(&e, ids[2]).iter().any(|n| n.starts_with("KệDiĐộng")));
    // Tủ lò: 2 ngăn kéo dưới, khoang lò 600, cánh lật trên.
    let n3 = names(&e, ids[3]);
    assert_eq!(n3.iter().filter(|n| n.starts_with("MặtNgăn")).count(), 2);
    assert!(n3.iter().any(|n| n.starts_with("CửaĐơn")));
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": ids[3]}));
    assert!(z.result["bays"].as_array().unwrap().iter().any(|b| (b["size"].as_f64().unwrap() - 600.0).abs() < 1e-6), "oven bay 600");
    // Một undo gỡ tủ lò vừa chèn.
    call(&mut e, json!({"cmd": "undo"}));
    assert!(!e.doc.objects.contains_key(&ids[3]));
    let r = call(&mut e, json!({"cmd": "insert_product", "key": "KHONG_CO"}));
    assert!(!r.ok);
}

#[test]
fn zones_changed_events_only_for_touched_cabinets() {
    use crate::protocol::CoreEvent;
    let mut e = Engine::new();
    let a = created_cabinet(&mut e);
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE"}));
    let b: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let zones_of = |r: &Response| r.events.iter().find_map(|ev| match ev {
        CoreEvent::ZonesChanged { cabinets } => Some(cabinets.clone()),
        _ => None,
    });
    // Kéo vách tủ a → chỉ tủ a có ZonesChanged.
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let divider: ObjectId = serde_json::from_value(kids.iter().find(|k| k["name"] == "HôngGiữa_01").unwrap()["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "move_split_panel", "id": divider, "before": 600}));
    assert_eq!(zones_of(&r), Some(vec![a]));
    // Đổi vật liệu tủ b → khoang không đổi, không có ZonesChanged.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": b, "name": "front_material", "value": "MDF17-OAK"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(zones_of(&r), None);
    // Dời tủ b → không đổi khoang (UI dùng TransformChanged cho ma trận).
    let r = call(&mut e, json!({"cmd": "move_objects", "ids": [b], "delta": [100, 0, 0]}));
    assert_eq!(zones_of(&r), None);
    // Undo kéo vách → ZonesChanged cho a (đối xứng).
    call(&mut e, json!({"cmd": "undo"}));
    call(&mut e, json!({"cmd": "undo"}));
    let r = call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(zones_of(&r), Some(vec![a]));
}

#[test]
fn preview_is_a_dry_run_without_history_or_revision() {
    let mut e = Engine::new();
    let cab = created_cabinet(&mut e);
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"][0]["id"].as_u64().unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let divider: ObjectId = serde_json::from_value(kids.iter().find(|k| k["name"] == "HôngGiữa_01").unwrap()["id"].clone()).unwrap();
    // Tạo một redo để chắc chạy thử không xóa redo.
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "1700"}));
    call(&mut e, json!({"cmd": "undo"}));
    let before = bays(&mut e, cab, root);
    let r0 = call(&mut e, json!({"cmd": "get_status"}));
    let r = call(&mut e, json!({"cmd": "preview", "cabinet": cab, "request": {"cmd": "move_split_panel", "id": divider, "before": 500}}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(r.result["ok"], true);
    assert!(r.events.is_empty(), "preview emits no events");
    let pb: Vec<f64> = r.result["bays"].as_array().unwrap().iter().filter(|b| b["zone"].as_u64() == Some(root)).map(|b| b["size"].as_f64().unwrap()).collect();
    assert!((pb[0] - 500.0).abs() < 1e-6, "{pb:?}");
    // Không đổi gì thật.
    assert_eq!(bays(&mut e, cab, root), before);
    assert_eq!(r.revision, r0.revision);
    assert!(r.can_redo, "redo kept");
    // Chạy thử lỗi → trả ok=false, vẫn không đổi.
    let r = call(&mut e, json!({"cmd": "preview", "cabinet": cab, "request": {"cmd": "move_split_panel", "id": divider, "before": 99999}}));
    assert_eq!(r.result["ok"], false);
    assert_eq!(bays(&mut e, cab, root), before);
    // Hàng lỗ 32: preview trả vị trí đã bắt lỗ (UI không tự đoán).
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "s_pin_row", "value": "ROW_32"}));
    let r = call(&mut e, json!({"cmd": "preview", "cabinet": cab, "request": {"cmd": "create_project", "name": "x"}}));
    assert!(!r.ok, "only edit requests can be previewed");
}

#[test]
fn edit_dims_are_computed_by_core_and_editable() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800, "plinth_height": 100, "shelves": 1, "doors": 2}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    let dims = z.result["dims"].as_array().unwrap().clone();
    let find = |view: &str, name: &str| dims.iter().find(|d| d["view"] == view && d["name"] == name).cloned();
    assert_eq!(find("side", "depth").unwrap()["value"], 600.0);
    assert!(find("side", "shelf_setback").is_some());
    assert_eq!(find("front", "plinth_height").unwrap()["value"], 100.0);
    assert!(find("front", "door_gap").is_some());
    let h = find("front", "s_handle_from_end").expect("handle dim on a base cabinet (top)");
    assert_eq!(h["value"], 60.0);
    // Sửa qua đúng tham số dim trả về (như UI): sâu 580, một undo.
    let dd = find("side", "depth").unwrap();
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": dd["id"], "name": dd["name"], "value": "580"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.param_value(cab, "depth"), Some(580.0));
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": h["name"], "value": "80"}));
    assert!(r.ok, "{:?}", r.error);
    let z = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab}));
    assert!(z.result["dims"].as_array().unwrap().iter().any(|d| d["name"] == "s_handle_from_end" && d["value"] == 80.0));
    call(&mut e, json!({"cmd": "undo"}));
    call(&mut e, json!({"cmd": "undo"}));
    assert_eq!(e.doc.param_value(cab, "depth"), Some(600.0));
}

#[test]
fn align_distribute_rotate_and_snap_to_wall() {
    let mut e = Engine::new();
    call(&mut e, json!({"cmd": "create_room", "width": 4000, "depth": 3000, "height": 2700}));
    let mut ids = Vec::new();
    for (x, y, z, h) in [(0.0, 0.0, 300.0, 700.0), (1000.0, 150.0, 100.0, 850.0), (2600.0, 400.0, 250.0, 600.0)] {
        let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WALL", "position": [x, y, z], "overrides": {"width": 600, "height": h}}));
        ids.push(serde_json::from_value::<ObjectId>(r.result["id"].clone()).unwrap());
    }
    let bb = |e: &Engine, i: ObjectId| e.doc.world_aabb(i);
    // Căn trên: cùng đỉnh, một undo.
    let r = call(&mut e, json!({"cmd": "align_objects", "ids": ids, "mode": "TOP"}));
    assert!(r.ok, "{:?}", r.error);
    let top = bb(&e, ids[0]).max[1];
    assert!(ids.iter().all(|i| (bb(&e, *i).max[1] - top).abs() < 1e-6));
    call(&mut e, json!({"cmd": "undo"}));
    assert!((bb(&e, ids[0]).max[1] - top).abs() > 1.0, "one undo restores all");
    // Phân bố đều theo X: khe bằng nhau.
    call(&mut e, json!({"cmd": "distribute_objects", "ids": ids, "axis": "X"}));
    let g1 = bb(&e, ids[1]).min[0] - bb(&e, ids[0]).max[0];
    let g2 = bb(&e, ids[2]).min[0] - bb(&e, ids[1]).max[0];
    assert!((g1 - g2).abs() < 1e-6, "{g1} {g2}");
    // Xoay 90°: rộng / sâu đổi chỗ trên hộp bao.
    let b0 = bb(&e, ids[0]);
    call(&mut e, json!({"cmd": "rotate_objects", "ids": [ids[0]], "deg": 90, "pivot": "CENTER"}));
    let b1 = bb(&e, ids[0]);
    assert!(((b1.max[0] - b1.min[0]) - (b0.max[2] - b0.min[2])).abs() < 1e-3);
    // Đặt sát tường: tủ 1 (z = 100) về tường sau với khe 5.
    let r = call(&mut e, json!({"cmd": "snap_to_wall", "ids": [ids[1]], "gap": 5}));
    assert!(r.ok, "{:?}", r.error);
    assert!((bb(&e, ids[1]).min[2] - 5.0).abs() < 1e-6, "{:?}", bb(&e, ids[1]));
    let r = call(&mut e, json!({"cmd": "align_objects", "ids": [ids[0]], "mode": "TOP"}));
    assert!(!r.ok, "needs two objects");
}

#[test]
fn tilted_shoe_shelves_and_partial_divider() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 1200, "height": 1000, "depth": 350, "doors": 0, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"][0]["id"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "zone_add_panels", "cabinet": cab, "zones": [root], "kind": "SHELF_ADJUSTABLE", "count": 3, "tilt_deg": [15, 0]}));
    assert!(r.ok, "{:?}", r.error);
    let l = e.doc.cabinet_layout(cab).unwrap();
    let shelves: Vec<_> = l.parts.iter().filter(|p| p.name.starts_with("KệDiĐộng")).collect();
    assert_eq!(shelves.len(), 3);
    let s = shelves[0];
    assert_eq!(s.rotation_deg, [-75.0, 0.0, 0.0], "tilted 15°");
    let sb = e.doc.param_value(cab, "shelf_setback").unwrap();
    let depth = 350.0 - 8.6 - sb; // approx: horizontal projection inside the zone
    assert!(s.size[1] > depth * 0.9 && s.size[1] * 15f64.to_radians().cos() < 350.0, "slope length {}", s.size[1]);
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("ThanhChặnGót")).count(), 3, "one heel stop per shelf");
    assert_eq!(l.fittings.shelf_pins, 0, "tilted shelves sit on rails, no pins");
    // Kệ nghiêng sửa được qua thuộc tính (tilt_fb), 0 → kệ phẳng, không thanh chặn.
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let kids = tree.result["roots"][0]["children"].as_array().unwrap().clone();
    let sid: ObjectId = serde_json::from_value(kids.iter().find(|k| k["name"] == "KệDiĐộng_01").unwrap()["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": sid, "name": "tilt_fb", "value": "0"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.cabinet_layout(cab).unwrap().parts.iter().filter(|p| p.name.starts_with("ThanhChặnGót")).count(), 2);
    // Vách lửng: cao 400 từ đáy.
    let w = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "overrides": {"height": 2000}}));
    let wc: ObjectId = serde_json::from_value(w.result["id"].clone()).unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let all: Vec<Value> = tree.result["roots"].as_array().unwrap().iter().flat_map(|r| r["children"].as_array().unwrap().clone()).collect();
    let did: ObjectId = serde_json::from_value(all.iter().find(|k| k["name"] == "HôngGiữa_01" && k["id"].as_u64() > Some(wc.0)).unwrap()["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": did, "name": "extent", "value": "400"}));
    assert!(r.ok, "{:?}", r.error);
    let d = e.doc.cabinet_layout(wc).unwrap().parts.into_iter().find(|p| p.name == "HôngGiữa_01").unwrap();
    assert!((d.size[1] - 400.0).abs() < 1e-6, "{:?}", d.size);
}

#[test]
fn back_cutouts_overlay_back_and_split_at_fixed_shelves() {
    use aic_domain::layout::PartKind;
    use aic_domain::MachiningFeature;
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800, "height": 720, "depth": 560, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    // Tủ lavabo: ống Ø60 giữa tủ, tâm cách đáy 250.
    let r = call(&mut e, json!({"cmd": "set_back_cutouts", "cabinet": cab, "cutouts": [{"kind": "PIPE", "anchor": "CENTER", "x": 0, "y": 250, "w": 60}]}));
    assert!(r.ok, "{:?}", r.error);
    let hole_center = |e: &Engine| {
        let l = e.doc.cabinet_layout(cab).unwrap();
        let back = l.parts.iter().find(|p| p.key == "c:back").unwrap();
        let PartKind::Panel { features, .. } = &back.kind else { panic!() };
        let c = features.iter().find_map(|f| match f { MachiningFeature::Contour(c) if c.inner => Some(c.clone()), _ => None }).expect("inner contour");
        let n = c.polygon.points.len() as f64;
        let (x, y) = c.polygon.points.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x / n, a.1 + p.y / n));
        (back.translation[0] + x, back.translation[1] + y)
    };
    let p = e.doc.param_value(cab, "plinth_height").unwrap();
    let t = e.doc.param_value(cab, "thickness").unwrap();
    let (x, y) = hole_center(&e);
    assert!((x - 400.0).abs() < 0.5 && (y - (p + t + 250.0)).abs() < 0.5, "{x} {y}");
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "900"}));
    assert!(r.ok, "{:?}", r.error);
    let (x, _) = hole_center(&e);
    assert!((x - 450.0).abs() < 0.5, "hole stays centred after resize: {x}");
    assert!(call(&mut e, json!({"cmd": "get_structure", "cabinet": cab})).result["back_cutouts"].as_array().unwrap().len() == 1);

    // Hậu ốp bắt vít: hậu phủ hết lưng, hồi ngắn lại một độ dày hậu, có vít.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "back_overlay", "value": "1"}));
    assert!(r.ok, "{:?}", r.error);
    let l = e.doc.cabinet_layout(cab).unwrap();
    let bt = e.doc.param_value(cab, "back_thickness").unwrap();
    let back = l.parts.iter().find(|p| p.key == "c:back").unwrap();
    let side = l.parts.iter().find(|p| p.key == "c:left").unwrap();
    assert!((back.size[0] - 900.0).abs() < 1e-6 && back.translation[2] == 0.0, "{:?}", back);
    assert!((side.size[0] - (560.0 - bt)).abs() < 1e-6);
    assert!(l.fittings.back_screws > 10);
    call(&mut e, json!({"cmd": "undo"}));

    // Hậu chia theo kệ cố định (hậu lọt): 2 tấm hậu, kệ chạy suốt ra mép sau.
    for (k, v) in [("back_groove", "0"), ("back_split_at_fixed", "1")] {
        let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": k, "value": v}));
        assert!(r.ok, "{k}: {:?}", r.error);
    }
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"][0]["id"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "zone_add_panels", "cabinet": cab, "zones": [root], "kind": "SHELF_FIXED", "count": 1}));
    assert!(r.ok, "{:?}", r.error);
    let l = e.doc.cabinet_layout(cab).unwrap();
    let backs: Vec<_> = l.parts.iter().filter(|p| p.key.starts_with("c:back")).collect();
    assert_eq!(backs.len(), 2, "one back per section");
    let shelf = l.parts.iter().find(|p| p.name.starts_with("KệCốĐịnh")).unwrap();
    assert!((shelf.translation[2] - shelf.size[1]).abs() < 1e-6, "fixed shelf reaches the rear edge");
    assert!((backs[0].translation[1] + backs[0].size[1] - shelf.translation[1]).abs() < 1e-6);
    // The pipe is in the lower section.
    let PartKind::Panel { features, .. } = &backs[0].kind else { panic!() };
    assert!(features.iter().any(|f| matches!(f, MachiningFeature::Contour(c) if c.inner)));
}

#[test]
fn cornice_end_panels_and_scribe_follow_the_cabinet() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "overrides": {"width": 1600, "height": 2200, "depth": 580}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    for (k, v) in [("tr_cornice", "3_SIDES"), ("tr_cornice_h", "80")] {
        let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": k, "value": v}));
        assert!(r.ok, "{k}: {:?}", r.error);
    }
    let part = |e: &Engine, key: &str| e.doc.cabinet_layout(cab).unwrap().parts.into_iter().find(|p| p.key == key);
    let f = part(&e, "c:cornice_f").unwrap();
    assert!((f.size[0] - 1640.0).abs() < 1e-6 && (f.size[1] - 80.0).abs() < 1e-6 && f.translation[1] == 2200.0, "{:?}", f);
    assert!(f.name.contains("vát 45°"));
    let side = part(&e, "c:cornice_l").unwrap().size[0];
    assert!((side - 600.0).abs() < 1e-6);
    call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "1800"}));
    assert!((part(&e, "c:cornice_f").unwrap().size[0] - 1840.0).abs() < 1e-6, "front cornice follows the width");
    assert!((part(&e, "c:cornice_l").unwrap().size[0] - side).abs() < 1e-6, "side cornice unchanged");
    // Ốp hông trái chạm sàn + nẹp 40: phào trước dài thêm cả hai.
    for (k, v) in [("tr_end_left", "on"), ("tr_scribe_left", "40")] {
        assert!(call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": k, "value": v})).ok);
    }
    let t = e.doc.param_value(cab, "thickness").unwrap();
    let end = part(&e, "c:end_l").unwrap();
    assert!(end.translation[0] == -t && end.translation[1] == 0.0 && (end.size[1] - 2200.0).abs() < 1e-6);
    let sc = part(&e, "c:scribe_l").unwrap();
    assert!((sc.translation[0] + t + 40.0).abs() < 1e-6);
    assert!((part(&e, "c:cornice_f").unwrap().size[0] - (1840.0 + t + 40.0)).abs() < 1e-6);
    let tabs = call(&mut e, json!({"cmd": "get_structure", "cabinet": cab})).result["tabs"].clone();
    assert!(tabs.as_array().unwrap().iter().any(|t| t["key"] == "trim"));
    // Một undo bỏ nẹp.
    call(&mut e, json!({"cmd": "undo"}));
    assert!(part(&e, "c:scribe_l").is_none());
}

#[test]
fn bed_generator_frame_slats_beam_and_side_drawers() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_furniture", "kind": "BED", "width": 1600, "depth": 2000, "height": 1000}));
    assert!(r.ok, "{:?}", r.error);
    let bed: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let t = e.doc.param_value(bed, "thickness").unwrap();
    let l = e.doc.cabinet_layout(bed).unwrap();
    let part = |k: &str| l.parts.iter().find(|p| p.key == k).unwrap_or_else(|| panic!("{k}"));
    // Lọt nệm 1600 × 2000: đầu / đuôi rộng 1600, vai dài 2000 + 2t.
    assert!((part("b:head").size[0] - 1600.0).abs() < 1e-6);
    assert!((part("b:rail_l").size[0] - (2000.0 + 2.0 * t)).abs() < 1e-6);
    let rail_gap = part("b:rail_r").translation[0] - (part("b:rail_l").translation[0] + t);
    assert!((rail_gap - 1600.0).abs() < 1e-6, "mattress fits between the rails");
    assert!(l.parts.iter().any(|p| p.key == "b:beam"), "center beam for W ≥ 1400");
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("NanDát")).count(), 14);
    assert_eq!(l.fittings.legs, 6);
    let slat = part("b:slat0");
    assert!((slat.translation[1] + slat.size[2] - 400.0).abs() < 1e-6, "slat top at frame_h");
    // Hộc kéo 2 bên × 2 → 4 hộc, ray theo sâu hộc, chân chỉ ở 4 góc.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": bed, "name": "bed_storage", "value": "DRAWERS_2_SIDES"}));
    assert!(r.ok, "{:?}", r.error);
    let l = e.doc.cabinet_layout(bed).unwrap();
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("MặtHộcGiường")).count(), 4);
    assert_eq!(l.fittings.slides.values().sum::<u32>(), 4);
    assert!(l.fittings.slides.keys().all(|k| *k as f64 <= 800.0 - 40.0));
    assert_eq!(l.fittings.legs, 4);
    // Đổi rộng → mọi chi tiết giải lại; bảng thuộc tính có tab Giường; báo giá theo chiếc.
    call(&mut e, json!({"cmd": "set_parameter", "id": bed, "name": "width", "value": "1800"}));
    let l = e.doc.cabinet_layout(bed).unwrap();
    assert!((l.parts.iter().find(|p| p.key == "b:head").unwrap().size[0] - 1800.0).abs() < 1e-6);
    let s = call(&mut e, json!({"cmd": "get_structure", "cabinet": bed}));
    assert_eq!(s.result["tabs"][0]["key"], "bed");
    let c = call(&mut e, json!({"cmd": "get_costing"}));
    assert!(c.result.to_string().contains("\"unit\":\"chiếc\""), "{}", c.result["quote"]);
    // Rộng nệm ngoài [800, 2200] bị từ chối.
    assert!(!call(&mut e, json!({"cmd": "create_furniture", "kind": "BED", "width": 3000})).ok);
    // Một undo bỏ hẳn giường.
    call(&mut e, json!({"cmd": "undo"}));
    call(&mut e, json!({"cmd": "undo"}));
    call(&mut e, json!({"cmd": "undo"}));
    assert!(!e.doc.objects.contains_key(&bed));
}

#[test]
fn desk_with_drawer_unit_hutch_and_cable_hole() {
    use aic_domain::layout::PartKind;
    use aic_domain::MachiningFeature;
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_furniture", "kind": "DESK", "width": 1200, "height": 750, "depth": 600, "options": {"desk_hutch_h": "600", "desk_hutch_shelves": "2"}}));
    assert!(r.ok, "{:?}", r.error);
    let desk: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let l = e.doc.cabinet_layout(desk).unwrap();
    let top = l.parts.iter().find(|p| p.key == "k:top").unwrap();
    assert!((top.size[0] - 1200.0).abs() < 1e-6 && (top.size[2] - 25.0).abs() < 1e-6);
    let PartKind::Panel { features, .. } = &top.kind else { panic!() };
    assert!(features.iter().any(|f| matches!(f, MachiningFeature::Contour(c) if c.inner)), "cable hole");
    // Hộc phải 3 ngăn, rộng 400 (khóa); kệ trên 2 tầng; yếm.
    let fronts = l.parts.iter().filter(|p| matches!(p.kind, PartKind::Panel { role: aic_domain::PanelRole::DrawerFront, .. })).count();
    assert_eq!(fronts, 3);
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("KệTrên")).count(), 2);
    assert!(l.parts.iter().any(|p| p.name == "Yếm"));
    let unit_x = |e: &Engine| {
        let l = e.doc.cabinet_layout(desk).unwrap();
        let a = l.parts.iter().find(|p| p.key == "k:unit_1_l").unwrap().translation[0];
        let b = l.parts.iter().find(|p| p.key == "k:unit_1_r").unwrap().translation[0];
        b + 17.2 - a
    };
    assert!((unit_x(&e) - 400.0).abs() < 1e-6);
    call(&mut e, json!({"cmd": "set_parameter", "id": desk, "name": "width", "value": "1400"}));
    assert!((unit_x(&e) - 400.0).abs() < 1e-6, "unit width locked when the desk grows");
    // Chân sắt trái → 2 chân.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": desk, "name": "desk_support_left", "value": "LEG"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(e.doc.cabinet_layout(desk).unwrap().fittings.legs, 2);
}

#[test]
fn wall_cladding_modules_follow_width_and_battens() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_furniture", "kind": "CLADDING", "width": 3000, "height": 2700}));
    assert!(r.ok, "{:?}", r.error);
    let w: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let boards = |e: &Engine| e.doc.cabinet_layout(w).unwrap().parts.into_iter().filter(|p| p.name.starts_with("TấmỐp")).collect::<Vec<_>>();
    let b = boards(&e);
    assert_eq!(b.len(), 5);
    let pw = (3000.0 - 4.0 * 3.0) / 5.0;
    assert!(b.iter().all(|p| (p.size[0] - pw).abs() < 1e-6 && (p.size[1] - 2700.0).abs() < 1e-6));
    call(&mut e, json!({"cmd": "set_parameter", "id": w, "name": "width", "value": "3200"}));
    assert!((boards(&e)[0].size[0] - (3200.0 - 12.0) / 5.0).abs() < 1e-6, "modules follow the width");
    // Khoét hộp điện giữa vách, cao 300.
    let r = call(&mut e, json!({"cmd": "set_back_cutouts", "cabinet": w, "cutouts": [{"kind": "SOCKET", "x": 0, "y": 300, "w": 80, "h": 80}]}));
    assert!(r.ok, "{:?}", r.error);
    let with_hole = boards(&e).iter().filter(|p| matches!(&p.kind, aic_domain::layout::PartKind::Panel { features, .. } if !features.is_empty())).count();
    assert_eq!(with_hole, 1);
    // Công thức sai bị từ chối; lam dọc 40 / 25.
    assert!(!call(&mut e, json!({"cmd": "set_parameter", "id": w, "name": "cl_cols", "value": "abc"})).ok);
    assert!(call(&mut e, json!({"cmd": "set_parameter", "id": w, "name": "cl_batten", "value": "on"})).ok);
    let lams = e.doc.cabinet_layout(w).unwrap().parts.iter().filter(|p| p.name.starts_with("Lam_")).count();
    assert_eq!(lams, ((3200.0 + 25.0) / 65.0_f64).floor() as usize);
}

#[test]
fn sliding_doors_follow_the_track_system_with_alu_frame_and_glass() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE", "overrides": {"width": 2400, "height": 2600, "depth": 650}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"][0]["id"].as_u64().unwrap();
    // Tủ áo có sẵn cánh ở khoang con; bỏ, rồi đặt 3 cánh lùa cho cả tủ.
    let r = call(&mut e, json!({"cmd": "zone_add_doors", "cabinet": cab, "zones": [root], "kind": "SLIDING", "cols": 3, "mount": "OVERLAY"}));
    assert!(r.ok, "{:?}", r.error);
    let leaves = |e: &Engine| e.doc.cabinet_layout(cab).unwrap().parts.into_iter().filter(|p| p.name.starts_with("CửaLùa_")).collect::<Vec<_>>();
    let door = leaves(&e)[0].clone();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let all: Vec<Value> = tree.result["roots"].as_array().unwrap().iter().flat_map(|r| r["children"].as_array().unwrap().clone()).collect();
    let did: ObjectId = serde_json::from_value(all.iter().find(|k| k["name"] == door.name.as_str()).unwrap()["id"].clone()).unwrap();
    for (k, v) in [("door_slide_overlap", "35"), ("door_slide_deduct_top", "0"), ("door_slide_deduct_bottom", "0")] {
        let r = call(&mut e, json!({"cmd": "set_parameter", "id": did, "name": k, "value": v}));
        assert!(r.ok, "{k}: {:?}", r.error);
    }
    let l = leaves(&e);
    assert_eq!(l.len(), 3);
    let track = e.doc.cabinet_layout(cab).unwrap().parts.into_iter().find(|p| p.name == "RayLùaTrên").unwrap();
    let total = track.size[0];
    assert!((l[0].size[0] - (total + 2.0 * 35.0) / 3.0).abs() < 1e-6, "leaf = (W + 2 × overlap) / 3");
    // Khung nhôm bản 20 + kính → 3 tấm kính, profile theo mét, không còn cánh ván.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": did, "name": "door_slide_frame", "value": "ALU_THIN"}));
    assert!(r.ok, "{:?}", r.error);
    // Cánh giờ là khung nhôm + ô nhét: chọn thanh khung vẫn sửa được hệ ray của cánh.
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let all: Vec<Value> = tree.result["roots"].as_array().unwrap().iter().flat_map(|r| r["children"].as_array().unwrap().clone()).collect();
    let bar: ObjectId = serde_json::from_value(all.iter().find(|k| k["name"].as_str().is_some_and(|n| n.starts_with("KhungNhôm"))).unwrap()["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": bar, "name": "door_slide_infill", "value": "GLASS"}));
    assert!(r.ok, "{:?}", r.error);
    let lay = e.doc.cabinet_layout(cab).unwrap();
    assert_eq!(lay.parts.iter().filter(|p| p.name.starts_with("KínhCửa")).count(), 3);
    assert!(lay.fittings.alu_profile_mm > 3.0 * 2.0 * 2500.0);
    assert!(lay.fittings.glass_mm2 > 0.0);
    let c = call(&mut e, json!({"cmd": "get_costing"})).result.to_string();
    assert!(c.contains("Profile nhôm cánh") && c.contains("Kính / gương cánh"));
}

#[test]
fn lift_up_door_hk_on_wall_cabinet_and_height_check() {
    use aic_domain::layout::PartKind;
    use aic_domain::MachiningFeature;
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WALL", "overrides": {"width": 800, "height": 400, "depth": 350, "doors": 0, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"][0]["id"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "zone_add_doors", "cabinet": cab, "zones": [root], "kind": "LIFT_UP"}));
    assert!(r.ok, "{:?}", r.error);
    let l = e.doc.cabinet_layout(cab).unwrap();
    let doors: Vec<_> = l.parts.iter().filter(|p| p.name.starts_with("CửaLật")).collect();
    assert_eq!(doors.len(), 1);
    let PartKind::Panel { features, .. } = &doors[0].kind else { panic!() };
    let cups = features.iter().filter(|f| matches!(f, MachiningFeature::Drill(d) if d.purpose == aic_domain::DrillPurpose::HingeCup)).count();
    assert_eq!(cups, 2, "2 hinge cups on the top edge");
    assert_eq!(l.fittings.lifts.values().sum::<u32>(), 1, "one HK set");
    assert!(l.fittings.lifts.keys().all(|k| k.starts_with("HK-")));
    // Khoang cao 300 → lỗi LIFT_HEIGHT.
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WALL", "overrides": {"width": 800, "height": 334, "depth": 350, "doors": 0, "shelves": 0}}));
    let low: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": low})).result["zones"][0]["id"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "zone_add_doors", "cabinet": low, "zones": [root], "kind": "LIFT_UP"}));
    assert_eq!(r.error.unwrap().details["constraint"], "LIFT_HEIGHT");
    // Cánh gập 2 lá + khung nhôm kính.
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WALL", "overrides": {"width": 900, "height": 700, "depth": 350, "doors": 0, "shelves": 0}}));
    let tall: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": tall})).result["zones"][0]["id"].as_u64().unwrap();
    assert!(call(&mut e, json!({"cmd": "zone_add_doors", "cabinet": tall, "zones": [root], "kind": "FOLD"})).ok);
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let all: Vec<Value> = tree.result["roots"].as_array().unwrap().iter().flat_map(|r| r["children"].as_array().unwrap().clone()).collect();
    let did: ObjectId = serde_json::from_value(all.iter().rfind(|k| k["name"].as_str().is_some_and(|n| n.starts_with("CửaLật"))).unwrap()["id"].clone()).unwrap();
    for (k, v) in [("door_lift", "HF"), ("door_glass_frame", "ALU_THIN")] {
        let r = call(&mut e, json!({"cmd": "set_parameter", "id": did, "name": k, "value": v}));
        assert!(r.ok, "{k}: {:?}", r.error);
    }
    let l = e.doc.cabinet_layout(tall).unwrap();
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("KhungNhôm")).count(), 8, "2 leaves × 4 bars");
    assert!(l.fittings.lifts.keys().all(|k| k.starts_with("HF-")));
}

#[test]
fn accessories_check_the_zone_and_warn_without_blocking_resize() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 798.4, "doors": 0, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"][0]["id"].as_u64().unwrap();
    // Khoang 764 → giá bát 800 không vừa: vẫn thêm, trả cảnh báo, get_zones có misfits.
    let list = call(&mut e, json!({"cmd": "get_accessories", "cabinet": cab, "zones": [root]}));
    let dish = list.result["accessories"].as_array().unwrap().iter().find(|a| a["code"] == "DISH-800").unwrap().clone();
    assert_eq!(dish["fits"], false);
    let r = call(&mut e, json!({"cmd": "zone_add_link", "cabinet": cab, "zones": [root], "kind": "ACCESSORY", "code": "DISH-800"}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(r.result["misfit_zones"].as_array().unwrap().len(), 1);
    assert_eq!(call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["misfits"].as_array().unwrap().len(), 1);
    // Đổi tủ rộng 800 → khoang 765.6 → hết cảnh báo; báo giá có giá bát + ray.
    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "800"}));
    assert!(r.ok, "{:?}", r.error);
    assert!(call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["misfits"].as_array().unwrap().is_empty());
    let l = e.doc.cabinet_layout(cab).unwrap();
    assert_eq!(l.fittings.accessories.get("DISH-800"), Some(&1));
    assert_eq!(l.fittings.slides.values().sum::<u32>(), 1);
    // Thu nhỏ lại: không bị chặn, chỉ cảnh báo.
    assert!(call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "700"})).ok);
    assert_eq!(call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["misfits"].as_array().unwrap().len(), 1);
    // LED theo mét + nguồn; mã lạ bị từ chối.
    assert!(call(&mut e, json!({"cmd": "zone_add_link", "cabinet": cab, "zones": [root], "kind": "ACCESSORY", "code": "LED-STRIP"})).ok);
    let c = call(&mut e, json!({"cmd": "get_costing"})).result.to_string();
    assert!(c.contains("Đèn LED thanh nhôm") && c.contains("Nguồn LED") && c.contains("Giá bát đĩa 800"));
    assert!(!call(&mut e, json!({"cmd": "zone_add_link", "cabinet": cab, "zones": [root], "kind": "ACCESSORY", "code": "NOPE"})).ok);
}

#[test]
fn appliance_bay_oven_with_support_vent_and_fit_check() {
    use aic_domain::layout::PartKind;
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 600, "height": 2300, "depth": 580, "doors": 0, "shelves": 0}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let root = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"][0]["id"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "split_zone", "cabinet": cab, "zone": root, "kind": "SHELF_FIXED", "formula": "720,610,*", "from_end": false}));
    assert!(r.ok, "{:?}", r.error);
    let zs = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"].clone();
    let leaves: Vec<&Value> = zs.as_array().unwrap().iter().filter(|z| z["leaf"] == true).collect();
    let mid = leaves.iter().find(|z| (z["size"][1].as_f64().unwrap() - 610.0).abs() < 1.0).expect("610 bay")["id"].as_u64().unwrap();
    // Khoang lò ≥ 560 × 590 × 550.
    let r = call(&mut e, json!({"cmd": "zone_add_link", "cabinet": cab, "zones": [mid], "kind": "APPLIANCE_BAY", "code": "OVEN-600"}));
    assert!(r.ok, "{:?}", r.error);
    let l = e.doc.cabinet_layout(cab).unwrap();
    assert!(l.parts.iter().any(|p| p.name.starts_with("ThanhĐỡThiếtBị")));
    assert_eq!(l.fittings.appliances.get("OVEN-600"), Some(&1));
    let back = l.parts.iter().find(|p| p.key == "c:back").unwrap();
    let PartKind::Panel { features, .. } = &back.kind else { panic!() };
    assert!(features.iter().any(|f| matches!(f, aic_domain::MachiningFeature::Contour(c) if c.inner)), "vent cut in the back");
    // Khoang thấp → lỗi APPLIANCE_FIT, không đổi dữ liệu.
    let low = leaves.iter().find(|z| (z["size"][1].as_f64().unwrap() - 720.0).abs() < 1.0).unwrap()["id"].as_u64().unwrap();
    let r = call(&mut e, json!({"cmd": "zone_add_link", "cabinet": cab, "zones": [low], "kind": "APPLIANCE_BAY", "code": "FRIDGE-600"}));
    assert_eq!(r.error.unwrap().details["constraint"], "APPLIANCE_FIT");
}

#[test]
fn drawing_sheet_kitchen_elevation_on_one_a3() {
    let mut e = Engine::new();
    let mut prev: Option<ObjectId> = None;
    let mut widths = 0.0;
    for w in [800.0, 600.0, 900.0] {
        let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "room": "Bếp", "after": prev, "overrides": {"width": w}}));
        assert!(r.ok, "{:?}", r.error);
        prev = Some(serde_json::from_value(r.result["id"].clone()).unwrap());
        widths += w;
    }
    for x in [0.0, 800.0] {
        let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WALL", "room": "Bếp", "position": [x, 1450, 0], "overrides": {"width": 800}}));
        assert!(r.ok, "{:?}", r.error);
    }
    let r = call(&mut e, json!({"cmd": "get_drawing_sheet", "room": "Bếp", "paper": "A3", "drawer": "KTS", "date": "30/09/2026"}));
    assert!(r.ok, "{:?}", r.error);
    let sheets = r.result["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 1, "one A3 page");
    assert_eq!(sheets[0]["paper"], json!([420.0, 297.0]));
    let items = sheets[0]["items"].as_array().unwrap();
    let dims: Vec<f64> = items.iter().filter(|i| i["cls"] == "dimtext").filter_map(|i| i["value"].as_f64()).collect();
    assert!(dims.iter().any(|d| (d - widths).abs() < 0.5), "total chain = sum of base widths ({widths}): {dims:?}");
    assert!(items.iter().any(|i| i["cls"] == "open"), "door opening symbols");
    assert!(items.iter().any(|i| i["t"] == "text" && i["s"] == "KTS"), "title block");
    let titles: Vec<&str> = sheets[0]["views"].as_array().unwrap().iter().map(|v| v["title"].as_str().unwrap()).collect();
    assert!(titles.contains(&"Mặt đứng") && titles.contains(&"Mặt bằng"), "{titles:?}");
    // Chi tiết từng tủ: nhiều hình → có thể nhiều trang, cùng tỷ lệ chuẩn.
    let r = call(&mut e, json!({"cmd": "get_drawing_sheet", "room": "Bếp", "paper": "A4", "views": ["DETAIL"], "hide_fronts": true}));
    assert!(r.ok, "{:?}", r.error);
    assert!(r.result["scale"].as_str().unwrap().starts_with("1:"));
    let n: usize = r.result["sheets"].as_array().unwrap().iter().map(|s| s["views"].as_array().unwrap().len()).sum();
    assert_eq!(n, 10, "front + side per cabinet");
}

#[test]
fn builtin_templates_insert_with_params_one_undo_each() {
    let mut e = Engine::new();
    let list = call(&mut e, json!({"cmd": "get_products"})).result["products"].as_array().unwrap().clone();
    assert!(list.len() >= 13, "≥ 13 mẫu dựng sẵn");
    for p in &list {
        let r = call(&mut e, json!({"cmd": "insert_product", "key": p["key"]}));
        assert!(r.ok, "{}: {:?}", p["key"], r.error);
        let id: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
        assert!(!e.doc.cabinet_layout(id).unwrap().parts.is_empty(), "{}", p["key"]);
        assert!(e.doc.cabinet_layout(id).unwrap().problems.is_empty(), "{} has unsolvable zones", p["key"]);
    }
    // Tủ áo 4 cánh 1800 với W = 2000, kịch trần 2700 (che trần 50).
    let r = call(&mut e, json!({"cmd": "insert_product", "key": "WARDROBE_4D_1800", "width": 2000, "params": {"ceiling": "on", "ceiling_h": "2700", "ceiling_gap": "50"}}));
    assert!(r.ok, "{:?}", r.error);
    let id: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    assert_eq!(e.doc.param_value(id, "width"), Some(2000.0));
    assert_eq!(e.doc.param_value(id, "height"), Some(2650.0));
    let l = e.doc.cabinet_layout(id).unwrap();
    let doors: Vec<_> = l.parts.iter().filter(|p| p.name.starts_with("CửaĐôi")).collect();
    assert_eq!(doors.len(), 8, "2 khoang × (2 cánh dưới + 2 cánh trên)");
    let lower: Vec<f64> = doors.iter().filter(|d| d.size[1] > 1000.0).map(|d| d.size[0]).collect();
    assert_eq!(lower.len(), 4);
    assert!(lower.iter().all(|w| (w - lower[0]).abs() < 1.0), "4 cánh dưới rộng đều: {lower:?}");
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("ThanhOval")).count(), 2, "2 khoang treo");
    assert_eq!(l.parts.iter().filter(|p| p.name.starts_with("MặtNgănTrong")).count(), 6);
    // Một undo bỏ cả mẫu.
    call(&mut e, json!({"cmd": "undo"}));
    assert!(!e.doc.objects.contains_key(&id));
}

#[test]
fn island_front_drawers_rear_doors_face_back_no_back_panel() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_furniture", "kind": "ISLAND", "width": 1800, "height": 900, "depth": 900}));
    assert!(r.ok, "{:?}", r.error);
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let zs = call(&mut e, json!({"cmd": "get_zones", "cabinet": cab})).result["zones"].clone();
    let leaves: Vec<Value> = zs.as_array().unwrap().iter().filter(|z| z["leaf"] == true).cloned().collect();
    assert_eq!(leaves.len(), 2);
    let (rear, front) = if leaves[0]["min"][2].as_f64() < leaves[1]["min"][2].as_f64() { (&leaves[0], &leaves[1]) } else { (&leaves[1], &leaves[0]) };
    let (rear, front) = (rear["id"].as_u64().unwrap(), front["id"].as_u64().unwrap());
    assert!(call(&mut e, json!({"cmd": "zone_add_drawers", "cabinet": cab, "zones": [front], "count": 3})).ok);
    assert!(call(&mut e, json!({"cmd": "zone_add_doors", "cabinet": cab, "zones": [rear], "kind": "DOUBLE", "cols": 2})).ok);
    let l = e.doc.cabinet_layout(cab).unwrap();
    assert!(!l.parts.iter().any(|p| p.key.starts_with("c:back")), "no back panel");
    assert!(l.parts.iter().any(|p| p.name.starts_with("HậuPhụ")), "vách giữa");
    let aabb = |p: &aic_domain::Part| aic_math::Obb::new(p.size, aic_math::Transform3D::new(p.translation, p.rotation_deg)).aabb();
    let fronts: Vec<_> = l.parts.iter().filter(|p| p.name.starts_with("MặtNgăn")).collect();
    assert_eq!(fronts.len(), 3);
    assert!(fronts.iter().all(|p| aabb(p).min[2] > 800.0), "drawers at the front");
    let doors: Vec<_> = l.parts.iter().filter(|p| p.name.starts_with("CửaĐôi")).collect();
    assert_eq!(doors.len(), 2);
    assert!(doors.iter().all(|p| aabb(p).max[2] < 50.0), "rear doors face the back: {:?}", doors.iter().map(|p| aabb(p)).collect::<Vec<_>>());
    // Mặt đá nhô phía ghế 300.
    let top = l.parts.iter().find(|p| p.key == "c:island_top").unwrap();
    let tb = aabb(top);
    assert!((tb.min[2] + 300.0).abs() < 1e-6 && (tb.max[2] - 920.0).abs() < 1e-6, "{tb:?}");
    assert_eq!(call(&mut e, json!({"cmd": "get_structure", "cabinet": cab})).result["tabs"][0]["key"], "island");
}

#[test]
fn export_machine_files_dxf_layers_mpr_cix() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "BASE", "overrides": {"width": 800}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    assert!(call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "s_joint_type", "value": "CAM_DOWEL"})).ok);
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    let all: Vec<Value> = tree.result["roots"].as_array().unwrap().iter().flat_map(|r| r["children"].as_array().unwrap().clone()).collect();
    let ids: Vec<Value> = all.iter().filter(|k| k["kind"] == "PANEL").map(|k| k["id"].clone()).collect();
    let r = call(&mut e, json!({"cmd": "export_machine", "ids": ids, "format": "DXF"}));
    assert!(r.ok, "{:?}", r.error);
    let files = r.result["files"].as_array().unwrap();
    let all_dxf: String = files.iter().map(|f| f["content"].as_str().unwrap()).collect();
    let layers: std::collections::BTreeSet<&str> = all_dxf.lines().collect::<Vec<_>>().windows(2).filter(|w| w[0] == "8").map(|w| w[1]).collect();
    assert!(layers.contains("CUT"));
    assert!(layers.contains("DRILL_15_12.5"), "cam Ø15 sâu 12.5: {layers:?}");
    assert!(layers.iter().any(|l| l.starts_with("HDRILL_8_")), "chốt gỗ khoan cạnh Ø8: {layers:?}");
    assert!(files.iter().all(|f| f["name"].as_str().unwrap().ends_with(".dxf")));
    for fmt in ["MPR", "CIX"] {
        let r = call(&mut e, json!({"cmd": "export_machine", "ids": [ids[0]], "format": fmt}));
        assert!(r.ok, "{fmt}: {:?}", r.error);
        let c = r.result["files"][0]["content"].as_str().unwrap();
        assert!(if fmt == "MPR" { c.contains("_BSX=") } else { c.contains("BEGIN MAINDATA") && c.contains("LPX=") });
    }
    assert!(!call(&mut e, json!({"cmd": "export_machine", "ids": [ids[0]], "format": "XYZ"})).ok);
}

#[test]
fn array_cabinet_by_size_formula_on_any_axis() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WALL", "overrides": {"width": 600}}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let r = call(&mut e, json!({"cmd": "array_cabinet", "id": cab, "count": 3, "axis": 0, "gap": 0, "sizes": "400,600,800"}));
    assert!(r.ok, "{:?}", r.error);
    let ids: Vec<ObjectId> = serde_json::from_value(r.result["created"].clone()).unwrap();
    assert_eq!(ids.len(), 3);
    let widths: Vec<f64> = ids.iter().map(|i| e.doc.param_value(*i, "width").unwrap()).collect();
    assert_eq!(widths, vec![400.0, 600.0, 800.0]);
    let x: Vec<f64> = ids.iter().map(|i| e.doc.world_aabb(*i).min[0]).collect();
    let x0 = e.doc.world_aabb(cab).min[0];
    assert!((x[0] - (x0 + 600.0)).abs() < 1e-6 && (x[1] - (x0 + 1000.0)).abs() < 1e-6 && (x[2] - (x0 + 1600.0)).abs() < 1e-6, "{x:?}");
    // Một undo bỏ cả 3; trục Y (chồng tủ) với khe 10.
    call(&mut e, json!({"cmd": "undo"}));
    assert!(ids.iter().all(|i| !e.doc.objects.contains_key(i)));
    let r = call(&mut e, json!({"cmd": "array_cabinet", "id": cab, "count": 2, "axis": 1, "gap": 10}));
    assert!(r.ok, "{:?}", r.error);
    assert!(!call(&mut e, json!({"cmd": "array_cabinet", "id": cab, "count": 1, "sizes": "abc"})).ok);
}

#[test]
fn team_library_shared_folder_between_two_machines() {
    let dir = std::env::temp_dir().join(format!("aic-team-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let shared = dir.join("cong-ty");
    let src = |ro: bool| json!([{"name": "Công ty", "path": shared.display().to_string(), "readonly": ro}]);
    // Máy A: lưu chuẩn xưởng "MFC 18" rồi đẩy lên thư mục chung.
    let mut a = Engine::new();
    a.set_library_path(Some(dir.join("a").join("library.json")));
    assert!(call(&mut a, json!({"cmd": "set_library_sources", "sources": src(false)})).ok);
    let r = call(&mut a, json!({"cmd": "create_cabinet", "kind": "BASE"}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    assert!(call(&mut a, json!({"cmd": "save_group_preset", "cabinet": cab, "group": "all", "name": "MFC 18"})).ok);
    let r = call(&mut a, json!({"cmd": "publish_library_item", "source": "Công ty", "kind": "groups", "group": "all", "name": "MFC 18"}));
    assert!(r.ok, "{:?}", r.error);
    // Máy B: trỏ cùng thư mục → thấy "MFC 18"; file máy B không chép mục nguồn.
    let mut b = Engine::new();
    b.set_library_path(Some(dir.join("b").join("library.json")));
    let r = call(&mut b, json!({"cmd": "set_library_sources", "sources": src(true)}));
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(r.result["sources"][0]["items"], 1);
    let rb = call(&mut b, json!({"cmd": "create_cabinet", "kind": "BASE"}));
    let cb: ObjectId = serde_json::from_value(rb.result["id"].clone()).unwrap();
    let st = call(&mut b, json!({"cmd": "get_structure", "cabinet": cb}));
    assert!(st.result["standards"].as_array().unwrap().iter().any(|s| s == "MFC 18"), "{}", st.result["standards"]);
    assert!(call(&mut b, json!({"cmd": "apply_group_preset", "ids": [cb], "group": "all", "name": "MFC 18"})).ok);
    let local_b = std::fs::read_to_string(dir.join("b").join("library.json")).unwrap();
    assert!(!local_b.contains("MFC 18"), "remote items are not copied into the machine library");
    // Nguồn chỉ đọc: không đẩy được.
    assert!(call(&mut b, json!({"cmd": "save_group_preset", "cabinet": cb, "group": "all", "name": "B riêng"})).ok);
    let r = call(&mut b, json!({"cmd": "publish_library_item", "source": "Công ty", "kind": "groups", "group": "all", "name": "B riêng"}));
    assert_eq!(r.error.unwrap().details["constraint"], "LIBRARY_READONLY");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn grain_matched_doors_are_nested_side_by_side() {
    let mut e = Engine::new();
    let r = call(&mut e, json!({"cmd": "insert_product", "key": "WARDROBE_4D_1800"}));
    let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
    let tree = call(&mut e, json!({"cmd": "get_scene_tree"}));
    fn walk(v: &Value, out: &mut Vec<Value>) {
        out.push(v.clone());
        for c in v["children"].as_array().into_iter().flatten() {
            walk(c, out);
        }
    }
    let mut all = Vec::new();
    for r in tree.result["roots"].as_array().unwrap() {
        walk(r, &mut all);
    }
    let _ = cab;
    // 4 cánh dưới (cao) nối vân ngang.
    let doors: Vec<(ObjectId, f64)> = all
        .iter()
        .filter(|n| n["name"].as_str().is_some_and(|s| s.starts_with("CửaĐôi")))
        .map(|n| serde_json::from_value::<ObjectId>(n["id"].clone()).unwrap())
        .map(|id| (id, e.doc.world_aabb(id).max[1] - e.doc.world_aabb(id).min[1]))
        .filter(|(_, h)| *h > 1000.0)
        .collect();
    assert_eq!(doors.len(), 4);
    // 2 cánh của khoang trái (vừa khổ ván 1220 khi đặt cạnh nhau).
    let ids: Vec<ObjectId> = doors.iter().take(2).map(|d| d.0).collect();
    let r = call(&mut e, json!({"cmd": "set_grain_group", "ids": ids, "group": "Cánh tủ áo"}));
    assert!(r.ok, "{:?}", r.error);
    let mat = e.doc.panel(ids[0]).unwrap().material_id.0.clone();
    let n = call(&mut e, json!({"cmd": "run_nesting", "material": mat}));
    assert!(n.ok, "{:?}", n.error);
    let pl = n.result["jobs"][0]["result"]["placements"].as_array().unwrap().clone();
    let mut mine: Vec<&Value> = pl.iter().filter(|p| ids.iter().any(|i| serde_json::to_value(i).unwrap() == p["part_id"])).collect();
    assert_eq!(mine.len(), 2);
    let sheet = mine[0]["sheet_id"].clone();
    let rot = mine[0]["rotation_deg"].clone();
    assert!(mine.iter().all(|p| p["sheet_id"] == sheet && p["rotation_deg"] == rot), "same sheet, same direction");
    // Liền nhau: khoảng hở giữa 2 tấm kề = khoảng cách dao.
    let horiz = mine.iter().all(|p| (p["y_mm"].as_f64().unwrap() - mine[0]["y_mm"].as_f64().unwrap()).abs() < 1e-6);
    let key = if horiz { "x_mm" } else { "y_mm" };
    let size = if horiz { "width_mm" } else { "height_mm" };
    mine.sort_by(|a, b| a[key].as_f64().unwrap().total_cmp(&b[key].as_f64().unwrap()));
    for w in mine.windows(2) {
        let gap = w[1][key].as_f64().unwrap() - (w[0][key].as_f64().unwrap() + w[0][size].as_f64().unwrap());
        assert!((gap - 12.0).abs() < 1e-6, "gap {gap}");
    }
    // Một undo bỏ nhóm.
    call(&mut e, json!({"cmd": "undo"}));
    assert!(e.grain_group_of(ids[0]).is_none());
}
