//! The Rust side of the plugin: the whole tool window UI.
//!
//! `main.rs` runs [`ToolWindow`] inside the IDE when the plugin starts the
//! binary, and as a desktop window otherwise. [`ide`] is the message contract
//! with the plugin's `ide-host's Rust message bridge`.

pub mod dashboard;
pub mod ide;
pub mod run_settings;
pub mod stability;
pub mod studio;
pub mod studio_model;

#[cfg(test)]
pub(crate) static TEST_OUTBOX_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub use dashboard::Dashboard as ToolWindow;
