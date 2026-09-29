//! Scene tree and property sheets. Pure read models built from the document;
//! the UI renders them generically and sends edits back as `set_parameter`.

use crate::Engine;
use aic_domain::{DomainObject, EdgeSide, ObjectId};
use aic_project::{CoreError, Document};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
pub struct TreeNode {
    pub id: ObjectId,
    pub name: String,
    pub kind: &'static str,
    pub role: Option<String>,
    pub visible: bool,
    pub locked: bool,
    pub generated: bool,
    pub children: Vec<TreeNode>,
}

fn tree_node(doc: &Document, id: ObjectId) -> Option<TreeNode> {
    let o = doc.objects.get(&id)?;
    let n = doc.scene.node(id).ok()?;
    let role = match o {
        DomainObject::Panel(p) => Some(format!("{:?}", p.role)),
        DomainObject::Hardware(h) => Some(format!("{:?}", h.kind)),
        DomainObject::Cabinet(c) => Some(format!("{:?}", c.kind)),
        DomainObject::Room(_) => None,
    };
    Some(TreeNode {
        id,
        name: o.name().to_string(),
        kind: o.kind(),
        role,
        visible: n.visible,
        locked: n.locked,
        generated: doc.generated.contains(&id),
        children: n.children.iter().filter_map(|c| tree_node(doc, *c)).collect(),
    })
}

pub fn scene_tree(doc: &Document) -> Value {
    let roots: Vec<TreeNode> = doc.scene.roots().iter().filter_map(|r| tree_node(doc, *r)).collect();
    json!({ "name": doc.meta.name, "roots": roots })
}

#[derive(Debug, Serialize)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub value: Value,
    /// Raw source (e.g. `= cabinet.inner_width`) when the field is parametric.
    pub source: Option<String>,
    pub expression: bool,
    pub unit: Option<&'static str>,
    /// number | text | select | bool | readonly
    pub kind: &'static str,
    pub options: Vec<Value>,
    pub editable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Group {
    pub key: &'static str,
    pub title: &'static str,
    pub fields: Vec<Field>,
}

fn f(key: &str, label: &str, kind: &'static str, value: Value) -> Field {
    Field {
        key: key.into(),
        label: label.into(),
        value,
        source: None,
        expression: false,
        unit: None,
        kind,
        options: vec![],
        editable: kind != "readonly",
        error: None,
    }
}

fn param(doc: &Document, id: ObjectId, name: &str, label: &str, editable: bool) -> Option<Field> {
    let k = format!("#{}.{}", id.0, name);
    let p = doc.params.get(&k)?;
    Some(Field {
        key: name.into(),
        label: label.into(),
        value: p.value.as_ref().map(|v| json!((v * 1000.0).round() / 1000.0)).unwrap_or(Value::Null),
        source: Some(p.source.clone()),
        expression: p.is_expression(),
        unit: Some("mm"),
        kind: "number",
        options: vec![],
        editable,
        error: p.value.as_ref().err().cloned(),
    })
}

fn material_field(doc: &Document, key: &str, label: &str, current: &str) -> Field {
    let mut fl = f(key, label, "select", json!(current));
    fl.options = doc.materials.iter().map(|m| json!({ "value": m.id.0, "label": m.name, "color": m.color })).collect();
    fl
}

fn rotation(doc: &Document, id: ObjectId, editable: bool) -> Group {
    let r = doc.scene.node(id).map(|n| n.local_transform.rotation_deg).unwrap_or_default();
    let mut fields = Vec::new();
    for (i, k) in ["rx", "ry", "rz"].iter().enumerate() {
        let mut fl = f(k, &k[1..].to_uppercase(), "number", json!(r[i]));
        fl.unit = Some("deg");
        fl.editable = editable;
        fields.push(fl);
    }
    Group { key: "rotation", title: "Rotation", fields }
}

fn position(doc: &Document, id: ObjectId, editable: bool) -> Group {
    let fields = ["x", "y", "z"].iter().filter_map(|n| param(doc, id, n, &n.to_uppercase(), editable)).collect();
    Group { key: "position", title: "Position", fields }
}

pub fn properties(e: &mut Engine, id: ObjectId) -> Result<Value, CoreError> {
    let locked = e.doc.scene.is_effectively_locked(id);
    let editable = !locked;
    let obj = e.doc.object(id)?.clone();
    let doc = &e.doc;
    let mut groups: Vec<Group> = Vec::new();
    let mut general = vec![f("name", "Name", "text", json!(obj.name()))];
    let kind = obj.kind();
    match &obj {
        DomainObject::Panel(p) => {
            general.push(f("role", "Role", "readonly", json!(p.role.label())));
            groups.push(Group { key: "general", title: "General", fields: general });
            groups.push(Group {
                key: "size",
                title: "Size",
                fields: [("width", "Width"), ("height", "Height"), ("thickness", "Thickness")]
                    .iter()
                    .filter_map(|(n, l)| param(doc, id, n, l, editable))
                    .collect(),
            });
            groups.push(position(doc, id, editable));
            groups.push(rotation(doc, id, editable));
            let mut mat = vec![material_field(doc, "material", "Board", &p.material_id.0)];
            mat.push(f("grain", "Grain", "readonly", json!(format!("{:?}", p.grain_direction))));
            groups.push(Group { key: "material", title: "Material", fields: mat });
            let edges = EdgeSide::ALL
                .iter()
                .map(|s| {
                    let name = format!("{s:?}").to_lowercase();
                    let mut fl = f(&format!("edge_{name}"), &format!("{s:?}"), "bool", json!(p.edge_band(*s).is_some()));
                    fl.editable = editable;
                    fl
                })
                .collect();
            groups.push(Group { key: "edges", title: "Edges", fields: edges });
            let flat = e.flat_panel(id)?;
            let s = &flat.summary;
            groups.push(Group {
                key: "manufacturing",
                title: "Manufacturing",
                fields: vec![
                    f("drills", "Drills", "readonly", json!(s.drills + s.edge_drills)),
                    f("grooves", "Grooves", "readonly", json!(s.grooves)),
                    f("pockets", "Pockets", "readonly", json!(s.pockets)),
                    f("volume", "Volume", "readonly", json!(format!("{:.3} dm³", p.volume_mm3() / 1e6))),
                ],
            });
        }
        DomainObject::Cabinet(c) => {
            general.push(f("kind", "Type", "readonly", json!(c.kind.label())));
            groups.push(Group { key: "general", title: "General", fields: general });
            groups.push(Group {
                key: "size",
                title: "Size",
                fields: [("width", "Width"), ("height", "Height"), ("depth", "Depth")]
                    .iter()
                    .filter_map(|(n, l)| param(doc, id, n, l, editable))
                    .collect(),
            });
            let mut construction: Vec<Field> = [
                ("thickness", "Panel thickness"),
                ("back_thickness", "Back thickness"),
                ("plinth_height", "Plinth height"),
                ("door_gap", "Door gap"),
            ]
            .iter()
            .filter_map(|(n, l)| param(doc, id, n, l, editable))
            .collect();
            for (k, l) in [("top_style", "Top style"), ("bottom_style", "Bottom style")] {
                let mut fl = f(k, l, "select", json!(doc.structure_value(id, k)));
                fl.options = vec![json!({"value": "INSET", "label": "Inset"}), json!({"value": "OVERLAY", "label": "Overlay"})];
                fl.editable = editable;
                construction.push(fl);
            }
            let mut back = f("back_panel", "Back panel", "bool", json!(c.back_panel));
            back.editable = editable;
            construction.push(back);
            groups.push(Group { key: "construction", title: "Construction", fields: construction });
            let mut content = Vec::new();
            for (k, l) in [("shelves", "Shelves"), ("doors", "Doors"), ("drawers", "Drawers")] {
                let v: u32 = doc.structure_value(id, k).and_then(|s| s.parse().ok()).unwrap_or(0);
                let mut fl = f(k, l, "number", json!(v));
                fl.editable = editable;
                content.push(fl);
            }
            groups.push(Group { key: "content", title: "Content", fields: content });
            groups.push(position(doc, id, editable));
            groups.push(rotation(doc, id, editable));
            groups.push(Group {
                key: "material",
                title: "Material",
                fields: vec![
                    material_field(doc, "carcass_material", "Carcass", &c.carcass_material.0),
                    material_field(doc, "front_material", "Fronts", &c.front_material.0),
                    material_field(doc, "back_material", "Back", &c.back_material.0),
                ],
            });
            groups.push(Group {
                key: "derived",
                title: "Computed",
                fields: [("inner_width", "Inner width"), ("inner_height", "Inner height"), ("inner_depth", "Inner depth")]
                    .iter()
                    .filter_map(|(n, l)| param(doc, id, n, l, false))
                    .collect(),
            });
        }
        DomainObject::Hardware(h) => {
            general.push(f("kind", "Type", "readonly", json!(h.kind.label())));
            general.push(f("catalog", "Code", "readonly", json!(h.catalog_code)));
            groups.push(Group { key: "general", title: "General", fields: general });
            let mut size = vec![];
            if let Some(l) = param(doc, id, "length", "Length", editable) {
                size.push(l);
            }
            size.push(f("size", "Size", "readonly", json!(format!("{:.0} × {:.0} × {:.0}", h.size_mm[0], h.size_mm[1], h.size_mm[2]))));
            groups.push(Group { key: "size", title: "Size", fields: size });
            groups.push(position(doc, id, editable));
            groups.push(rotation(doc, id, editable));
        }
        DomainObject::Room(_) => {
            groups.push(Group { key: "general", title: "General", fields: general });
            groups.push(Group {
                key: "size",
                title: "Size",
                fields: [("width", "Width"), ("depth", "Depth"), ("height", "Height")]
                    .iter()
                    .filter_map(|(n, l)| param(doc, id, n, l, editable))
                    .collect(),
            });
        }
    }
    let b = e.doc.world_aabb(id);
    Ok(json!({
        "id": id,
        "kind": kind,
        "name": obj.name(),
        "locked": locked,
        "groups": groups,
        "bounds": if b.is_empty() { Value::Null } else { json!({ "min": b.min, "max": b.max }) },
    }))
}
