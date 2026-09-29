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
    /// Room (phòng) of a cabinet; empty = no room.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor: Option<String>,
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
        room: o.as_cabinet().map(|c| c.room.clone()),
        floor: o.as_cabinet().map(|c| c.floor.clone()),
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
    /// The locked one of a set of alternative values (red dot in the UI).
    pub locked: bool,
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
        locked: false,
    }
}

fn opts(items: &[(&str, &str)]) -> Vec<Value> {
    items.iter().map(|(v, l)| json!({ "value": v, "label": l })).collect()
}

fn sel(key: &str, label: &str, value: &str, items: &[(&str, &str)], editable: bool) -> Field {
    let mut fl = f(key, label, "select", json!(value));
    fl.options = opts(items);
    fl.editable = editable;
    fl
}

fn numf(key: &str, label: &str, value: f64, editable: bool) -> Field {
    let mut fl = f(key, label, "number", json!((value * 10.0).round() / 10.0));
    fl.unit = Some("mm");
    fl.editable = editable;
    fl
}

fn up(v: impl std::fmt::Debug) -> String {
    let s = format!("{v:?}");
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    out
}

/// Chỉnh tấm groups for generated parts (position lock, door/drawer config, co giãn, tools, links).
fn generated_groups(e: &mut Engine, id: ObjectId, editable: bool, groups: &mut Vec<Group>) -> Result<(), CoreError> {
    use crate::zones::{parse_key, PartRef};
    use aic_domain::zone::{Front, Lock};
    let Some((cab, key)) = e.part_ref(id) else { return Ok(()) };
    let def = e.cabinet_def(cab)?;
    let layout = e.doc.cabinet_layout(cab).ok_or(CoreError::NotFound { id: cab })?;
    match parse_key(&key) {
        PartRef::Split(uid) => {
            if let (Some(sp), Some(pos)) = (def.zones.panel(uid), layout.positions.iter().find(|p| p.uid == uid)) {
                let (a, b) = match pos.axis {
                    0 => ("Cách trái", "Cách phải"),
                    1 => ("Cách dưới", "Cách trên"),
                    _ => ("Cách sau", "Cách trước"),
                };
                let mut r = f("pos_ratio", "Tỷ lệ (%)", "number", json!((pos.ratio * 1000.0).round() / 10.0));
                r.editable = editable;
                r.locked = matches!(pos.lock, Lock::Ratio | Lock::Even);
                let mut s1 = numf("pos_start", a, pos.from_start, editable);
                s1.locked = pos.lock == Lock::FromStart;
                let mut s2 = numf("pos_end", b, pos.from_end, editable);
                s2.locked = pos.lock == Lock::FromEnd;
                let mut cells = f("cells", "Lọt lòng mỗi ô", "readonly", json!(format!("{:.1} / {:.1}", pos.cell_before, pos.cell_after)));
                cells.unit = Some("mm");
                let kinds: &[(&str, &str)] = match sp.kind.axis() {
                    1 => &[("SHELF_ADJUSTABLE", "KệDiĐộng"), ("SHELF_FIXED", "KệCốĐịnh")],
                    0 => &[("DIVIDER", "HôngGiữa")],
                    _ => &[("BACK_SUB", "HậuPhụ")],
                };
                groups.push(Group {
                    key: "zone_position",
                    title: "Position",
                    fields: vec![sel("split_kind", "Loại", &up(sp.kind), kinds, editable), r, s1, s2, cells],
                });
            }
        }
        PartRef::Door(uid) => {
            if let Some(Front::Doors(d)) = def.zones.zone_of_attachment(uid).and_then(|z| def.zones.zone(z)).and_then(|z| z.front.as_ref()) {
                let mut fields = vec![
                    sel("door_kind", "Kiểu cửa", &up(d.kind), &[("SINGLE", "Đơn"), ("DOUBLE", "Đôi"), ("SLIDING", "Lùa")], editable),
                    sel("door_mount", "Kiểu kết cấu", &up(d.mount), &[("OVERLAY", "Phủ bì"), ("INSET", "Lọt lòng")], editable),
                    sel("door_hinge", "Lắp lề", &up(d.hinge), &[("LEFT", "Trái"), ("RIGHT", "Phải"), ("TOP", "Trên"), ("BOTTOM", "Dưới")], editable),
                    {
                        let mut x = f("door_cols", "Số cánh ngang", "number", json!(d.cols));
                        x.editable = editable;
                        x
                    },
                    {
                        let mut x = f("door_rows", "Số cánh dọc", "number", json!(d.rows));
                        x.editable = editable;
                        x
                    },
                    sel("door_stop", "Thanh chặn", &up(d.stop.kind), &[("NONE", "Không"), ("L_SHAPE", "Chữ L"), ("STRAIGHT", "Thẳng")], editable),
                ];
                if d.stop.kind != aic_domain::zone::StopRail::None {
                    fields.push(numf("door_stop_height", "Cao vùng", d.stop.height, editable));
                    fields.push(numf("door_stop_cover", "Cửa phủ lên", d.stop.cover_up, editable));
                    fields.push(numf("door_stop_leg", "Sâu chân", d.stop.leg_depth, editable));
                    fields.push(numf("door_stop_setback", "Lùi thanh", d.stop.setback, editable));
                }
                groups.push(Group { key: "door", title: "Door", fields });
            }
        }
        PartRef::Drawer(uid) => {
            if let Some(Front::Drawers(d)) = def.zones.zone_of_attachment(uid).and_then(|z| def.zones.zone(z)).and_then(|z| z.front.as_ref()) {
                let mut cnt = f("drawer_count", "Số ngăn", "number", json!(d.count));
                cnt.editable = editable;
                let mut cols = f("drawer_cols", "Số cột", "number", json!(d.cols));
                cols.editable = editable;
                let mut bx = f("drawer_box", "Tạo hộc kéo", "bool", json!(d.with_box));
                bx.editable = editable;
                groups.push(Group {
                    key: "drawer",
                    title: "Drawer",
                    fields: vec![
                        cnt,
                        cols,
                        sel("drawer_type", "Loại ngăn kéo", "RAYBI_17", &[("RAYBI_17", "01. RayBi-Ván17mm")], false),
                        sel("drawer_mount", "Kiểu mặt", &up(d.mount), &[("OVERLAY", "Phủ bì"), ("INSET", "Lọt lòng")], editable),
                        numf("drawer_face_thickness", "Dày mặt", d.face_thickness.unwrap_or(e.doc.param_value(cab, "door_thickness").unwrap_or(17.2)), editable),
                        numf("drawer_side_gap", "Hở hông", d.side_gap, editable),
                        numf("drawer_gap", "Khe giữa 2 ngăn", d.gap, editable),
                        bx,
                    ],
                });
            }
        }
        PartRef::Link(_) => {
            if let Some(l) = def.zones.zones().iter().flat_map(|z| z.links.iter()).find(|l| key == format!("l:{}", l.uid)) {
                groups.push(Group { key: "link", title: "Link", fields: vec![numf("link_offset", "Cách nóc vùng", l.offset, editable)] });
            }
        }
        PartRef::Carcass => {}
    }
    let m = def.mods.get(&key).cloned().unwrap_or_default();
    if matches!(e.doc.objects.get(&id), Some(DomainObject::Panel(_))) {
        groups.push(Group {
            key: "stretch",
            title: "Stretch",
            fields: vec![
                numf("ext_left", "Giãn trái", m.extend[0], editable),
                numf("ext_right", "Giãn phải", m.extend[1], editable),
                numf("ext_bottom", "Giãn dưới", m.extend[2], editable),
                numf("ext_top", "Giãn trên", m.extend[3], editable),
            ],
        });
        // Offset (lùi) of each face in the cabinet frame: stored as parameters.
        groups.push(Group {
            key: "offset",
            title: "Offset",
            fields: ["off_front", "off_back", "off_left", "off_right", "off_top", "off_bottom"]
                .iter()
                .map(|k| {
                    let i = OFFSET_KEYS.iter().position(|x| x == k).unwrap();
                    numf(k, k, m.offsets[i], editable)
                })
                .collect(),
        });
        // Liên kết (x/y): neighbours touching this part.
        let rel = e.relations();
        let names: Vec<Value> = rel
            .relations_of(id)
            .iter()
            .filter(|r| r.contact == aic_assembly::ContactType::Touch)
            .filter_map(|r| e.doc.objects.get(&r.target).map(|o| json!({ "id": r.target, "name": o.name(), "region": r.source_region })))
            .collect();
        let mut links = f("links", "Liên kết", "list", Value::Array(names));
        links.editable = false;
        let mut tools = f("tools", "Tool đã áp", "list", json!(m.tools));
        tools.editable = editable;
        groups.push(Group { key: "relations", title: "Links", fields: vec![links, tools] });
    }
    Ok(())
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
        locked: false,
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

/// Multi-selection sheet: groups / fields present on every object; a field whose
/// values differ is `mixed` (value null). List fields are left out.
pub fn properties_multi(e: &mut Engine, ids: &[ObjectId]) -> Result<Value, CoreError> {
    let sheets: Vec<Value> = ids.iter().map(|id| properties(e, *id)).collect::<Result<_, _>>()?;
    let Some(first) = sheets.first() else { return Ok(json!({ "kind": "MULTI", "ids": ids, "groups": [] })) };
    let field = |sheet: &Value, g: &str, k: &str| -> Option<Value> {
        sheet["groups"].as_array()?.iter().find(|x| x["key"] == g)?["fields"].as_array()?.iter().find(|f| f["key"] == k).cloned()
    };
    let mut groups = Vec::new();
    for g in first["groups"].as_array().cloned().unwrap_or_default() {
        let gk = g["key"].as_str().unwrap_or_default().to_string();
        let mut fields = Vec::new();
        for f in g["fields"].as_array().cloned().unwrap_or_default() {
            let k = f["key"].as_str().unwrap_or_default();
            if f["kind"] == "list" || k == "name" {
                continue;
            }
            let others: Option<Vec<Value>> = sheets[1..].iter().map(|s| field(s, &gk, k)).collect();
            let Some(others) = others else { continue };
            let mut out = f.clone();
            let same = others.iter().all(|o| o["value"] == f["value"]);
            if !same {
                out["value"] = Value::Null;
                out["mixed"] = json!(true);
            }
            let editable = f["editable"].as_bool().unwrap_or(false) && others.iter().all(|o| o["editable"].as_bool().unwrap_or(false));
            out["editable"] = json!(editable);
            out["expression"] = json!(false);
            fields.push(out);
        }
        if !fields.is_empty() {
            groups.push(json!({ "key": gk, "title": g["title"], "fields": fields }));
        }
    }
    let kinds: std::collections::BTreeSet<String> = sheets.iter().filter_map(|s| s["kind"].as_str().map(str::to_string)).collect();
    Ok(json!({
        "id": ids[0],
        "ids": ids,
        "kind": if kinds.len() == 1 { kinds.into_iter().next().unwrap() } else { "MULTI".into() },
        "name": format!("{} đối tượng", ids.len()),
        "locked": sheets.iter().any(|s| s["locked"] == true),
        "groups": groups,
        "bounds": Value::Null,
    }))
}

/// Offset parameter keys in `PartMod::offsets` order.
pub const OFFSET_KEYS: [&str; 6] = ["off_left", "off_right", "off_bottom", "off_top", "off_back", "off_front"];

pub fn properties(e: &mut Engine, id: ObjectId) -> Result<Value, CoreError> {
    let locked = e.doc.scene.is_effectively_locked(id);
    let editable = !locked;
    let obj = e.doc.object(id)?.clone();
    let mut gen_groups = Vec::new();
    generated_groups(e, id, editable, &mut gen_groups)?;
    let effective_edges = e.effective_edges(id);
    let doc = &e.doc;
    let mut groups: Vec<Group> = Vec::new();
    let mut general = vec![f("name", "Name", "text", json!(obj.name()))];
    let kind = obj.kind();
    match &obj {
        DomainObject::Panel(p) => {
            let generated = p.gen_key.is_some() && doc.generated.contains(&id);
            general.push(f("role", "Role", "readonly", json!(format!("{:?}", p.role))));
            if let Some(cab) = doc.cabinet_of(id).and_then(|c| doc.objects.get(&c)).and_then(|o| o.as_cabinet()) {
                let prefix = if cab.room.is_empty() { format!("[{}]", cab.name) } else { format!("[{} - {}]", cab.room, cab.name) };
                general.push(f("full_name", "Full name", "readonly", json!(format!("{prefix} {}", p.name))));
            }
            groups.push(Group { key: "general", title: "General", fields: general });
            // Generated parts: length/width are computed (grey), thickness is editable.
            groups.push(Group {
                key: "size",
                title: "Size",
                fields: [("width", "Width"), ("height", "Height"), ("thickness", "Thickness")]
                    .iter()
                    .filter_map(|(n, l)| param(doc, id, n, l, editable && (!generated || *n == "thickness")))
                    .collect(),
            });
            if generated {
                groups.append(&mut gen_groups);
            } else {
                groups.push(position(doc, id, editable));
                groups.push(rotation(doc, id, editable));
            }
            let mut mat = vec![material_field(doc, "material", "Board", &p.material_id.0)];
            mat.push(f("grain", "Grain", "readonly", json!(format!("{:?}", p.grain_direction))));
            groups.push(Group { key: "material", title: "Material", fields: mat });
            let edges = EdgeSide::ALL
                .iter()
                .map(|s| {
                    let name = format!("{s:?}").to_lowercase();
                    let mut fl = f(&format!("edge_{name}"), &format!("{s:?}"), "bool", json!(effective_edges.iter().any(|b| b.edge == *s)));
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
            general.push(f("kind", "Type", "readonly", json!(format!("{:?}", c.kind))));
            let mut room = f("room", "Room", "text", json!(c.room));
            room.editable = editable;
            general.push(room);
            let mut floor = f("floor", "Floor", "text", json!(c.floor));
            floor.editable = editable;
            general.push(floor);
            groups.push(Group { key: "general", title: "General", fields: general });
            groups.push(Group {
                key: "size",
                title: "Size",
                fields: [("width", "Width"), ("height", "Height"), ("depth", "Depth")]
                    .iter()
                    .filter_map(|(n, l)| param(doc, id, n, l, editable))
                    .chain([("anchor_w", "Width anchor", c.anchors.width), ("anchor_h", "Height anchor", c.anchors.height), ("anchor_d", "Depth anchor", c.anchors.depth)].into_iter().map(|(k, l, a)| {
                        let mut fl = f(k, l, "select", json!(format!("{a:?}").to_uppercase()));
                        let names: [&str; 3] = match k {
                            "anchor_w" => ["KEEP_LEFT", "KEEP_CENTER", "KEEP_RIGHT"],
                            "anchor_h" => ["KEEP_BOTTOM", "KEEP_CENTER", "KEEP_TOP"],
                            _ => ["KEEP_BACK", "KEEP_CENTER", "KEEP_FRONT"],
                        };
                        fl.options = ["START", "CENTER", "END"].iter().zip(names).map(|(v, n)| json!({"value": v, "label": n})).collect();
                        fl.editable = editable;
                        fl
                    }))
                    .collect(),
            });
            let mut construction: Vec<Field> = [
                ("thickness", "Panel thickness"),
                ("back_thickness", "Back thickness"),
                ("plinth_height", "Plinth height"),
                ("door_gap", "Door gap"),
                ("door_thickness", "Door thickness"),
                ("shelf_setback", "Shelf setback"),
                ("back_groove", "Back groove"),
                ("back_offset", "Back offset"),
                ("rail_width", "Rail width"),
            ]
            .iter()
            .filter_map(|(n, l)| param(doc, id, n, l, editable))
            .collect();
            for (k, l) in [("top_style", "Top style"), ("bottom_style", "Bottom style")] {
                let mut fl = f(k, l, "select", json!(doc.structure_value(id, k)));
                fl.options = vec![json!({"value": "INSET", "label": "Inset"}), json!({"value": "OVERLAY", "label": "Overlay"})];
                if k == "top_style" {
                    fl.options.push(json!({"value": "RAILS", "label": "Rails"}));
                }
                fl.editable = editable;
                construction.push(fl);
            }
            let mut back = f("back_panel", "Back panel", "bool", json!(c.back_panel));
            back.editable = editable;
            construction.push(back);
            let mut handles = f("handles", "Handles", "bool", json!(c.handles));
            handles.editable = editable;
            construction.push(handles);
            groups.push(Group { key: "construction", title: "Construction", fields: construction });
            let mut content = Vec::new();
            for (k, l) in [("shelves", "Shelves"), ("doors", "Doors"), ("drawers", "Drawers")] {
                let v: u32 = doc.structure_value(id, k).and_then(|s| s.parse().ok()).unwrap_or(0);
                let mut fl = f(k, l, "number", json!(v));
                fl.editable = editable;
                content.push(fl);
            }
            groups.push(Group { key: "content", title: "Content", fields: content });
            let r = &c.edge_rule;
            let mut edge = vec![
                sel("edge_mode", "Kiểu dán", &up(r.mode), &[("EXPOSED_ONLY", "Dán hở bỏ khuất"), ("ALL", "Dán toàn bộ"), ("NONE", "Không dán")], editable),
                sel("edge_band", "Loại chỉ dán", &r.band_code, &[("DON-0.5", "Đơn 0.5mm"), ("DON-1", "Đơn 1mm"), ("DON-2", "Đơn 2mm"), ("KEP-1", "Kép 1mm")], editable),
                numf("edge_threshold", "Ngưỡng dán cạnh", r.threshold, editable),
                numf("edge_min_length", "Bỏ cạnh ngắn ≤", r.min_length, editable),
            ];
            let mut skip = f("edge_skip", "Độ dày không dán", "text", json!(r.skip_thicknesses.iter().map(|t| format!("{t}")).collect::<Vec<_>>().join(", ")));
            skip.editable = editable;
            edge.push(skip);
            groups.push(Group { key: "edge_rule", title: "Edge rule", fields: edge });
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
