// Native Slint entry point.
//
// The system layer is intentionally reused from the Tauri implementation.
// These modules are Rust-only and do not depend on Tauri APIs.
#[path = "../../src-tauri/src/system/mod.rs"]
mod system;
mod monitor;

slint::include_modules!();

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

