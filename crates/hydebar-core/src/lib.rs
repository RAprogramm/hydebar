/// Default height of the main status bar in logical pixels.
pub const HEIGHT: f64 = 34.;

pub mod adapters;
pub mod components;
pub mod config;
/// Event bus primitives for communicating UI updates across the core.
pub mod event_bus;
pub mod menu;
pub mod module_context;
pub mod modules;
pub mod outputs;
pub mod password_dialog;
pub mod position_button;
pub mod services;
pub mod style;
// Make test_utils available for both internal tests and cross-crate testing
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
pub mod utils;

pub use module_context::{ModuleContext, ModuleEventSender};
