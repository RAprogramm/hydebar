use iced::{
    Task,
    platform_specific::shell::commands::layer_surface::{
        Anchor, set_anchor, set_exclusive_zone, set_size
    },
    window::Id
};
use log::debug;
use wayland_client::protocol::wl_output::WlOutput;

use super::{
    config::is_output_requested,
    wayland::{LayerSurfaceCreation, create_layer_surfaces, destroy_layer_surfaces, layer_height}
};
use crate::{
    config::{self, AppearanceStyle, Position},
    menu::{Menu, MenuType},
    position_button::ButtonUIRef
};

#[derive(Debug, Clone)]
struct ShellInfo {
    id:           Id,
    position:     Position,
    style:        AppearanceStyle,
    menu:         Menu,
    scale_factor: f64
}

/// Collection of Wayland outputs currently tracked by the bar.
///
/// Instances manage Wayland layer-surfaces for both the main bar surface and
/// the associated menu surface per monitor. All operations return [`Task`]
/// objects that must be executed by the caller to coordinate with the
/// compositor.
///
/// # Examples
///
/// ```
/// # use hydebar_core::outputs::Outputs;
/// # use hydebar_core::config::Config;
/// let config = Config::default();
/// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
/// assert!(!outputs.menu_is_open());
/// ```
#[derive(Debug, Clone)]
pub struct Outputs(Vec<(Option<String>, Option<ShellInfo>, Option<WlOutput>)>);

/// Result of looking up a Wayland surface identifier.
///
/// The lookup differentiates between the main bar surface and the menu surface
/// so that event handlers can update the appropriate component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HasOutput<'a> {
    /// The identifier refers to the main bar surface.
    Main,
    /// The identifier refers to the menu surface along with its optional
    /// metadata about the menu currently shown.
    Menu(Option<&'a (MenuType, ButtonUIRef)>)
}

impl Outputs {
    /// Construct a new collection with a fallback surface that is active even
    /// before the compositor reports specific monitors.
    ///
    /// The returned [`Task`] must be spawned so that the fallback layer-surface
    /// is created. Once actual monitors appear, [`Outputs::add`] replaces this
    /// fallback entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (outputs, task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(!outputs.menu_is_open());
    /// # let _ = task;
    /// ```
    pub fn new<Message: 'static>(
        style: AppearanceStyle,
        position: Position,
        config: &crate::config::Config
    ) -> (Self, Task<Message>) {
        let LayerSurfaceCreation {
            main_id,
            menu_id,
            task
        } = create_layer_surfaces(
            style,
            None,
            position,
            config.menu_keyboard_focus,
            config.appearance.scale_factor
        );

        (
            Self(vec![(
                None,
                Some(ShellInfo {
                    id: main_id,
                    menu: Menu::new(menu_id),
                    position,
                    style,
                    scale_factor: config.appearance.scale_factor
                }),
                None
            )]),
            task
        )
    }

    /// Attempt to resolve a window [`Id`] to a tracked output or menu surface.
    ///
    /// Returns [`None`] when the identifier does not belong to the bar.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// # use iced::window::Id;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// let unknown = Id::unique();
    /// assert!(outputs.has(unknown).is_none());
    /// ```
    pub fn has(&self, id: Id) -> Option<HasOutput<'_>> {
        self.0.iter().find_map(|(_, info, _)| {
            if let Some(info) = info {
                if info.id == id {
                    Some(HasOutput::Main)
                } else if info.menu.id == id {
                    Some(HasOutput::Menu(info.menu.menu_info.as_ref()))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Retrieve the monitor name associated with a given surface identifier.
    ///
    /// Returns [`None`] when the identifier does not belong to a tracked output
    /// or the output has no reported name yet.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// # use iced::window::Id;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(outputs.get_monitor_name(Id::unique()).is_none());
    /// ```
    pub fn get_monitor_name(&self, id: Id) -> Option<&str> {
        self.0.iter().find_map(|(name, info, _)| {
            if let Some(info) = info {
                if info.id == id { name.as_deref() } else { None }
            } else {
                None
            }
        })
    }

    /// Check whether an output with the provided name is already tracked.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(!outputs.has_name("DP-1"));
    /// ```
    pub fn has_name(&self, name: &str) -> bool {
        self.0
            .iter()
            .any(|(n, info, _)| info.is_some() && n.as_deref() == Some(name))
    }

    /// Register a new monitor if it matches the configuration filters.
    ///
    /// Callers must execute the returned [`Task`] to materialise the
    /// compositor-side layer-surfaces. When the monitor name is not requested
    /// by configuration the [`Task`] is empty and the state records the
    /// Wayland output for future synchronisation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let (mut outputs, _) = Outputs::new(style, position, &config);
    /// let wl_output = obtain_wl_output();
    /// let task = outputs.add(style, &config.outputs, position, name, wl_output, &config);
    /// spawn(task);
    /// ```
    pub fn add<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        name: &str,
        wl_output: WlOutput,
        config: &crate::config::Config
    ) -> Task<Message> {
        let target = is_output_requested(Some(name), request_outputs);

        if target {
            debug!("Found target output, creating a new layer surface");

            let LayerSurfaceCreation {
                main_id,
                menu_id,
                task
            } = create_layer_surfaces(
                style,
                Some(wl_output.clone()),
                position,
                config.menu_keyboard_focus,
                config.appearance.scale_factor
            );

            let destroy_task = match self
                .0
                .iter()
                .position(|(key, _, _)| key.as_deref() == Some(name))
            {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => {
                            destroy_layer_surfaces(shell_info.id, shell_info.menu.id)
                        }
                        _ => Task::none()
                    }
                }
                _ => Task::none()
            };

            self.0.push((
                Some(name.to_owned()),
                Some(ShellInfo {
                    id: main_id,
                    menu: Menu::new(menu_id),
                    position,
                    style,
                    scale_factor: config.appearance.scale_factor
                }),
                Some(wl_output)
            ));

            let destroy_fallback_task = match self.0.iter().position(|(key, _, _)| key.is_none()) {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => {
                            destroy_layer_surfaces(shell_info.id, shell_info.menu.id)
                        }
                        _ => Task::none()
                    }
                }
                _ => Task::none()
            };

            Task::batch(vec![destroy_task, destroy_fallback_task, task])
        } else {
            self.0.push((Some(name.to_owned()), None, Some(wl_output)));

            Task::none()
        }
    }

    /// Remove the layer-surfaces associated with a departed monitor.
    ///
    /// The returned [`Task`] destroys the compositor resources and potentially
    /// spawns a fallback surface when no monitors remain.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let task = outputs.remove(style, position, wl_output, &config);
    /// spawn(task);
    /// ```
    pub fn remove<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        position: Position,
        wl_output: WlOutput,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter().position(|(_, _, assigned_wl_output)| {
            assigned_wl_output
                .as_ref()
                .map(|assigned_wl_output| *assigned_wl_output == wl_output)
                .unwrap_or_default()
        }) {
            Some(index_to_remove) => {
                debug!("Removing layer surface for output");

                let (name, shell_info, wl_output) = self.0.swap_remove(index_to_remove);

                let destroy_task = if let Some(shell_info) = shell_info {
                    destroy_layer_surfaces(shell_info.id, shell_info.menu.id)
                } else {
                    Task::none()
                };

                self.0.push((name.to_owned(), None, wl_output));

                if !self.0.iter().any(|(_, shell_info, _)| shell_info.is_some()) {
                    debug!("No outputs left, creating a fallback layer surface");

                    let LayerSurfaceCreation {
                        main_id,
                        menu_id,
                        task
                    } = create_layer_surfaces(
                        style,
                        None,
                        position,
                        config.menu_keyboard_focus,
                        config.appearance.scale_factor
                    );

                    self.0.push((
                        None,
                        Some(ShellInfo {
                            id: main_id,
                            menu: Menu::new(menu_id),
                            position,
                            style,
                            scale_factor: config.appearance.scale_factor
                        }),
                        None
                    ));

                    Task::batch(vec![destroy_task, task])
                } else {
                    Task::batch(vec![destroy_task])
                }
            }
            _ => Task::none()
        }
    }

    /// Synchronise the tracked outputs with the desired configuration.
    ///
    /// The method returns a [`Task`] aggregating all compositor operations
    /// required to add or remove surfaces as well as to update style or
    /// position changes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (mut outputs, _task) =
    ///     Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// let task = outputs.sync::<()>(
    ///     config.appearance.style,
    ///     &config.outputs,
    ///     config.position,
    ///     &config
    /// );
    /// # let _ = task;
    /// ```
    pub fn sync<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        config: &crate::config::Config
    ) -> Task<Message> {
        debug!("Syncing outputs: {self:?}, request_outputs: {request_outputs:?}");

        let to_remove = self
            .0
            .iter()
            .filter_map(|(name, shell_info, wl_output)| {
                if !is_output_requested(name.as_deref(), request_outputs) && shell_info.is_some() {
                    Some(wl_output.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();
        debug!("Removing outputs: {to_remove:?}");

        let to_add = self
            .0
            .iter()
            .filter_map(|(name, shell_info, wl_output)| {
                if is_output_requested(name.as_deref(), request_outputs) && shell_info.is_none() {
                    Some((name.clone(), wl_output.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Adding outputs: {to_add:?}");

        let mut tasks = Vec::new();

        for (name, wl_output) in to_add {
            if let Some(wl_output) = wl_output
                && let Some(name) = name
            {
                tasks.push(self.add(
                    style,
                    request_outputs,
                    position,
                    name.as_str(),
                    wl_output,
                    config
                ));
            }
        }

        for wl_output in to_remove {
            tasks.push(self.remove(style, position, wl_output, config));
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info
                && shell_info.position != position
            {
                Some(shell_info)
            } else {
                None
            }
        }) {
            debug!(
                "Repositioning output: {:?}, new position {:?}",
                shell_info.id, position
            );
            shell_info.position = position;
            tasks.push(set_anchor(
                shell_info.id,
                match position {
                    Position::Top => Anchor::TOP,
                    Position::Bottom => Anchor::BOTTOM
                } | Anchor::LEFT
                    | Anchor::RIGHT
            ));
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info
                && (shell_info.style != style
                    || shell_info.scale_factor != config.appearance.scale_factor)
            {
                Some(shell_info)
            } else {
                None
            }
        }) {
            debug!(
                "Change style or scale_factor for output: {:?}, new style {:?}, new scale_factor {:?}",
                shell_info.id, style, config.appearance.scale_factor
            );
            shell_info.style = style;
            shell_info.scale_factor = config.appearance.scale_factor;
            let height = layer_height(style, config.appearance.scale_factor);
            tasks.push(Task::batch(vec![
                set_size(shell_info.id, None, Some(height as u32)),
                set_exclusive_zone(shell_info.id, height as i32),
            ]));
        }

        Task::batch(tasks)
    }

    /// Determine whether any tracked menu surface is currently visible.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(!outputs.menu_is_open());
    /// ```
    pub fn menu_is_open(&self) -> bool {
        self.0.iter().any(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .map(|shell_info| shell_info.menu.menu_info.is_some())
                .unwrap_or_default()
        })
    }

    /// Get the animated opacity for a menu window.
    pub fn get_menu_opacity(&self, id: Id) -> f32 {
        self.0
            .iter()
            .find_map(|(_, shell_info, _)| {
                shell_info.as_ref().and_then(|shell_info| {
                    if shell_info.menu.id == id {
                        Some(shell_info.menu.get_opacity())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0.0)
    }

    /// Update menu animations. Returns true if any menu is currently animating.
    pub fn tick_menu_animations(
        &mut self,
        animation_config: &crate::config::AnimationConfig
    ) -> bool {
        let mut is_animating = false;
        for (_, shell_info, _) in &mut self.0 {
            if let Some(shell_info) = shell_info
                && shell_info.menu.tick_animation(animation_config)
            {
                is_animating = true;
            }
        }
        is_animating
    }

    /// Toggle the menu associated with the provided surface identifier.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let task = outputs.toggle_menu(surface_id, MenuType::Tray("battery".into()), button_ref, &config);
    /// spawn(task);
    /// ```
    pub fn toggle_menu<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                let toggle_task = shell_info.menu.toggle(menu_type, button_ui_ref, config);
                let mut tasks = self
                    .0
                    .iter_mut()
                    .filter_map(|(_, shell_info, _)| {
                        if let Some(shell_info) = shell_info {
                            if shell_info.id != id && shell_info.menu.id != id {
                                Some(shell_info.menu.close(config))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                tasks.push(toggle_task);
                Task::batch(tasks)
            }
            _ => Task::none()
        }
    }

    /// Close the menu for a specific surface when it is currently open.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.close_menu(surface_id, &config);
    /// ```
    pub fn close_menu<Message: 'static>(
        &mut self,
        id: Id,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close(config),
            _ => Task::none()
        }
    }

    /// Close the menu only when it matches the specified [`MenuType`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.close_menu_if(surface_id, MenuType::Updates, &config);
    /// ```
    pub fn close_menu_if<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close_if(menu_type, config),
            _ => Task::none()
        }
    }

    /// Close every menu that matches the specified [`MenuType`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.close_all_menu_if(MenuType::Tray("network".into()), &config);
    /// ```
    pub fn close_all_menu_if<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        config: &crate::config::Config
    ) -> Task<Message> {
        Task::batch(
            self.0
                .iter_mut()
                .map(|(_, shell_info, _)| {
                    if let Some(shell_info) = shell_info {
                        shell_info.menu.close_if(menu_type.clone(), config)
                    } else {
                        Task::none()
                    }
                })
                .collect::<Vec<_>>()
        )
    }

    /// Close every open menu regardless of its type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (mut outputs, _task) =
    ///     Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// outputs.close_all_menus::<()>(&config);
    /// ```
    pub fn close_all_menus<Message: 'static>(
        &mut self,
        config: &crate::config::Config
    ) -> Task<Message> {
        Task::batch(
            self.0
                .iter_mut()
                .map(|(_, shell_info, _)| {
                    if let Some(shell_info) = shell_info {
                        if shell_info.menu.menu_info.is_some() {
                            shell_info.menu.close(config)
                        } else {
                            Task::none()
                        }
                    } else {
                        Task::none()
                    }
                })
                .collect::<Vec<_>>()
        )
    }

    /// Request keyboard focus for the menu associated with the identifier.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.request_keyboard(surface_id, true);
    /// ```
    pub fn request_keyboard<Message: 'static>(
        &self,
        id: Id,
        menu_keyboard_focus: bool
    ) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                shell_info.menu.request_keyboard(menu_keyboard_focus)
            }
            _ => Task::none()
        }
    }

    /// Release keyboard focus from the identified menu surface.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.release_keyboard(surface_id, false);
    /// ```
    pub fn release_keyboard<Message: 'static>(
        &self,
        id: Id,
        menu_keyboard_focus: bool
    ) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                shell_info.menu.release_keyboard(menu_keyboard_focus)
            }
            _ => Task::none()
        }
    }

    #[cfg(test)]
    fn iter_internal(
        &self
    ) -> impl Iterator<Item = &(Option<String>, Option<ShellInfo>, Option<WlOutput>)> {
        self.0.iter()
    }
}

// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use iced::Point;

    use super::*;
    use crate::config::Config;

    #[test]
    fn toggle_menu_opens_and_closes() {
        let config = Config::default();
        let (mut outputs, _task) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let id = outputs
            .iter_internal()
            .next()
            .unwrap()
            .1
            .as_ref()
            .unwrap()
            .id;

        let button_ref = ButtonUIRef {
            position: Point::new(0.0, 0.0),
            viewport: (0., 0.)
        };
        outputs.toggle_menu::<()>(id, MenuType::Updates, button_ref, &config);
        assert!(outputs.menu_is_open());

        outputs.close_menu::<()>(id, &config);
        assert!(!outputs.menu_is_open());
    }

    #[test]
    fn sync_updates_position_internally() {
        let config = Config::default();
        let (mut outputs, _task) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let id = outputs
            .iter_internal()
            .next()
            .unwrap()
            .1
            .as_ref()
            .unwrap()
            .id;

        let mut updated_config = config.clone();
        updated_config.position = match updated_config.position {
            Position::Top => Position::Bottom,
            Position::Bottom => Position::Top
        };

        outputs.sync::<()>(
            updated_config.appearance.style,
            &updated_config.outputs,
            updated_config.position,
            &updated_config
        );

        assert!(matches!(outputs.has(id), Some(HasOutput::Main)));
    }
}
