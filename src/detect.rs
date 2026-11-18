//! Detección del perfil del proyecto (docs/diseno.md §4.1).

use crate::contract::Profile;
use crate::fsutil::walk;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const FRAMEWORKS: &[(&str, &str)] = &[
    ("axum", "axum"),
    ("actix-web", "actix-web"),
    ("rocket", "rocket"),
    ("express", "express"),
    ("react", "react"),
    ("next", "next"),
    ("vue", "vue"),
    ("svelte", "svelte"),
    ("@nestjs/core", "nestjs"),
    ("django", "django"),
    ("flask", "flask"),
    ("fastapi", "fastapi"),
];

const ORMS: &[(&str, &str)] = &[
    ("diesel", "diesel"),
    ("sea-orm", "sea-orm"),
    ("sqlx", "sqlx"),
    ("prisma", "prisma"),
    ("@prisma/client", "prisma"),
    ("typeorm", "typeorm"),
    ("sequelize", "sequelize"),
    ("drizzle-orm", "drizzle"),
    ("sqlalchemy", "sqlalchemy"),
    ("peewee", "peewee"),
    ("tortoise-orm", "tortoise"),
];

/// Por orden de prioridad.
const BUNDLERS: &[&str] = &["vite", "webpack", "esbuild", "rollup", "parcel"];

#[derive(Default)]
struct Acc {
    languages: BTreeSet<String>,
    frameworks: BTreeSet<String>,
    orm: BTreeSet<String>,
    bundlers: BTreeSet<String>,
    iac: BTreeSet<String>,
    docs: BTreeSet<String>,
}

impl Acc {
    fn deps<I: IntoIterator<Item = String>>(&mut self, deps: I) {
        for dep in deps {
            let dep = dep.to_lowercase();
            for (name, label) in FRAMEWORKS {
                if dep == *name {
                    self.frameworks.insert((*label).into());
                }
            }
            for (name, label) in ORMS {
                if dep == *name {
                    self.orm.insert((*label).into());
                }
            }
            if BUNDLERS.contains(&dep.as_str()) {
                self.bundlers.insert(dep);
            }
        }
    }
}

pub fn detect(root: &Path) -> Profile {
    let files = walk(root);
    let mut acc = Acc::default();
    let read = |rel: &str| fs::read_to_string(root.join(rel)).unwrap_or_default();

    for rel in &files {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let lower = name.to_lowercase();
        match name {
            "Cargo.toml" => {
                acc.languages.insert("rust".into());
                acc.deps(cargo_deps(&read(rel)));
            }
            "package.json" => {
                acc.languages.insert("javascript".into());
                acc.deps(package_json_deps(&read(rel)));
            }
            "pyproject.toml" | "requirements.txt" | "Pipfile" | "setup.py" => {
                acc.languages.insert("python".into());
                acc.deps(python_deps(&read(rel)));
            }
            "go.mod" => {
                acc.languages.insert("go".into());
            }
            "pom.xml" | "build.gradle" | "build.gradle.kts" => {
                acc.languages.insert("java".into());
            }
            "Chart.yaml" => {
                acc.iac.insert("helm".into());
            }
            "kustomization.yaml" | "kustomization.yml" => {
                acc.iac.insert("kustomize".into());
            }
            _ => {}
        }
        if lower.starts_with("vite.config.") {
            acc.bundlers.insert("vite".into());
        } else if lower.starts_with("webpack.config.") {
            acc.bundlers.insert("webpack".into());
        } else if lower.starts_with("rollup.config.") {
            acc.bundlers.insert("rollup".into());
        }
        if lower == "tsconfig.json" || lower.ends_with(".ts") || lower.ends_with(".tsx") {
            acc.languages.insert("typescript".into());
        }
        if lower.ends_with(".tf") {
            acc.iac.insert("terraform".into());
        }
        if (lower.ends_with(".yaml") || lower.ends_with(".yml"))
            && name != "Chart.yaml"
            && is_k8s_manifest(&read(rel))
        {
            acc.iac.insert("kubernetes".into());
        }
        if lower.ends_with(".md") {
            if !rel.contains('/') && lower.starts_with("readme") {
                acc.docs.insert("readme".into());
            }
            if rel.starts_with("docs/") {
                acc.docs.insert("docs".into());
            }
            if rel.split('/').any(|c| c == "adr") {
                acc.docs.insert("adr".into());
            }
        }
    }

    let bundler = BUNDLERS
        .iter()
        .find(|b| acc.bundlers.contains(**b))
        .map(|b| (*b).to_string());
    let v = |s: BTreeSet<String>| s.into_iter().collect::<Vec<_>>();
    Profile {
        languages: v(acc.languages),
        frameworks: v(acc.frameworks),
        orm: v(acc.orm),
        bundler,
        iac: v(acc.iac),
        docs: v(acc.docs),
    }
}

fn is_k8s_manifest(text: &str) -> bool {
    text.len() < 1_000_000
        && text.lines().any(|l| l.starts_with("apiVersion:"))
        && text.lines().any(|l| l.starts_with("kind:"))
}

fn package_json_deps(text: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .filter_map(|k| v.get(*k).and_then(|d| d.as_object()))
        .flat_map(|o| o.keys().cloned())
        .collect()
}

fn cargo_deps(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_deps = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(h) = line.strip_prefix('[') {
            let h = h.trim_end_matches(']').trim_start_matches('[');
            in_deps = h.ends_with("dependencies");
            if let Some((_, name)) = h.split_once("dependencies.") {
                out.push(name.trim().to_string());
            }
        } else if in_deps && !line.starts_with('#') {
            let key: String = line
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !key.is_empty() {
                out.push(key);
            }
        }
    }
    out
}

fn python_deps(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

/// Tabla de texto para `--compact`.
pub fn compact(p: &Profile) -> String {
    let row =
        |k: &str, v: String| format!("{k:<11}{}\n", if v.is_empty() { "-".into() } else { v });
    let mut s = String::new();
    s += &row("languages", p.languages.join(", "));
    s += &row("frameworks", p.frameworks.join(", "));
    s += &row("orm", p.orm.join(", "));
    s += &row("bundler", p.bundler.clone().unwrap_or_default());
    s += &row("iac", p.iac.join(", "));
    s += &row("docs", p.docs.join(", "));
    s
}
