mod common;
use std::path::Path;
use tara::detect::detect;

fn expected(name: &str) -> serde_json::Value {
    let p = Path::new("tests/fixtures")
        .join(name)
        .join("expected-profile.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

#[test]
fn perfiles_de_fixtures() {
    for name in ["rust-min", "node-tf", "python-k8s"] {
        let got = serde_json::to_value(detect(&Path::new("tests/fixtures").join(name))).unwrap();
        assert_eq!(got, expected(name), "fixture {name}");
    }
}

#[test]
fn cli_detect_json_y_compact() {
    let out = common::tara()
        .args(["detect", "tests/fixtures/node-tf"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v, expected("node-tf"));

    let out = common::tara()
        .args(["detect", "tests/fixtures/node-tf", "--compact"])
        .output()
        .unwrap();
    let t = String::from_utf8(out.stdout).unwrap();
    assert!(t.contains("frameworks") && t.contains("express, react") && t.contains("vite"));
    assert!(!t.trim_start().starts_with('{'));
}
