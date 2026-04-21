//! Course Audit System — Rust + Leptos WebAssembly application.
//!
//! A privacy-first audit tool for PSU Computer Science students. All parsing
//! and auditing runs in the browser — nothing leaves the device.
//!
//! This file is intentionally tiny: it only wires up the panic hook and
//! mounts the `App` component. All UI state and event handlers live in
//! `app.rs`; curriculum matching lives in `logic::auditor`.

mod app;
mod components;
mod data;
mod logic;
mod models;

use leptos::{mount_to_body, view};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <app::App/> });
}
