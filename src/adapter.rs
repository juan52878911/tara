//! Trait `Adapter` (docs/diseno.md §4.4) y ejecución con degradación.

use crate::contract::{Coverage, Finding, Profile, Status};
use crate::fsutil::which;
use anyhow::Result;
use std::path::PathBuf;

/// Binario externo que un adaptador necesita.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalTool(pub &'static str);

pub struct Ctx {
    pub root: PathBuf,
    pub sha: String,
    pub profile: Profile,
}

pub trait Adapter {
    fn id(&self) -> &'static str;
    /// Dimensión que cubre (aparece en `coverage`).
    fn dimension(&self) -> &'static str;
    fn applies(&self, profile: &Profile) -> bool;
    fn requires(&self) -> &[ExternalTool];
    fn run(&self, ctx: &Ctx) -> Result<Vec<Finding>>;
}

/// Ejecuta los adaptadores aplicables. Si falta un binario o falla, lo declara en `coverage`.
pub fn run_all(adapters: &[Box<dyn Adapter>], ctx: &Ctx) -> (Vec<Coverage>, Vec<Finding>) {
    let mut coverage = Vec::new();
    let mut findings = Vec::new();
    for a in adapters {
        if !a.applies(&ctx.profile) {
            continue;
        }
        let mut cov = Coverage {
            dimension: a.dimension().into(),
            adapter: a.id().into(),
            status: Status::Ok,
            reason: None,
        };
        let missing: Vec<&str> = a
            .requires()
            .iter()
            .map(|t| t.0)
            .filter(|b| !which(b))
            .collect();
        if !missing.is_empty() {
            cov.status = Status::Skipped;
            cov.reason = Some(format!("{} no instalado", missing.join(", ")));
        } else {
            match a.run(ctx) {
                Ok(f) => findings.extend(f),
                Err(e) => {
                    cov.status = Status::Skipped;
                    cov.reason = Some(format!("error: {e}"));
                }
            }
        }
        coverage.push(cov);
    }
    coverage.sort_by(|a, b| (&a.dimension, &a.adapter).cmp(&(&b.dimension, &b.adapter)));
    findings.sort_by(|a, b| a.id.cmp(&b.id));
    (coverage, findings)
}
