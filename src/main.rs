use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;
use tara::baseline::{self, default_adapters};
use tara::detect;

#[derive(Parser)]
#[command(
    name = "tara",
    version,
    about = "Estimación de consumo de recursos con línea base por commit"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args)]
struct Fmt {
    /// Salida JSON (por defecto)
    #[arg(long, conflicts_with = "compact")]
    json: bool,
    /// Tabla de texto
    #[arg(long)]
    compact: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Perfil del proyecto
    Detect {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[command(flatten)]
        fmt: Fmt,
    },
    /// Genera el informe y guarda la línea base del commit actual
    Scan {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Compara el árbol actual con la línea base de un sha o ref
    Compare {
        /// sha, ref o `latest`
        rev: String,
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Umbral de subida (en %) a partir del cual se considera regresión
        #[arg(long, default_value_t = 5.0)]
        threshold_pct: f64,
        #[command(flatten)]
        fmt: Fmt,
    },
}

fn run() -> Result<u8> {
    match Cli::parse().cmd {
        Cmd::Detect { path, fmt } => {
            let p = detect::detect(&path);
            if fmt.compact {
                print!("{}", detect::compact(&p));
            } else {
                println!("{}", serde_json::to_string_pretty(&p)?);
            }
            Ok(0)
        }
        Cmd::Scan { path } => {
            let r = baseline::scan(&path, &default_adapters())?;
            println!("{}", serde_json::to_string_pretty(&r)?);
            Ok(0)
        }
        Cmd::Compare {
            rev,
            path,
            threshold_pct,
            fmt,
        } => {
            let Some(base) = baseline::load_baseline(&path, &rev)? else {
                eprintln!(
                    "tara: no hay línea base para `{rev}`; ejecuta `tara scan` en ese commit"
                );
                return Ok(2);
            };
            let head = baseline::build_report(&path, &default_adapters())?;
            let c = baseline::compare_reports(&base, &head, threshold_pct);
            if fmt.compact {
                print!("{}", baseline::compact(&c));
            } else {
                println!("{}", serde_json::to_string_pretty(&c)?);
            }
            Ok(u8::from(c.regression))
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("tara: {e:#}");
            ExitCode::from(2)
        }
    }
}
