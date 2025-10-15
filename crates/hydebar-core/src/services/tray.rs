use std::{future::Future, ops::Deref, pin::Pin};

use dbus::{DBusMenuProxy, Layout, StatusNotifierItemProxy};
use iced::{
    Task,
    widget::{image, svg}
};
use log::{debug, error};
use masterror::{AppError, AppResult};

use super::{ReadOnlyService, Service, ServiceEvent};

pub mod dbus;

mod icon;
mod watcher;

#[derive(Debug, Clone)]
pub enum TrayIcon {
    Image(image::Handle),
    Svg(svg::Handle)
}

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Registered(StatusNotifierItem),
    IconChanged(String, TrayIcon),
    MenuLayoutChanged(String, Layout),
    Unregistered(String),
    None
}

#[derive(Debug, Clone)]
pub struct StatusNotifierItem {
    pub name:   String,
    pub icon:   Option<TrayIcon>,
    pub menu:   Layout,
    item_proxy: StatusNotifierItemProxy<'static>,
    menu_proxy: DBusMenuProxy<'static>
}

impl StatusNotifierItem {
    pub async fn new(conn: &zbus::Connection, name: String) -> AppResult<Self> {
        let (dest, path) = if let Some(idx) = name.find('/') {
            (&name[..idx], &name[idx..])
        } else {
            (name.as_ref(), "/StatusNotifierItem")
        };

        let item_proxy = StatusNotifierItemProxy::builder(conn)
            .destination(dest.to_owned())
            .map_err(|e| {
                AppError::internal(format!(
                    "Failed to set StatusNotifierItemProxy destination: {}",
                    e
                ))
            })?
            .path(path.to_owned())
            .map_err(|e| {
                AppError::internal(format!("Failed to set StatusNotifierItemProxy path: {}", e))
            })?
            .build()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to build StatusNotifierItemProxy: {}", e))
            })?;

        debug!("item_proxy {item_proxy:?}");

        let icon_pixmap = item_proxy.icon_pixmap().await;

        let icon = match icon_pixmap {
            Ok(icons) => {
                debug!("icon_pixmap {icons:?}");
                icon::icon_from_pixmaps(icons)
            }
            Err(_) => item_proxy
                .icon_name()
                .await
                .ok()
                .as_deref()
                .and_then(icon::icon_from_name)
        };

        let menu_path = item_proxy
            .menu()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get menu path: {}", e)))?;
        let menu_proxy = dbus::DBusMenuProxy::builder(conn)
            .destination(dest.to_owned())
            .map_err(|e| {
                AppError::internal(format!("Failed to set DBusMenuProxy destination: {}", e))
            })?
            .path(menu_path.to_owned())
            .map_err(|e| AppError::internal(format!("Failed to set DBusMenuProxy path: {}", e)))?
            .build()
            .await
            .map_err(|e| AppError::internal(format!("Failed to build DBusMenuProxy: {}", e)))?;

        let (_, menu) = menu_proxy
            .get_layout(0, -1, &[])
            .await
            .map_err(|e| AppError::internal(format!("Failed to get menu layout: {}", e)))?;

        Ok(Self {
            name,
            icon,
            menu,
            item_proxy,
            menu_proxy
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct TrayData(Vec<StatusNotifierItem>);

impl Deref for TrayData {
    type Target = Vec<StatusNotifierItem>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct TrayService {
    pub data: TrayData,
    _conn:    zbus::Connection
}

impl Deref for TrayService {
    type Target = TrayData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl TrayService {
    /// Start listening for tray events using the underlying D-Bus watcher.
    ///
    /// The provided `publisher` receives service lifecycle events as they are
    /// produced by the watcher loop.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use hydebar_core::services::{ServiceEvent, tray::TrayService};
    ///
    /// async fn listen() {
    ///     TrayService::start_listening(|_event: ServiceEvent<TrayService>| async {}).await;
    /// }
    /// ```
    pub async fn start_listening<F, Fut>(publisher: F)
    where
        F: FnMut(ServiceEvent<Self>) -> Fut + Send,
        Fut: Future<Output = ()> + Send
    {
        watcher::start_listening(publisher).await;
    }

    pub async fn menu_voice_selected(
        menu_proxy: &DBusMenuProxy<'_>,
        id: i32
    ) -> AppResult<Layout> {
        let value = zbus::zvariant::Value::I32(32)
            .try_to_owned()
            .map_err(|e| AppError::internal(format!("Failed to convert value to owned: {}", e)))?;
        menu_proxy
            .event(
                id,
                "clicked",
                &value,
                chrono::offset::Local::now().timestamp_subsec_micros()
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to trigger menu event: {}", e)))?;

        let (_, layout) = menu_proxy
            .get_layout(0, -1, &[])
            .await
            .map_err(|e| AppError::internal(format!("Failed to get menu layout: {}", e)))?;

        Ok(layout)
    }

    pub fn prepare_command(&self, command: TrayCommand) -> Option<TrayCommandFuture> {
        match command {
            TrayCommand::MenuSelected(name, id) => {
                let menu = self.data.iter().find(|item| item.name == name)?;
                let proxy = menu.menu_proxy.clone();
                let tray_name = menu.name.clone();

                Some(Box::pin(async move {
                    debug!("Click tray menu voice {tray_name} : {id}");
                    match TrayService::menu_voice_selected(&proxy, id).await {
                        Ok(new_layout) => ServiceEvent::Update(TrayEvent::MenuLayoutChanged(
                            tray_name, new_layout
                        )),
                        Err(err) => {
                            error!("Failed to execute tray command: {err}");
                            ServiceEvent::Update(TrayEvent::None)
                        }
                    }
                }))
            }
        }
    }
}

impl ReadOnlyService for TrayService {
    type UpdateEvent = TrayEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            TrayEvent::Registered(new_item) => {
                match self
                    .data
                    .0
                    .iter_mut()
                    .find(|item| item.name == new_item.name)
                {
                    Some(existing_item) => {
                        *existing_item = new_item;
                    }
                    _ => {
                        self.data.0.push(new_item);
                    }
                }
            }
            TrayEvent::IconChanged(name, handle) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    item.icon = Some(handle);
                }
            }
            TrayEvent::MenuLayoutChanged(name, layout) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    debug!("menu layout updated, {layout:?}");
                    item.menu = layout;
                }
            }
            TrayEvent::Unregistered(name) => {
                self.data.0.retain(|item| item.name != name);
            }
            TrayEvent::None => {}
        }
    }

    fn subscribe() -> iced::Subscription<ServiceEvent<Self>> {
        iced::Subscription::none()
    }
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    MenuSelected(String, i32)
}

type TrayCommandFuture = Pin<Box<dyn Future<Output = ServiceEvent<TrayService>> + Send + 'static>>;

impl Service for TrayService {
    type Command = TrayCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        self.prepare_command(command)
            .map(|future| Task::perform(future, |event| event))
            .unwrap_or_else(Task::none)
    }
}
