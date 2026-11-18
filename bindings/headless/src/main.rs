//! `kition-vector`: the Vector kit for Kition's agent. The same document the in-page editor
//! runs (`kition-vector-kit`), served headless over the Kit Protocol by `kition-headless`, as the
//! kit's `kit.json` declares it, with the object model
//! (`model.describe`, `model.batch`) over the document's change exchange. The host owns every
//! file (KHP A29): the kit reaches none by path.

const SANDBOXED: &[&str] = &[];
/// `inspect()` keys about the editing session rather than the file.
const VOLATILE: &[&str] = kition_vector_kit::EDITOR_STATE;

fn main() -> std::process::ExitCode {
    // What kition-headless's `set_hosting` says once the kit pins a kition-sdk that has it.
    kition_vector_kit::hosting::set_files_only();
    kition_headless::run_manifest_model::<kition_vector_kit::VectorDocument>(include_str!("../../../kit.json"), SANDBOXED, VOLATILE)
}
