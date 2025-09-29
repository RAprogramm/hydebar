//! Output management façade, re-exporting the collection state and helpers.

mod config;
mod state;
mod wayland;

pub use state::{HasOutput, Outputs};
