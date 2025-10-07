use flexi_logger::LogSpecification;

mod centerbox;
mod views;

pub mod app;

pub use app::{App, Message};

pub fn get_log_spec(log_level: &str) -> LogSpecification {
    LogSpecification::env_or_parse(log_level).unwrap_or_else(|err| {
        panic!("Failed to parse log level: {err}");
    })
}
