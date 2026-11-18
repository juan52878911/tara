//! Recorrido determinista del árbol de ficheros y utilidades de entorno.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".tara",
    ".delega",
    ".venv",
    "venv",
    "__pycache__",
];

/// Rutas relativas (con `/`) de todos los ficheros, ordenadas. No sigue enlaces simbólicos.
pub fn walk(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk_dir(root, root, &mut out);
    out.sort();
    out
}

fn walk_dir(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        let path: PathBuf = entry.path();
        if ft.is_dir() {
            let name = entry.file_name();
            if IGNORED_DIRS.iter().any(|d| name == *d) {
                continue;
            }
            walk_dir(root, &path, out);
        } else if ft.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                let parts: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push(parts.join("/"));
            }
        }
    }
}

/// Busca un binario en `PATH`.
pub fn which(bin: &str) -> bool {
    if bin.contains('/') {
        return Path::new(bin).is_file();
    }
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
        .unwrap_or(false)
}

/// Marca de tiempo RFC 3339 en UTC. `TARA_NOW` la fija (para salidas reproducibles).
pub fn now_rfc3339() -> String {
    if let Ok(v) = std::env::var("TARA_NOW") {
        if !v.is_empty() {
            return v;
        }
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    format_unix(secs)
}

pub fn format_unix(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Fecha civil desde días desde 1970-01-01 (algoritmo de H. Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::format_unix;

    #[test]
    fn formatea_fechas() {
        assert_eq!(format_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix(1_795_000_000), "2026-11-18T11:06:40Z");
    }
}
