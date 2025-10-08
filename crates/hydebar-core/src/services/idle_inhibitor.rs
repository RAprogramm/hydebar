pub mod error;

pub use error::IdleInhibitorError;
use log::{debug, info, warn};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    protocol::{
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface,
    },
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
    zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

pub struct IdleInhibitorManager
{
    _connection: Connection,
    _display:    WlDisplay,
    _registry:   WlRegistry,
    event_queue: EventQueue<IdleInhibitorManagerData,>,
    handle:      QueueHandle<IdleInhibitorManagerData,>,
    data:        IdleInhibitorManagerData,
}

impl IdleInhibitorManager
{
    /// Create a new idle inhibitor manager connected to the Wayland compositor.
    ///
    /// # Errors
    /// Returns [`IdleInhibitorError`] when the Wayland connection cannot be
    /// established, when required globals are missing, or when dispatching the
    /// initial event roundtrip fails.
    pub fn new() -> Result<Self, IdleInhibitorError,>
    {
        let init = || -> Result<Self, IdleInhibitorError,> {
            let connection = Connection::connect_to_env()?;
            let display = connection.display();
            let event_queue = connection.new_event_queue();
            let handle = event_queue.handle();
            let registry = display.get_registry(&handle, (),);

            let mut obj = Self {
                _connection: connection,
                _display: display,
                _registry: registry,
                event_queue,
                handle,
                data: IdleInhibitorManagerData::default(),
            };

            obj.roundtrip()?;
            obj.ensure_required_globals()?;

            Ok(obj,)
        };

        init()
    }

    fn roundtrip(&mut self,) -> Result<usize, IdleInhibitorError,>
    {
        self.event_queue.roundtrip(&mut self.data,).map_err(IdleInhibitorError::from,)
    }

    pub fn is_inhibited(&self,) -> bool
    {
        self.data.idle_inhibitor_state.is_some()
    }

    pub fn toggle(&mut self,)
    {
        let res = if self.is_inhibited() {
            self.set_inhibit_idle(false,)
        } else {
            self.set_inhibit_idle(true,)
        };

        if let Err(err,) = res {
            warn!("Failed to toggle idle inhibitor: {err}");
        }
    }

    fn set_inhibit_idle(&mut self, inhibit_idle: bool,) -> Result<(), IdleInhibitorError,>
    {
        let data = &self.data;
        let (idle_manager, _,) = data
            .idle_manager
            .as_ref()
            .ok_or_else(IdleInhibitorError::missing_idle_inhibit_manager,)?;

        if inhibit_idle {
            if data.idle_inhibitor_state.is_none() {
                let surface =
                    data.surface.as_ref().ok_or_else(IdleInhibitorError::missing_surface,)?;
                self.data.idle_inhibitor_state =
                    Some(idle_manager.create_inhibitor(surface, &self.handle, (),),);

                self.roundtrip()?;
                info!(target: "IdleInhibitor::set_inhibit_idle", "Idle Inhibitor was ENABLED");
            }
        } else if let Some(state,) = &self.data.idle_inhibitor_state {
            state.destroy();
            self.data.idle_inhibitor_state = None;

            self.roundtrip()?;
            info!(target: "IdleInhibitor::set_inhibit_idle", "Idle Inhibitor was DISABLED");
        }

        Ok((),)
    }

    fn ensure_required_globals(&self,) -> Result<(), IdleInhibitorError,>
    {
        let state = IdleInhibitorInitState {
            has_compositor:   self.data.compositor.is_some(),
            has_surface:      self.data.surface.is_some(),
            has_idle_manager: self.data.idle_manager.is_some(),
        };

        Self::validate_init_state(state,)
    }

    fn validate_init_state(state: IdleInhibitorInitState,) -> Result<(), IdleInhibitorError,>
    {
        if !state.has_compositor {
            return Err(IdleInhibitorError::missing_compositor(),);
        }

        if !state.has_surface {
            return Err(IdleInhibitorError::missing_surface(),);
        }

        if !state.has_idle_manager {
            return Err(IdleInhibitorError::missing_idle_inhibit_manager(),);
        }

        Ok((),)
    }
}

#[derive(Clone, Copy,)]
struct IdleInhibitorInitState
{
    has_compositor:   bool,
    has_surface:      bool,
    has_idle_manager: bool,
}

#[derive(Default,)]
struct IdleInhibitorManagerData
{
    compositor:           Option<(WlCompositor, u32,),>,
    surface:              Option<WlSurface,>,
    idle_manager:         Option<(ZwpIdleInhibitManagerV1, u32,),>,
    idle_inhibitor_state: Option<ZwpIdleInhibitorV1,>,
}

impl Dispatch<WlRegistry, (),> for IdleInhibitorManagerData
{
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        handle: &wayland_client::QueueHandle<Self,>,
    )
    {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == WlCompositor::interface().name && state.compositor.is_none() {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding Compositor with name {name} and version {version}");
                    let compositor: WlCompositor = proxy.bind(name, version, handle, (),);

                    state.surface = Some(compositor.create_surface(handle, (),),);
                    state.compositor = Some((compositor, name,),);
                } else if interface == ZwpIdleInhibitManagerV1::interface().name
                    && state.idle_manager.is_none()
                {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding IdleInhibitManager with name {name} and version {version}");
                    state.idle_manager = Some((proxy.bind(name, version, handle, (),), name,),);
                };
            }
            wl_registry::Event::GlobalRemove {
                name,
            } => match &state.compositor {
                Some((_, compositor_name,),) => {
                    if name == *compositor_name {
                        warn!(target: "IdleInhibitor::GlobalRemove", "Compositor was removed!");

                        state.compositor = None;
                        state.surface = None;
                    }
                }
                _ => {
                    if let Some((_, idle_manager_name,),) = &state.idle_manager
                        && name == *idle_manager_name
                    {
                        warn!(target: "IdleInhibitor::GlobalRemove", "IdleInhibitManager was removed!");

                        state.idle_manager = None;
                    }
                }
            },
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, (),> for IdleInhibitorManagerData
{
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self,>,
    )
    {
    } // This interface has no events.
}

impl Dispatch<WlSurface, (),> for IdleInhibitorManagerData
{
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self,>,
    )
    {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, (),> for IdleInhibitorManagerData
{
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitManagerV1,
        _event: <ZwpIdleInhibitManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self,>,
    )
    {
    } // This interface has no events.
}

impl Dispatch<ZwpIdleInhibitorV1, (),> for IdleInhibitorManagerData
{
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitorV1,
        _event: <ZwpIdleInhibitorV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self,>,
    )
    {
    } // This interface has no events.
}

#[cfg(test)]
mod tests
{
    use super::{IdleInhibitorError, IdleInhibitorInitState, IdleInhibitorManager};

    #[test]
    fn validate_init_state_succeeds_with_all_globals()
    {
        let state = IdleInhibitorInitState {
            has_compositor:   true,
            has_surface:      true,
            has_idle_manager: true,
        };

        IdleInhibitorManager::validate_init_state(state,).expect("state should be valid",);
    }

    #[test]
    fn validate_init_state_fails_without_idle_manager()
    {
        let state = IdleInhibitorInitState {
            has_compositor:   true,
            has_surface:      true,
            has_idle_manager: false,
        };

        let err = IdleInhibitorManager::validate_init_state(state,).unwrap_err();
        assert!(matches!(err, IdleInhibitorError::MissingGlobal { .. }));
    }
}
