//! Voxora core: everything between "audio became text" and "text is ready to paste".
//!
//! Text stages are pure and synchronous ([`normalize`], [`math`], [`guard`], [`asr_filter`]);
//! [`store`] and [`settings`] own persistence. Nothing here depends on Win32 or Tauri.

pub mod asr_filter;
pub mod context;
pub mod engines;
pub mod gesture;
pub mod guard;
pub mod math;
pub mod mode;
pub mod normalize;
pub mod pipeline;
pub mod prompts;
pub mod settings;
pub mod store;
