//! Kition Vector against the Kition platform convergence suite (L6): two to five replicas of a
//! drawing, edited through the engine's own commands.

mod common;

use std::path::Path;

use common::{NAME, VectorKit};
use kition_convergence::wasm::{self, WasmEngine};
use kition_convergence::{Options, Outcome};

#[test]
fn converges() {
    let report = kition_convergence::run(&VectorKit, &Options::from_env());
    println!("{}", report.render());
    report.save_if_requested().unwrap();
    assert!(report.level6(), "{}", report.render());
    // Vector keeps a text object's characters in one register (`Node.data`), not a KDM `text`
    // field, so the adapter offers no text to type into concurrently: the text scenarios skip.
    assert!(matches!(report.outcome("text-position"), Some(Outcome::Skipped(_))));
}

/// Native and WebAssembly engines reach one state from one change log. Set `KITION_WASM_ENGINE`
/// to a folder with `kition_vector_kit.js` and `kition_vector_kit_bg.wasm` (a package's
/// `engine/`, or a `wasm-pack` output).
#[test]
fn wasm_parity() {
    let Some(engine) = WasmEngine::from_env("kition_vector_kit", "VectorDocument", NAME) else {
        println!("SKIP wasm_parity: KITION_WASM_ENGINE is not set");
        return;
    };
    wasm::check_parity(&VectorKit, &engine, Path::new(env!("CARGO_TARGET_TMPDIR")), 1).unwrap();
}
