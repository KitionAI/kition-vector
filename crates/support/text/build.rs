//! The optional kition-fonts build input (https://github.com/KitionAI/kition-fonts,
//! `docs/integration.md`): with `KITION_FONTS_DIR=<checkout>` set, every font in its
//! `fonts/manifest.txt` is embedded as `KITION_FONTS`; unset, `KITION_FONTS` is empty and the app
//! uses its own and the installed fonts. `KITION_FONTS_REQUIRED=1` turns a bad checkout into a build
//! error (release builds). Nothing is fetched: the build reads only the local checkout.
//!
//! Web (wasm32) builds embed only the UI font, BIZ UDPGothic Regular (~4.7 MB): the browser has no
//! system fonts, and all four fonts (~24 MB) would push the `.wasm` past static hosts' per-file
//! limits (Cloudflare Pages: 25 MiB).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-env-changed=KITION_FONTS_DIR");
    println!("cargo::rerun-if-env-changed=KITION_FONTS_REQUIRED");
    let required = std::env::var_os("KITION_FONTS_REQUIRED").is_some();
    let mut fonts = String::new();
    if let Some(dir) = std::env::var_os("KITION_FONTS_DIR").filter(|d| !d.is_empty()).map(PathBuf::from) {
        let dir = from_workspace(dir);
        match kition_fonts(&dir) {
            Ok(entries) => fonts = entries,
            Err(e) if required => println!("cargo::error=KITION_FONTS_DIR={}: {e}", dir.display()),
            Err(e) => println!("cargo::warning=building without kition-fonts: KITION_FONTS_DIR={}: {e}", dir.display()),
        }
    }
    let src = format!("pub static KITION_FONTS: &[KitionFont] = &[\n{fonts}];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("kition_fonts.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}

/// A relative `KITION_FONTS_DIR` is taken from the workspace root (where `cargo` is usually run),
/// not from this crate's directory, which is where build scripts run.
fn from_workspace(dir: PathBuf) -> PathBuf {
    if dir.is_absolute() {
        return dir;
    }
    let crate_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    crate_dir.join("../..").join(dir)
}

/// The one kition-fonts face web builds embed (family, style).
const WEB_FONT: (&str, &str) = ("BIZ UDPGothic", "Regular");

/// One `KitionFont { .. }` initialiser per manifest line.
fn kition_fonts(dir: &Path) -> Result<String, String> {
    let manifest = dir.join("fonts/manifest.txt");
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = f.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        if wasm && !(*family == WEB_FONT.0 && *style == WEB_FONT.1) {
            continue;
        }
        let path = dir.join(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
        println!("cargo::rerun-if-changed={}", path.display());
        let scripts: Vec<String> = scripts.split(',').map(|s| format!("{:?}", s.trim())).collect();
        let _ = writeln!(
            out,
            "    KitionFont {{ family: {family:?}, style: {style:?}, scripts: &[{}], bytes: include_bytes!({:?}) }},",
            scripts.join(", "),
            path.display().to_string(),
        );
    }
    Ok(out)
}
