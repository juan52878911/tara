mod common;
use common::{git, repo, tara};

#[test]
fn scan_escribe_linea_base_ordenada_y_reproducible() {
    let d = repo();
    let sha = git(d.path(), &["rev-parse", "HEAD"]);
    tara().arg("scan").arg(d.path()).assert().success();
    let dir = d.path().join(".tara/baselines");
    assert_eq!(
        std::fs::read_to_string(dir.join("latest")).unwrap().trim(),
        sha
    );
    let f1 = std::fs::read(dir.join(format!("{sha}.json"))).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&f1).unwrap();
    let ids: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
    assert_eq!(ids, ["code_size:src/a.rs", "code_size:src/b.py"]);

    std::fs::remove_dir_all(&dir).unwrap();
    tara().arg("scan").arg(d.path()).assert().success();
    let f2 = std::fs::read(dir.join(format!("{sha}.json"))).unwrap();
    assert_eq!(f1, f2);
}

#[test]
fn compare_umbral_y_codigos_de_salida() {
    let d = repo();
    tara().arg("scan").arg(d.path()).assert().success();
    // Sin cambios: 0.
    tara()
        .args(["compare", "HEAD"])
        .arg(d.path())
        .assert()
        .code(0);

    // Segundo commit: a.rs pasa de 3 a 4 líneas (+33 %).
    std::fs::write(d.path().join("src/a.rs"), "a\nb\nc\nd\n").unwrap();
    git(d.path(), &["commit", "-qam", "dos"]);
    // Sin línea base del nuevo commit no hace falta; comparamos contra HEAD~1.
    let out = tara()
        .args(["compare", "HEAD~1"])
        .arg(d.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let a = v["deltas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["location"]["path"] == "src/a.rs")
        .unwrap();
    assert_eq!(
        (a["old"].as_f64(), a["new"].as_f64(), a["delta"].as_f64()),
        (Some(3.0), Some(4.0), Some(1.0))
    );
    assert!((a["pct"].as_f64().unwrap() - 33.333).abs() < 0.01);

    // Umbral mayor que la subida: 0.
    tara()
        .args(["compare", "HEAD~1", "--threshold-pct", "50"])
        .arg(d.path())
        .assert()
        .code(0);

    // --compact.
    let out = tara()
        .args(["compare", "HEAD~1", "--compact"])
        .arg(d.path())
        .output()
        .unwrap();
    let t = String::from_utf8(out.stdout).unwrap();
    assert!(t.contains("src/a.rs") && t.contains("+33.3%") && t.contains("REGRESION"));
}

#[test]
fn compare_sin_linea_base_sale_con_2() {
    let d = repo();
    tara()
        .args(["compare", "HEAD"])
        .arg(d.path())
        .assert()
        .code(2);
    tara()
        .args(["compare", "latest"])
        .arg(d.path())
        .assert()
        .code(2);
}
