#!/usr/bin/env bash
# Paso del dorado: toolchain de Rust y dependencias descargadas (en el trabajo no hay red a crates.io).
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo >/dev/null 2>&1; then
  triple="$(uname -m)-unknown-linux-gnu"
  base="https://static.rust-lang.org/rustup/dist/$triple"
  tmp="$(mktemp -d)"
  curl -fsSL -o "$tmp/rustup-init" "$base/rustup-init"
  curl -fsSL -o "$tmp/rustup-init.sha256" "$base/rustup-init.sha256"
  esperado="$(awk '{print $1}' "$tmp/rustup-init.sha256")"
  real="$(sha256sum "$tmp/rustup-init" | awk '{print $1}')"
  [ "$esperado" = "$real" ] || { echo "sha256 de rustup-init no coincide" >&2; exit 1; }
  chmod +x "$tmp/rustup-init"
  "$tmp/rustup-init" -y --profile minimal --component clippy,rustfmt
  rm -rf "$tmp"
fi
grep -q '.cargo/bin' ~/.bashrc 2>/dev/null || echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
cargo fetch --locked
