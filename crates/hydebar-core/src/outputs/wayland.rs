use iced::{
    Task,
    platform_specific::shell::commands::layer_surface::{
        Anchor, KeyboardInteractivity, Layer, destroy_layer_surface, get_layer_surface,
    },
    runtime::platform_specific::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
    window::Id,
};
use wayland_client::protocol::wl_output::WlOutput;

use crate::{
    HEIGHT,
    config::{AppearanceStyle, Position},
};

pub(crate) struct LayerSurfaceCreation<Message,>
{
    pub(crate) main_id: Id,
    pub(crate) menu_id: Id,
    pub(crate) task:    Task<Message,>,
}

pub(crate) fn layer_height(style: AppearanceStyle, scale_factor: f64,) -> f64
{
    (HEIGHT
        - match style {
            AppearanceStyle::Solid | AppearanceStyle::Gradient => 8.,
            AppearanceStyle::Islands => 0.,
        })
        * scale_factor
}

pub(crate) fn create_layer_surfaces<Message: 'static,>(
    style: AppearanceStyle,
    wl_output: Option<WlOutput,>,
    position: Position,
    menu_keyboard_focus: bool,
    scale_factor: f64,
) -> LayerSurfaceCreation<Message,>
{
    let main_id = Id::unique();
    let height = layer_height(style, scale_factor,);

    let main_task = get_layer_surface(SctkLayerSurfaceSettings {
        id: main_id,
        namespace: "hydebar-main-layer".to_string(),
        size: Some((None, Some(height as u32,),),),
        layer: Layer::Bottom,
        pointer_interactivity: true,
        keyboard_interactivity: if menu_keyboard_focus {
            KeyboardInteractivity::OnDemand
        } else {
            KeyboardInteractivity::None
        },
        exclusive_zone: height as i32,
        output: wl_output.clone().map_or(IcedOutput::Active, IcedOutput::Output,),
        anchor: match position {
            Position::Top => Anchor::TOP,
            Position::Bottom => Anchor::BOTTOM,
        } | Anchor::LEFT
            | Anchor::RIGHT,
        ..Default::default()
    },);

    let menu_id = Id::unique();
    let menu_task = get_layer_surface(SctkLayerSurfaceSettings {
        id: menu_id,
        namespace: "hydebar-main-layer".to_string(),
        size: Some((None, None,),),
        layer: Layer::Background,
        pointer_interactivity: true,
        keyboard_interactivity: KeyboardInteractivity::None,
        output: wl_output.map_or(IcedOutput::Active, IcedOutput::Output,),
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        ..Default::default()
    },);

    LayerSurfaceCreation {
        main_id,
        menu_id,
        task: Task::batch(vec![main_task, menu_task],),
    }
}

pub(crate) fn destroy_layer_surfaces<Message: 'static,>(
    main_id: Id, menu_id: Id,
) -> Task<Message,>
{
    Task::batch(vec![destroy_layer_surface(main_id,), destroy_layer_surface(menu_id,)],)
}
