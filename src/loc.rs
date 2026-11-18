//! Adaptador interno `loc`: líneas por fichero de código.

use crate::adapter::{Adapter, Ctx, ExternalTool};
use crate::contract::{Anchor, AnchorClass, Confidence, Finding, Location, Method, Profile};
use crate::fsutil::walk;
use anyhow::Result;
use std::fs;

const CODE_EXTS: &[&str] = &[
    "rs", "js", "jsx", "mjs", "cjs", "ts", "tsx", "py", "go", "java", "kt", "c", "h", "cpp", "rb",
];

pub struct Loc;

impl Adapter for Loc {
    fn id(&self) -> &'static str {
        "loc"
    }
    fn dimension(&self) -> &'static str {
        "code_size"
    }
    fn applies(&self, _: &Profile) -> bool {
        true
    }
    fn requires(&self) -> &[ExternalTool] {
        &[]
    }
    fn run(&self, ctx: &Ctx) -> Result<Vec<Finding>> {
        let mut out = Vec::new();
        for rel in walk(&ctx.root) {
            let ext = rel.rsplit('.').next().unwrap_or("");
            if !rel.contains('.') || !CODE_EXTS.contains(&ext) {
                continue;
            }
            let Ok(bytes) = fs::read(ctx.root.join(&rel)) else {
                continue;
            };
            let lines = String::from_utf8_lossy(&bytes).lines().count();
            out.push(Finding {
                id: format!("code_size:{rel}"),
                dimension: "code_size".into(),
                metric: "lines".into(),
                value: lines as f64,
                unit: "lines".into(),
                method: Method::Static,
                confidence: Confidence::High,
                location: Location {
                    path: rel,
                    line: None,
                    symbol: None,
                },
                anchored_at: Anchor {
                    sha: ctx.sha.clone(),
                    class: AnchorClass::Deterministic,
                },
                budget: None,
                hotspot_score: None,
                evidence: None,
            });
        }
        Ok(out)
    }
}
