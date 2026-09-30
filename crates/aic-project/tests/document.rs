use aic_domain::zone::{DoorKind, DoorSpec, Front, HingeSide, Lock, Mount, SplitKind, StopRailSpec};
use aic_domain::{CabinetKind, CabinetSpec, DomainObject, MaterialId, PanelRole};
use aic_math::Transform3D;
use aic_project::{Command, CoreError, Document, History, ProjectFile};

const T: f64 = 17.2;

fn cabinet(kind: CabinetKind) -> (Document, History, aic_domain::ObjectId) {
    let mut doc = Document::new("Test");
    let mut h = History::new();
    h.execute(&mut doc, Command::CreateCabinet { id: None, spec: CabinetSpec::preset(kind), position: [0.0; 3], parent: None, name: None }).unwrap();
    let cab = *doc.scene.roots().last().unwrap();
    doc.take_changes();
    (doc, h, cab)
}

fn wardrobe() -> (Document, History, aic_domain::ObjectId) {
    cabinet(CabinetKind::Wardrobe)
}

fn find(doc: &Document, role: PanelRole, idx: u32) -> &aic_domain::Panel {
    doc.objects.values().filter_map(|o| o.as_panel()).find(|p| p.role == role && p.role_index == idx).unwrap()
}

fn count(doc: &Document, role: PanelRole) -> usize {
    doc.objects.values().filter(|o| matches!(o, DomainObject::Panel(p) if p.role == role)).count()
}

/// Adjustable shelf width in the left bay of a wardrobe of width `w` (divider at 50%).
fn left_shelf_w(w: f64) -> f64 {
    (w - 3.0 * T) / 2.0 - 1.0
}

#[test]
fn wardrobe_preset_and_parameter_update() {
    let (mut doc, mut h, cab) = wardrobe();
    let shelf = find(&doc, PanelRole::Shelf, 0).clone();
    assert_eq!(shelf.width_mm, left_shelf_w(1600.0));
    assert_eq!(count(&doc, PanelRole::Shelf), 4);
    assert_eq!(count(&doc, PanelRole::Divider), 1);
    assert_eq!(count(&doc, PanelRole::Door), 4);
    assert!(shelf.name.starts_with("KệDiĐộng_"));
    h.execute(&mut doc, Command::SetParameter { id: cab, name: "width".into(), value: "1200".into() }).unwrap();
    assert_eq!(find(&doc, PanelRole::Shelf, 0).width_mm, left_shelf_w(1200.0));
    assert_eq!(find(&doc, PanelRole::Shelf, 0).id, shelf.id, "ids are stable across re-flow");
    let right = find(&doc, PanelRole::RightSide, 0).id;
    assert_eq!(doc.scene.world(right).translation[0], 1200.0 - T);
    let ch = doc.take_changes();
    assert!(ch.geometry.contains(&shelf.id));
    assert!(ch.transform.contains(&right));
    assert!(!ch.geometry.contains(&find(&doc, PanelRole::LeftSide, 0).id), "left side size unchanged");
    h.undo(&mut doc).unwrap();
    assert_eq!(find(&doc, PanelRole::Shelf, 0).width_mm, left_shelf_w(1600.0));
}

#[test]
fn expression_on_panel_parameter() {
    let (mut doc, mut h, _) = wardrobe();
    let shelf = find(&doc, PanelRole::Shelf, 1).id;
    h.execute(&mut doc, Command::SetParameter { id: shelf, name: "width".into(), value: "= cabinet.inner_width - 100".into() }).unwrap();
    assert_eq!(doc.panel(shelf).unwrap().width_mm, 1600.0 - 2.0 * T - 100.0);
    let err = h.execute(&mut doc, Command::SetParameter { id: shelf, name: "width".into(), value: "= width + 1".into() }).unwrap_err();
    assert!(matches!(err, CoreError::DependencyCycle { .. }), "{err:?}");
    let err = h.execute(&mut doc, Command::SetParameter { id: shelf, name: "width".into(), value: "= 1 +".into() }).unwrap_err();
    assert_eq!(err.code(), "INVALID_PARAMETER");
}

#[test]
fn constraint_violation_is_rejected() {
    let (mut doc, mut h, cab) = wardrobe();
    let err = h.execute(&mut doc, Command::SetParameter { id: cab, name: "width".into(), value: "30".into() }).unwrap_err();
    assert!(matches!(&err, CoreError::ConstraintViolated { constraint, .. } if constraint == "WIDTH_LESS_THAN_SIDES"), "{err:?}");
    assert_eq!(doc.param_value(cab, "width"), Some(1600.0));
}

#[test]
fn structural_change_keeps_ids_and_undoes() {
    let (mut doc, mut h, cab) = wardrobe();
    let s0 = find(&doc, PanelRole::Shelf, 0).id;
    h.execute(&mut doc, Command::SetParameter { id: cab, name: "shelves".into(), value: "2".into() }).unwrap();
    assert_eq!(count(&doc, PanelRole::Shelf), 2);
    assert_eq!(find(&doc, PanelRole::Shelf, 0).id, s0, "stable id");
    h.execute(&mut doc, Command::SetParameter { id: cab, name: "doors".into(), value: "0".into() }).unwrap();
    assert_eq!(count(&doc, PanelRole::Door), 0);
    h.undo(&mut doc).unwrap();
    h.undo(&mut doc).unwrap();
    assert_eq!(count(&doc, PanelRole::Shelf), 4);
    h.redo(&mut doc).unwrap();
    assert_eq!(count(&doc, PanelRole::Shelf), 2);
}

#[test]
fn zone_editing_through_set_cabinet() {
    let (mut doc, mut h, cab) = cabinet(CabinetKind::Base);
    // P1: base kitchen unit = 5 boards 17.2 + 1 back 8.6 (no shelves by default here: preset has 1 shelf).
    let boards = doc.objects.values().filter_map(|o| o.as_panel()).filter(|p| p.thickness_mm == T && p.role != PanelRole::Shelf && p.role != PanelRole::Door).count();
    assert_eq!(boards, 5);
    assert_eq!(doc.objects.values().filter_map(|o| o.as_panel()).filter(|p| p.thickness_mm == 8.6).count(), 1);

    // Add a divider at 50% in the root zone, then a double inset door on the left bay.
    let mut def = doc.object(cab).unwrap().as_cabinet().unwrap().clone();
    def.zones.set_front(def.zones.root.id, None).unwrap();
    def.zones.set_even_shelves(def.zones.root.id, 0, T).unwrap();
    let root = def.zones.root.id;
    def.zones.add_panels(root, SplitKind::Divider, 1, T, Lock::Ratio, 0.5).unwrap();
    let left = def.zones.root.split.as_ref().unwrap().children[0].id;
    let uid = def.zones.alloc();
    def.zones
        .set_front(left, Some(Front::Doors(DoorSpec { uid, kind: DoorKind::Single, cols: 1, rows: 1, mount: Mount::Inset, hinge: HingeSide::Right, thickness: None, gap: None, side_gaps: None, stop: StopRailSpec::default(), fixed: false, sliding: None })))
        .unwrap();
    h.execute(&mut doc, Command::SetCabinet { id: cab, cabinet: Box::new(def), label: "Zone".into() }).unwrap();
    assert_eq!(count(&doc, PanelRole::Divider), 1);
    let door = find(&doc, PanelRole::Door, 0);
    let bay = (800.0 - 3.0 * T) / 2.0;
    assert!((door.width_mm - (bay - 4.0)).abs() < 1e-9, "inset door = bay − 2 gaps");
    assert_eq!(door.hinge, Some(aic_domain::EdgeSide::Right));
    // Resizing the cabinet re-flows the zone content.
    h.execute(&mut doc, Command::SetParameter { id: cab, name: "width".into(), value: "1000".into() }).unwrap();
    let bay = (1000.0 - 3.0 * T) / 2.0;
    assert!((find(&doc, PanelRole::Door, 0).width_mm - (bay - 4.0)).abs() < 1e-9);
    h.undo(&mut doc).unwrap();
    h.undo(&mut doc).unwrap();
    assert_eq!(count(&doc, PanelRole::Divider), 0);
}

#[test]
fn delete_duplicate_undo_redo() {
    let (mut doc, mut h, cab) = wardrobe();
    let n = doc.objects.len();
    h.execute(&mut doc, Command::DuplicateObject { id: cab, offset: None }).unwrap();
    assert_eq!(doc.objects.len(), 2 * n);
    let copy = *doc.scene.roots().last().unwrap();
    assert_eq!(doc.scene.world(copy).translation[0], 1650.0);
    h.execute(&mut doc, Command::SetParameter { id: copy, name: "width".into(), value: "900".into() }).unwrap();
    assert_eq!(doc.param_value(cab, "width"), Some(1600.0), "copy is independent");
    h.execute(&mut doc, Command::DeleteObject { id: cab }).unwrap();
    assert_eq!(doc.objects.len(), n);
    h.undo(&mut doc).unwrap();
    assert_eq!(doc.objects.len(), 2 * n);
    assert_eq!(find(&doc, PanelRole::Shelf, 0).width_mm, left_shelf_w(1600.0));
}

#[test]
fn move_and_lock() {
    let (mut doc, mut h, cab) = wardrobe();
    h.execute(&mut doc, Command::SetTransform { id: cab, transform: Transform3D::new([500.0, 0.0, 0.0], [0.0, 90.0, 0.0]) }).unwrap();
    assert_eq!(doc.param_value(cab, "x"), Some(500.0));
    h.undo(&mut doc).unwrap();
    assert_eq!(doc.param_value(cab, "x"), Some(0.0));
    assert_eq!(doc.scene.node(cab).unwrap().local_transform.rotation_deg, [0.0; 3]);
    h.execute(&mut doc, Command::SetLocked { id: cab, locked: true }).unwrap();
    let shelf = find(&doc, PanelRole::Shelf, 0).id;
    let e = h.execute(&mut doc, Command::SetParameter { id: shelf, name: "width".into(), value: "10".into() }).unwrap_err();
    assert_eq!(e.code(), "LOCKED");
}

#[test]
fn material_and_features() {
    let (mut doc, mut h, cab) = wardrobe();
    h.execute(&mut doc, Command::SetMaterial { id: cab, material: MaterialId::new("PB18-WALNUT") }).unwrap();
    assert_eq!(find(&doc, PanelRole::LeftSide, 0).material_id.0, "PB18-WALNUT");
    assert_eq!(find(&doc, PanelRole::Door, 0).material_id.0, "MDF17-OAK", "front untouched");
    // A per-panel material survives re-flow.
    let door = find(&doc, PanelRole::Door, 0).id;
    h.execute(&mut doc, Command::SetMaterial { id: door, material: MaterialId::new("MDF17-WALNUT") }).unwrap();
    h.execute(&mut doc, Command::SetParameter { id: cab, name: "width".into(), value: "1500".into() }).unwrap();
    assert_eq!(doc.panel(door).unwrap().material_id.0, "MDF17-WALNUT");
    h.undo(&mut doc).unwrap();
    h.undo(&mut doc).unwrap();
    h.undo(&mut doc).unwrap();
    assert_eq!(find(&doc, PanelRole::LeftSide, 0).material_id.0, "MDF17-WHITE");
}

#[test]
fn save_load_identical() {
    let (mut doc, mut h, cab) = wardrobe();
    let shelf = find(&doc, PanelRole::Shelf, 1).id;
    h.execute(&mut doc, Command::SetParameter { id: shelf, name: "width".into(), value: "= cabinet.inner_width - 100".into() }).unwrap();
    h.execute(&mut doc, Command::CreateRoom { id: None, name: "Room 01".into(), width: 4000.0, depth: 3000.0, height: 2700.0 }).unwrap();
    let file = ProjectFile::from_document(&doc);
    let json = file.to_json();
    assert!(json.contains("\"format\": \"aic-project\""));
    assert!(json.contains("\"version\": 1"));
    let loaded = ProjectFile::from_json(&json).unwrap();
    assert_eq!(loaded, file);
    let doc2 = loaded.into_document().unwrap();
    assert_eq!(doc2.objects, doc.objects);
    assert_eq!(doc2.scene, doc.scene);
    assert_eq!(ProjectFile::from_document(&doc2), file);
    // Parameters still live after load.
    let mut doc2 = doc2;
    let mut h2 = History::new();
    h2.execute(&mut doc2, Command::SetParameter { id: cab, name: "width".into(), value: "1800".into() }).unwrap();
    assert_eq!(doc2.panel(shelf).unwrap().width_mm, 1800.0 - 2.0 * T - 100.0);
}

#[test]
fn migrates_v0() {
    let v0 = r##"{
        "name": "Old project",
        "scene": {"nodes": {"1": {"id":1,"parent":null,"children":[],"local_transform":{"translation":[0,0,0],"rotation_deg":[0,0,0]},"object_id":1,"visible":true,"locked":false}}, "roots":[1]},
        "objects": [{"type":"PANEL","id":1,"name":"P","role":"GENERIC","width_mm":600,"height_mm":500,"thickness_mm":18,"material_id":"MDF18-WHITE","grain_direction":"ALONG_HEIGHT","edge_bands":[],"features":[]}],
        "params": {"#1.width":"600","#1.height":"500","#1.thickness":"18","#1.x":"0","#1.y":"0","#1.z":"0"}
    }"##;
    let doc = ProjectFile::from_json(v0).unwrap().into_document().unwrap();
    assert_eq!(doc.meta.name, "Old project");
    assert_eq!(doc.panel(aic_domain::ObjectId(1)).unwrap().volume_mm3(), 600.0 * 500.0 * 18.0);
    assert!(ProjectFile::from_json(r#"{"format":"aic-project","version":99}"#).is_err());
}

