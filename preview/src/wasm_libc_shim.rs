//! The one libc symbol the wasm client is missing when tree-sitter is linked.
//!
//! `arborium` (via `dioxus-code`'s `lang-css` -> `runtime`) pulls tree-sitter's
//! C runtime into the wasm client; the debug Style tab highlights CSS in the
//! browser (`LazyCssCodeBlock` in `main.rs`). `arborium` 2.17.0 ships its own
//! Rust libc shim for `wasm32-unknown-unknown` (`arborium/src/wasm.rs`:
//! `malloc`, `free`, `fputs`, ...) but not the data symbol `stderr`, which
//! tree-sitter's `ts_stack_print_dot_graph` (`stack.c`: `if (!f) f = stderr;`)
//! reads. The sibling `arborium-sysroot` crate does define it, but is never
//! referenced, so it is never linked, and linking it as well duplicates every
//! other shim symbol (`duplicate symbol: malloc`...).
//!
//! rustc 1.97 (measured 2026-10-03) does not pass `--allow-undefined` for
//! `wasm32-unknown-unknown`, so an unoptimised (debug) build, where lld cannot
//! dead-strip that function, fails with `undefined symbol: stderr`. Release
//! builds link only because lld garbage-collects the unreachable function.
//! Defining the symbol once here makes the link strict and complete in every
//! profile instead of tolerating undefined symbols. The debug-graph printer is
//! never called, so the value is never dereferenced.

#![allow(non_upper_case_globals)]

/// C `FILE *stderr`; see the module docs. Wasm-only: on a host target the
/// symbol must come from the system libc.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub static mut stderr: *mut core::ffi::c_void = core::ptr::null_mut();
