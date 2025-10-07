use std::{process::Stdio, sync::Arc};

use crate::{
    ModuleContext, ModuleEventSender,
    components::icons::{Icons, icon, icon_raw},
    config::CustomModuleDef,
    event_bus::ModuleEvent,
    services::ServiceEvent,
};
use iced::widget::canvas;
use iced::{
    Element, Length, Subscription, Theme,
    widget::{Stack, row, text},
};
use iced::{
    mouse::Cursor,
    widget::{
        canvas::{Cache, Geometry, Path, Program},
        container,
    },
};
use log::{error, info};
use masterror::Error;
use serde::Deserialize;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader, Lines},
    process::Command,
    task::JoinHandle,
};

use super::{Module, ModuleError, OnModulePress};

#[derive(Default, Debug)]
pub struct Custom {
    data: CustomListenData,
    last_error: Option<CustomCommandError>,
    registration: Option<CustomRegistration>,
    sender: Option<ModuleEventSender<Message>>,
    listener_task: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
struct CustomRegistration {
    name: Arc<str>,
    listen_command: Arc<str>,
}

impl Custom {
    fn abort_listener(&mut self) {
        if let Some(handle) = self.listener_task.take() {
            handle.abort();
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Event(ServiceEvent::Update(data)) => {
                self.data = data;
                self.last_error = None;
            }
            Message::Event(ServiceEvent::Error(error)) => {
                self.last_error = Some(error);
            }
            Message::Event(ServiceEvent::Init(_)) => {}
        }
    }
}

impl Drop for Custom {
    fn drop(&mut self) {
        self.abort_listener();
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CustomListenData {
    pub alt: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<CustomCommandService>),
}

#[derive(Debug, Clone, Default)]
pub struct CustomCommandService;

impl crate::services::ReadOnlyService for CustomCommandService {
    type UpdateEvent = CustomListenData;
    type Error = CustomCommandError;

    fn update(&mut self, _event: Self::UpdateEvent) {}

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::none()
    }
}

#[derive(Debug, Clone)]
pub enum CustomCommandError {
    Spawn(Arc<std::io::Error>),
    MissingStdout,
    Read(Arc<std::io::Error>),
    Parse(String, Arc<serde_json::Error>),
    Wait(Arc<std::io::Error>),
    NonZeroExit { status: Option<i32> },
    ChannelClosed,
}

impl std::fmt::Display for CustomCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(err) => write!(f, "failed to spawn custom module listener process: {}", err),
            Self::MissingStdout => write!(f, "custom module listener did not expose stdout"),
            Self::Read(err) => write!(f, "failed to read line from custom module output: {}", err),
            Self::Parse(snippet, err) => write!(f, "failed to parse custom module output: {} ({})", snippet, err),
            Self::Wait(err) => write!(f, "failed to wait for custom module process: {}", err),
            Self::NonZeroExit { status } => write!(f, "custom module process exited unsuccessfully ({:?})", status),
            Self::ChannelClosed => write!(f, "custom module updates channel closed"),
        }
    }
}

impl std::error::Error for CustomCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(err) => Some(err.as_ref()),
            Self::Read(err) => Some(err.as_ref()),
            Self::Parse(_, err) => Some(err.as_ref()),
            Self::Wait(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl CustomCommandError {
    fn to_display_message(&self) -> String {
        match self {
            CustomCommandError::Parse(snippet, ..) => {
                format!("Invalid output: {snippet}")
            }
            CustomCommandError::NonZeroExit { status } => match status {
                Some(code) => format!("Listener exited with status {code}"),
                None => String::from("Listener exited due to signal"),
            },
            CustomCommandError::ChannelClosed => String::from("Listener updates queue closed"),
            CustomCommandError::MissingStdout => String::from("Listener stdout unavailable"),
            CustomCommandError::Spawn(_)
            | CustomCommandError::Read(_)
            | CustomCommandError::Wait(_) => String::from("Listener IO failure"),
        }
    }
}

fn truncate_snippet(line: &str) -> String {
    const MAX_LEN: usize = 120;

    if line.len() <= MAX_LEN {
        return line.to_owned();
    }

    let mut truncated = String::with_capacity(MAX_LEN + 1);
    for (idx, ch) in line.char_indices() {
        if idx >= MAX_LEN {
            truncated.push('…');
            break;
        }
        truncated.push(ch);
    }
    truncated
}

#[derive(Debug, Clone)]
enum CustomListenerError {
    Command(CustomCommandError),
    Module(ModuleError),
}

impl std::fmt::Display for CustomListenerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(err) => write!(f, "{}", err),
            Self::Module(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for CustomListenerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Command(err) => Some(err),
            Self::Module(err) => Some(err),
        }
    }
}

fn send_event(
    sender: &ModuleEventSender<Message>,
    event: ServiceEvent<CustomCommandService>,
) -> Result<(), ModuleError> {
    sender
        .try_send(Message::Event(event))
        .map_err(ModuleError::from)
}

async fn forward_custom_updates<R>(
    reader: &mut Lines<R>,
    module_name: &str,
    sender: &ModuleEventSender<Message>,
) -> Result<(), CustomListenerError>
where
    R: AsyncBufRead + Unpin,
{
    while let Some(line) = reader
        .next_line()
        .await
        .map_err(|err| CustomListenerError::Command(CustomCommandError::Read(Arc::new(err))))?
    {
        match serde_json::from_str::<CustomListenData>(&line) {
            Ok(event) => {
                send_event(sender, ServiceEvent::Update(event))
                    .map_err(CustomListenerError::Module)?;
            }
            Err(err) => {
                let parse_error = CustomCommandError::Parse(truncate_snippet(&line), Arc::new(err));
                error!(
                    "Custom module '{module_name}' failed to parse JSON output: {parse_error:?}"
                );
                send_event(sender, ServiceEvent::Error(parse_error.clone()))
                    .map_err(CustomListenerError::Module)?;
            }
        }
    }

    Ok(())
}

// Define a struct for the canvas program
#[derive(Debug, Clone, Copy, Default)]
struct AlertIndicator;

impl<Message> Program<Message> for AlertIndicator {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let cache = Cache::new(); // Use a local cache for simplicity here

        vec![cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            // Use a smaller radius so the circle doesn't touch the canvas edges
            let radius = 2.0; // Creates a 4px diameter circle
            let circle = Path::circle(center, radius);
            frame.fill(&circle, theme.palette().danger);
        })]
    }
}

impl<M> Module<M> for Custom {
    type ViewData<'a> = &'a CustomModuleDef;
    type RegistrationData<'a> = Option<&'a CustomModuleDef>;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        self.abort_listener();
        self.sender = None;
        self.last_error = None;
        self.registration = config.and_then(|definition| {
            definition
                .listen_cmd
                .as_ref()
                .map(|command| CustomRegistration {
                    name: Arc::from(definition.name.as_str()),
                    listen_command: Arc::from(command.as_str()),
                })
        });

        let Some(registration) = self.registration.clone() else {
            return Ok(());
        };

        let module_name_for_sender = Arc::clone(&registration.name);
        let sender = ctx.module_sender(move |message| ModuleEvent::Custom {
            name: Arc::clone(&module_name_for_sender),
            message,
        });

        self.sender = Some(sender.clone());
        let module_name_for_task = Arc::clone(&registration.name);
        let listen_command = Arc::clone(&registration.listen_command);
        let error_sender = sender.clone();
        let runtime_handle = ctx.runtime_handle().clone();

        self.listener_task = Some(runtime_handle.spawn(async move {
            match run_custom_listener(module_name_for_task.clone(), listen_command, sender).await {
                Ok(()) => {}
                Err(CustomListenerError::Command(error)) => {
                    error!(
                        "Custom module '{}' listener terminated with error: {error:?}",
                        module_name_for_task
                    );

                    if !matches!(error, CustomCommandError::ChannelClosed) {
                        if let Err(send_error) =
                            send_event(&error_sender, ServiceEvent::Error(error.clone()))
                        {
                            error!(
                                "Custom module '{}' failed to publish error notification: {send_error}",
                                module_name_for_task
                            );
                        }
                    }
                }
                Err(CustomListenerError::Module(error)) => {
                    error!(
                        "Custom module '{}' failed to publish event: {error}",
                        module_name_for_task
                    );
                }
            }
        }));

        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + Clone,
    {
        let mut icon_element = config
            .icon
            .as_ref()
            .map_or_else(|| icon(Icons::None), |text| icon_raw(text.clone()));

        if let Some(icons_map) = &config.icons {
            for (re, icon_str) in icons_map {
                if re.is_match(&self.data.alt) {
                    icon_element = icon_raw(icon_str.clone());
                    break; // Use the first match
                }
            }
        }

        // Wrap the icon in a container to apply padding
        let padded_icon_container = container(icon_element).padding([0, 1]);

        let mut show_alert = false;
        if let Some(re) = &config.alert {
            if re.is_match(&self.data.alt) {
                show_alert = true;
            }
        }

        if self.last_error.is_some() {
            show_alert = true;
        }

        let icon_with_alert = if show_alert {
            let alert_canvas = canvas(AlertIndicator)
                .width(Length::Fixed(5.0)) // Size of the dot
                .height(Length::Fixed(5.0));

            // Container to position the dot at the top-right
            let alert_indicator_container = container(alert_canvas)
                .width(Length::Fill) // Take full width of the stack item
                .height(Length::Fill) // Take full height
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top);
            // Optional: Add padding to nudge it slightly
            // .padding([2, 2, 0, 0]); // top, right, bottom, left

            Stack::new()
                .push(padded_icon_container) // Padded icon is the base layer
                .push(alert_indicator_container) // Dot container on top
                .into()
        } else {
            padded_icon_container.into() // No alert, just the padded icon
        };

        let maybe_text_element = if let Some(error) = &self.last_error {
            Some(text(error.to_display_message()))
        } else {
            self.data.text.as_ref().and_then(|text_content| {
                if !text_content.is_empty() {
                    Some(text(text_content.clone()))
                } else {
                    None
                }
            })
        };

        let row_content = if let Some(text_element) = maybe_text_element {
            row![icon_with_alert, text_element].spacing(8).into()
        } else {
            icon_with_alert
        };

        // NOTE: This returns None for action since we can't construct M in generic code.
        // The GUI layer should handle command launching based on module configuration.
        Some((row_content, None))
    }
}

async fn run_custom_listener(
    module_name: Arc<str>,
    command: Arc<str>,
    sender: ModuleEventSender<Message>,
) -> Result<(), CustomListenerError> {
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(command.as_ref())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| CustomListenerError::Command(CustomCommandError::Spawn(Arc::new(err))))?;

    let stdout = child.stdout.take().ok_or(CustomListenerError::Command(
        CustomCommandError::MissingStdout,
    ))?;

    let mut reader = BufReader::new(stdout).lines();

    forward_custom_updates(&mut reader, module_name.as_ref(), &sender).await?;

    match child.wait().await {
        Ok(status) => {
            info!("Custom module '{module_name}' listener exited with status: {status}");
            if status.success() {
                Ok(())
            } else {
                Err(CustomListenerError::Command(
                    CustomCommandError::NonZeroExit {
                        status: status.code(),
                    },
                ))
            }
        }
        Err(err) => Err(CustomListenerError::Command(CustomCommandError::Wait(
            Arc::new(err),
        ))),
    }
}

#[cfg(test)]
mod tests;
