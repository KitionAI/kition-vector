//! How a host runs this kit's documents (KHP A29, section 10.6).
//!
//! Every kit document reaches no file by path, in the page and headless: [`crate::VectorDocument`]
//! turns on the engine's host-only mode ([`kition_vector_engine::file_access`]) for its session
//! and for the process when it makes the session. What this module adds is whether a host that
//! serves the kit to an agent runs it (`kition-vector`, the headless kit): then the commands that
//! cannot work without a folder of their own ([`crate::gate::HOSTED_OFF`]) leave the command list.
//!
//! The kit's pinned kition-sdk has no hosting signal yet, so the headless `main` calls
//! [`set_files_only`] before it serves. Newer kition-sdk revisions set
//! `kition_kit::host::set_hosting` in kition-headless and tell each document through
//! `KitDocument::hosted`; once the kit pins one, [`files_only`] reads
//! `kition_kit::host::hosting().files_only`, and [`set_files_only`] and its call in `main` go.

use std::sync::atomic::{AtomicBool, Ordering};

static FILES_ONLY: AtomicBool = AtomicBool::new(false);

/// Says a host serves this process's documents to an agent, owning every file: what
/// kition-headless's `set_hosting` will say. It also turns on the engine's host-only mode for the
/// whole process.
pub fn set_files_only() {
    FILES_ONLY.store(true, Ordering::SeqCst);
    kition_vector_engine::file_access::host_files_only_process();
}

/// Whether a host serves this process's documents to an agent ([`set_files_only`]).
pub fn files_only() -> bool {
    FILES_ONLY.load(Ordering::SeqCst)
}
