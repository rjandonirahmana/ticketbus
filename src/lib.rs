//! lib.rs — WASM entry point (Unified SSR + Hydration).
//!
//! `web` dikompilasi untuk KEDUA target (native SSR + wasm32 hydration).
//! Setiap modul backend (`config`, `middleware`, `repository`, `service`,
//! `state`) hanya untuk native — lihat aturan cfg-gating di README/CLAUDE.md.

#![recursion_limit = "512"]

#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod middleware;
#[cfg(not(target_arch = "wasm32"))]
pub mod repository;
#[cfg(not(target_arch = "wasm32"))]
pub mod service;
#[cfg(not(target_arch = "wasm32"))]
pub mod state;
#[cfg(not(target_arch = "wasm32"))]
pub mod utils;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use web::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
