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
    assert_eq!(tree.result["roots"][0]["children"].as_array().unwrap().len(), 15);

    // Render all, then only changed geometry after a width edit.
    let all = call(&mut e, json!({"cmd": "get_render_objects"}));
    let objects = all.result["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 15);
    let known: Vec<String> = all.result["meshes"].as_object().unwrap().keys().cloned().collect();
    assert!(known.len() < objects.len(), "identical parts share geometry");

    let r = call(&mut e, json!({"cmd": "set_parameter", "id": cab, "name": "width", "value": "800"}));
    assert!(r.ok);
    let geo = r.events.iter().find_map(|ev| match ev { CoreEvent::GeometryChanged { ids } => Some(ids.clone()), _ => None }).unwrap();
    assert!(geo.len() >= 5 && geo.len() < 15, "only affected parts: {geo:?}");
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
    assert_eq!(e.doc.param_value(cab, "width"), Some(1000.0));
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
    assert_eq!(md.result["summary"]["drills"].as_u64().unwrap(), 3, "3 hinge cups on a tall door");

    // Parts, nesting, CNC.
    let parts = call(&mut e, json!({"cmd": "get_parts"}));
    assert_eq!(parts.result["parts"].as_array().unwrap().len(), 12);
    let nest = call(&mut e, json!({"cmd": "run_nesting", "material": "MDF18-WHITE"}));
    assert!(nest.ok, "{:?}", nest.error);
    let job = &nest.result["jobs"][0];
    assert!(job["result"]["unplaced"].as_array().unwrap().is_empty());
    let cnc = call(&mut e, json!({"cmd": "generate_cnc", "material": "MDF18-WHITE", "sheet_id": 0}));
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
    let s = call(&mut e, json!({"cmd": "snap", "id": b, "delta": [-(bx - 1000.0) + 7.0, 0.0, 0.0]}));
    assert!((s.result["delta"][0].as_f64().unwrap() + (bx - 1000.0)).abs() < 1e-6, "{}", s.result);
    let del = call(&mut e, json!({"cmd": "delete_objects", "ids": [b]}));
    assert!(del.events.iter().any(|ev| matches!(ev, CoreEvent::SelectionInvalidated { .. })));
}

#[test]
fn bad_request_is_reported() {
    let mut e = Engine::new();
    let out = e.dispatch_json(r#"{"cmd":"nope"}"#);
    assert!(out.contains("\"ok\":false"));
}
