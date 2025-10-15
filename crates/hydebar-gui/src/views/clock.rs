/// Clock module view layer - Pure rendering, no business logic
use hydebar_core::modules::clock::ClockData;
use iced::{Element, widget::text};

use crate::app::Message;

/// Render clock with given format
pub fn render_clock(data: &ClockData, format: &str) -> Element<'static, Message> {
    text(data.format(format)).into()
}
