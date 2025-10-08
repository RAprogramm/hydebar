/// Battery module view layer - Pure rendering, no business logic
use hydebar_core::{
    components::icons::icon,
    config::BatteryModuleConfig,
    modules::battery::{BatteryData, IndicatorState},
};
use iced::{
    Alignment, Element, Theme,
    widget::{container, row, text},
};

use crate::app::Message;

/// Render battery indicator for the bar
pub fn render_battery_indicator(
    data: &BatteryData,
    config: &BatteryModuleConfig,
) -> Element<'static, Message,>
{
    let mut content = row![icon(data.icon.into())].align_y(Alignment::Center,).spacing(4,);

    if config.show_percentage {
        content = content.push(text(format!("{}%", data.capacity),),);
    }

    let indicator_state = data.indicator_state;
    container(content,)
        .style(move |theme: &Theme| container::Style {
            text_color: Some(match indicator_state {
                IndicatorState::Success => theme.palette().success,
                IndicatorState::Warning => theme.extended_palette().danger.weak.color,
                IndicatorState::Danger => theme.palette().danger,
                IndicatorState::Normal => theme.palette().text,
            },),
            ..Default::default()
        },)
        .into()
}

/// Render power profile indicator
pub fn render_power_profile(data: &BatteryData,) -> Element<'static, Message,>
{
    container(icon(data.power_profile.into(),),)
        .style(|theme: &Theme| container::Style {
            text_color: Some(theme.palette().primary,),
            ..Default::default()
        },)
        .into()
}

/// Render complete battery widget (indicator + profile)
pub fn render_battery(
    data: &BatteryData,
    config: &BatteryModuleConfig,
) -> Element<'static, Message,>
{
    let mut segments = vec![];

    if config.show_power_profile {
        segments.push(render_power_profile(data,),);
    }

    segments.push(render_battery_indicator(data, config,),);

    row(segments,).align_y(Alignment::Center,).spacing(4,).into()
}
