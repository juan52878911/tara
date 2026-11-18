mod common;
use serde_json::Value;
use tara::contract::Report;

fn check(schema: &Value, defs: &Value, data: &Value, path: &str) {
    if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
        let name = r.rsplit('/').next().unwrap();
        return check(&defs[name], defs, data, path);
    }
    if let Some(req) = schema.get("required").and_then(Value::as_array) {
        for k in req {
            let k = k.as_str().unwrap();
            assert!(data.get(k).is_some(), "falta `{k}` en {path}");
        }
    }
    if let (Some(props), Some(obj)) = (
        schema.get("properties").and_then(Value::as_object),
        data.as_object(),
    ) {
        for (k, sub) in props {
            if let Some(v) = obj.get(k) {
                check(sub, defs, v, &format!("{path}.{k}"));
            }
        }
    }
    if let (Some(items), Some(arr)) = (schema.get("items"), data.as_array()) {
        for (i, v) in arr.iter().enumerate() {
            check(items, defs, v, &format!("{path}[{i}]"));
        }
    }
    if let Some(e) = schema.get("enum").and_then(Value::as_array) {
        assert!(e.contains(data), "valor {data} fuera de enum en {path}");
    }
}

#[test]
fn salida_de_scan_cumple_el_schema() {
    let d = common::repo();
    let out = common::tara().arg("scan").arg(d.path()).output().unwrap();
    assert!(out.status.success());
    let data: Value = serde_json::from_slice(&out.stdout).unwrap();
    let schema: Value =
        serde_json::from_str(&std::fs::read_to_string("schema/tara.v1.json").unwrap()).unwrap();
    assert!(!data["findings"].as_array().unwrap().is_empty());
    check(&schema, &schema["$defs"], &data, "$");
}

#[test]
fn ida_y_vuelta() {
    let d = common::repo();
    let out = common::tara().arg("scan").arg(d.path()).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let r: Report = serde_json::from_str(&text).unwrap();
    assert_eq!(r.schema_version, 1);
    let again: Report = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(r, again);
    let v1: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v1, serde_json::to_value(&r).unwrap());
}
