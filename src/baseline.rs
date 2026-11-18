//! `scan` (línea base por commit) y `compare`.

use crate::adapter::{run_all, Adapter, Ctx};
use crate::contract::{Location, Report, SCHEMA_VERSION};
use crate::detect::detect;
use crate::fsutil::now_rfc3339;
use crate::loc::Loc;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn default_adapters() -> Vec<Box<dyn Adapter>> {
    vec![Box::new(Loc)]
}

/// Resuelve una ref a sha completo con git.
pub fn rev_parse(root: &Path, rev: &str) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{rev}^{{commit}}"))
        .output()
        .context("no se pudo ejecutar git")?;
    if !out.status.success() {
        bail!("no se pudo resolver `{rev}` en {}", root.display());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn build_report(root: &Path, adapters: &[Box<dyn Adapter>]) -> Result<Report> {
    let sha = rev_parse(root, "HEAD")?;
    let profile = detect(root);
    let ctx = Ctx {
        root: root.to_path_buf(),
        sha: sha.clone(),
        profile: profile.clone(),
    };
    let (coverage, findings) = run_all(adapters, &ctx);
    Ok(Report {
        schema_version: SCHEMA_VERSION,
        tool: "tara".into(),
        commit: sha,
        generated_at: now_rfc3339(),
        profile,
        coverage,
        findings,
    })
}

/// Genera el informe y escribe `.tara/baselines/<sha>.json` y `latest`.
pub fn scan(root: &Path, adapters: &[Box<dyn Adapter>]) -> Result<Report> {
    let report = build_report(root, adapters)?;
    let dir = root.join(".tara").join("baselines");
    fs::create_dir_all(&dir)?;
    let mut json = serde_json::to_string_pretty(&report)?;
    json.push('\n');
    fs::write(dir.join(format!("{}.json", report.commit)), json)?;
    fs::write(dir.join("latest"), format!("{}\n", report.commit))?;
    Ok(report)
}

/// Lee una línea base por sha, ref o `latest`. `None` si no existe.
pub fn load_baseline(root: &Path, rev: &str) -> Result<Option<Report>> {
    let dir = root.join(".tara").join("baselines");
    let sha = if rev == "latest" {
        match fs::read_to_string(dir.join("latest")) {
            Ok(s) => s.trim().to_string(),
            Err(_) => return Ok(None),
        }
    } else {
        rev_parse(root, rev)?
    };
    match fs::read_to_string(dir.join(format!("{sha}.json"))) {
        Ok(s) => Ok(Some(
            serde_json::from_str(&s).context("línea base ilegible")?,
        )),
        Err(_) => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Delta {
    pub dimension: String,
    pub metric: String,
    pub location: Location,
    pub old: Option<f64>,
    pub new: Option<f64>,
    pub delta: Option<f64>,
    /// `None` si no hay valor previo o era 0.
    pub pct: Option<f64>,
    /// `unchanged | up | down | added | removed`
    pub change: String,
    pub regression: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Comparison {
    pub base: String,
    pub head: String,
    pub threshold_pct: f64,
    pub regression: bool,
    pub deltas: Vec<Delta>,
}

pub fn compare_reports(base: &Report, head: &Report, threshold_pct: f64) -> Comparison {
    type Key = (String, String, Location);
    let key = |f: &crate::contract::Finding| -> Key {
        (f.dimension.clone(), f.metric.clone(), f.location.clone())
    };
    let old: BTreeMap<Key, f64> = base.findings.iter().map(|f| (key(f), f.value)).collect();
    let new: BTreeMap<Key, f64> = head.findings.iter().map(|f| (key(f), f.value)).collect();
    let mut keys: Vec<&Key> = old.keys().chain(new.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut deltas = Vec::new();
    for k in keys {
        let (o, n) = (old.get(k).copied(), new.get(k).copied());
        let (delta, pct, change, regression) = match (o, n) {
            (Some(o), Some(n)) => {
                let d = n - o;
                let pct = if o != 0.0 { Some(d / o * 100.0) } else { None };
                let up_over = d > 0.0 && pct.is_none_or(|p| p > threshold_pct);
                let change = if d > 0.0 {
                    "up"
                } else if d < 0.0 {
                    "down"
                } else {
                    "unchanged"
                };
                (Some(d), pct, change, up_over)
            }
            (None, Some(_)) => (None, None, "added", false),
            (Some(_), None) => (None, None, "removed", false),
            (None, None) => unreachable!(),
        };
        deltas.push(Delta {
            dimension: k.0.clone(),
            metric: k.1.clone(),
            location: k.2.clone(),
            old: o,
            new: n,
            delta,
            pct,
            change: change.into(),
            regression,
        });
    }
    Comparison {
        base: base.commit.clone(),
        head: head.commit.clone(),
        threshold_pct,
        regression: deltas.iter().any(|d| d.regression),
        deltas,
    }
}

fn num(v: Option<f64>) -> String {
    v.map_or("-".into(), |v| format!("{v}"))
}

/// Tabla de texto para `--compact`.
pub fn compact(c: &Comparison) -> String {
    let mut s = format!(
        "base {} -> head {} (umbral {}%)\n",
        &c.base[..c.base.len().min(12)],
        &c.head[..c.head.len().min(12)],
        c.threshold_pct
    );
    s += "dimension\tmetric\tlocation\told\tnew\tdelta\tpct\tchange\n";
    for d in &c.deltas {
        let pct = d.pct.map_or("-".into(), |p| format!("{p:+.1}%"));
        let flag = if d.regression { " !" } else { "" };
        s += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}{}\n",
            d.dimension,
            d.metric,
            d.location.path,
            num(d.old),
            num(d.new),
            num(d.delta),
            pct,
            d.change,
            flag
        );
    }
    s += if c.regression { "REGRESION\n" } else { "OK\n" };
    s
}
