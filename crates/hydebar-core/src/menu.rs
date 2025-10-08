use std::time::Instant;

use iced::{
    self, Element, Length, Padding, Task,
    alignment::{Horizontal, Vertical},
    platform_specific::shell::commands::layer_surface::{
        KeyboardInteractivity, Layer, set_keyboard_interactivity, set_layer,
    },
    widget::{container, mouse_area},
    window::Id,
};

use crate::{
    config::{AnimationConfig, AppearanceStyle, Position},
    position_button::ButtonUIRef,
    style::{menu_backdrop_style, menu_container_style},
};

#[derive(Eq, PartialEq, Clone, Debug,)]
pub enum MenuType
{
    Updates,
    Settings,
    Tray(String,),
    MediaPlayer,
    SystemInfo,
    Notifications,
    Screenshot,
}

#[derive(Clone, Debug,)]
pub struct Menu
{
    pub id:              Id,
    pub menu_info:       Option<(MenuType, ButtonUIRef,),>,
    pub current_opacity: f32,
    pub target_opacity:  f32,
    pub animation_start: Option<Instant,>,
}

impl Menu
{
    pub fn new(id: Id,) -> Self
    {
        Self {
            id,
            menu_info: None,
            current_opacity: 0.0,
            target_opacity: 0.0,
            animation_start: None,
        }
    }

    pub fn open<Message: 'static,>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config,
    ) -> Task<Message,>
    {
        self.menu_info.replace((menu_type, button_ui_ref,),);

        // Start fade-in animation
        if config.appearance.animations.enabled {
            self.target_opacity = config.appearance.menu.opacity;
            self.animation_start = Some(Instant::now(),);
        } else {
            self.current_opacity = config.appearance.menu.opacity;
            self.target_opacity = config.appearance.menu.opacity;
        }

        let mut tasks = vec![set_layer(self.id, Layer::Overlay,)];

        if config.menu_keyboard_focus {
            tasks.push(set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand,),);
        }

        Task::batch(tasks,)
    }

    pub fn close<Message: 'static,>(&mut self, config: &crate::config::Config,) -> Task<Message,>
    {
        if self.menu_info.is_some() {
            self.menu_info.take();

            // Start fade-out animation
            if config.appearance.animations.enabled {
                self.target_opacity = 0.0;
                self.animation_start = Some(Instant::now(),);
            } else {
                self.current_opacity = 0.0;
                self.target_opacity = 0.0;
            }

            let mut tasks = vec![set_layer(self.id, Layer::Background,)];

            if config.menu_keyboard_focus {
                tasks.push(set_keyboard_interactivity(self.id, KeyboardInteractivity::None,),);
            }

            Task::batch(tasks,)
        } else {
            Task::none()
        }
    }

    pub fn toggle<Message: 'static,>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config,
    ) -> Task<Message,>
    {
        match self.menu_info.as_mut() {
            None => self.open(menu_type, button_ui_ref, config,),
            Some((current_type, _,),) if *current_type == menu_type => self.close(config,),
            Some((current_type, current_button_ui_ref,),) => {
                *current_type = menu_type;
                *current_button_ui_ref = button_ui_ref;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static,>(
        &mut self,
        menu_type: MenuType,
        config: &crate::config::Config,
    ) -> Task<Message,>
    {
        if let Some((current_type, _,),) = self.menu_info.as_ref() {
            if *current_type == menu_type { self.close(config,) } else { Task::none() }
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static,>(&self, menu_keyboard_focus: bool,)
    -> Task<Message,>
    {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand,)
        } else {
            Task::none()
        }
    }

    pub fn release_keyboard<Message: 'static,>(&self, menu_keyboard_focus: bool,)
    -> Task<Message,>
    {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::None,)
        } else {
            Task::none()
        }
    }

    /// Update menu animation state. Returns true if animation is in progress.
    pub fn tick_animation(&mut self, animation_config: &AnimationConfig,) -> bool
    {
        if !animation_config.enabled {
            return false;
        }

        if let Some(start,) = self.animation_start {
            let elapsed = start.elapsed().as_millis() as u64;
            let duration = animation_config.menu_fade_duration_ms;

            if elapsed >= duration {
                // Animation complete
                self.current_opacity = self.target_opacity;
                self.animation_start = None;
                false
            } else {
                // Interpolate opacity
                let progress = elapsed as f32 / duration as f32;
                let delta = self.target_opacity - self.current_opacity;
                self.current_opacity += delta * progress;
                true
            }
        } else {
            false
        }
    }

    /// Get the current animated opacity for rendering
    pub fn get_opacity(&self,) -> f32
    {
        self.current_opacity
    }
}

pub enum MenuSize
{
    Small,
    Medium,
    Large,
}

impl MenuSize
{
    fn size(&self,) -> f32
    {
        match self {
            MenuSize::Small => 250.,
            MenuSize::Medium => 350.,
            MenuSize::Large => 450.,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn menu_wrapper<Message: Clone + 'static,>(
    _id: Id,
    content: Element<Message,>,
    menu_size: MenuSize,
    button_ui_ref: ButtonUIRef,
    bar_position: Position,
    style: AppearanceStyle,
    opacity: f32,
    menu_backdrop: f32,
    none_message: Message,
    close_menu_message: Message,
) -> Element<Message,>
{
    mouse_area(
        container(
            mouse_area(
                container(content,)
                    .height(Length::Shrink,)
                    .width(Length::Shrink,)
                    .max_width(menu_size.size(),)
                    .padding(16,)
                    .style(menu_container_style(opacity,),),
            )
            .on_release(none_message,),
        )
        .align_y(match bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom,
        },)
        .align_x(Horizontal::Left,)
        .padding({
            let size = menu_size.size();

            let v_padding = match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => 2,
                AppearanceStyle::Islands => 0,
            };

            Padding::new(0.,)
                .top(if bar_position == Position::Top { v_padding } else { 0 },)
                .bottom(if bar_position == Position::Bottom { v_padding } else { 0 },)
                .left(f32::min(
                    f32::max(button_ui_ref.position.x - size / 2., 8.,),
                    button_ui_ref.viewport.0 - size - 8.,
                ),)
        },)
        .width(Length::Fill,)
        .height(Length::Fill,)
        .style(menu_backdrop_style(menu_backdrop,),),
    )
    .on_release(close_menu_message,)
    .into()
}
