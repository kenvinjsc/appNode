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
