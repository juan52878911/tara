use anyhow::Result;
use tara::adapter::{run_all, Adapter, Ctx, ExternalTool};
use tara::contract::{Finding, Profile, Status};
use tara::loc::Loc;

struct Falso;
impl Adapter for Falso {
    fn id(&self) -> &'static str {
        "falso"
    }
    fn dimension(&self) -> &'static str {
        "cpu"
    }
    fn applies(&self, _: &Profile) -> bool {
        true
    }
    fn requires(&self) -> &[ExternalTool] {
        &[ExternalTool("tara-binario-que-no-existe")]
    }
    fn run(&self, _: &Ctx) -> Result<Vec<Finding>> {
        panic!("no debe ejecutarse");
    }
}

#[test]
fn adaptador_sin_binario_se_salta() {
    let ctx = Ctx {
        root: ".".into(),
        sha: "abc".into(),
        profile: Profile::default(),
    };
    let adapters: Vec<Box<dyn Adapter>> = vec![Box::new(Falso)];
    let (cov, f) = run_all(&adapters, &ctx);
    assert!(f.is_empty());
    assert_eq!(cov.len(), 1);
    assert_eq!(cov[0].status, Status::Skipped);
    assert!(cov[0]
        .reason
        .as_deref()
        .unwrap()
        .contains("tara-binario-que-no-existe"));
}

#[test]
fn loc_un_hallazgo_por_fichero() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.rs"), "1\n2\n3\n").unwrap();
    std::fs::write(d.path().join("notas.txt"), "no es codigo\n").unwrap();
    let ctx = Ctx {
        root: d.path().into(),
        sha: "abc".into(),
        profile: Profile::default(),
    };
    let (cov, f) = run_all(&[Box::new(Loc)], &ctx);
    assert_eq!(cov[0].dimension, "code_size");
    assert_eq!(cov[0].status, Status::Ok);
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].metric.as_str(), f[0].value), ("lines", 3.0));
    assert_eq!(serde_json::to_value(&f[0]).unwrap()["method"], "static");
    assert_eq!(serde_json::to_value(&f[0]).unwrap()["confidence"], "high");
}
