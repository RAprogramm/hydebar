use std::{any::TypeId, process::Stdio, sync::Arc};

use crate::{
    ModuleContext,
    app::{self},
    components::icons::{Icons, icon, icon_raw},
    config::CustomModuleDef,
    services::ServiceEvent,
};
use iced::futures::channel::mpsc::Sender;
use iced::widget::canvas;
use iced::{
    Element, Length, Subscription, Theme,
    stream::channel,
    widget::{Stack, row, text},
};
use iced::{
    mouse::Cursor,
    widget::{
        canvas::{Cache, Geometry, Path, Program},
        container,
    },
};
use log::{error, info, warn};
use masterror::Error;
use serde::Deserialize;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader, Lines},
    process::Command,
    task::yield_now,
};

use super::{Module, ModuleError, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Custom {
    data: CustomListenData,
    last_error: Option<CustomCommandError>,
    registration: Option<CustomRegistration>,
}

#[derive(Debug, Clone)]
struct CustomRegistration {
    name: Arc<str>,
    listen_command: Arc<str>,
}

impl Custom {
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

#[derive(Debug, Clone, Error)]
pub enum CustomCommandError {
    #[error("failed to spawn custom module listener process: {0}")]
    Spawn(Arc<std::io::Error>),
    #[error("custom module listener did not expose stdout")]
    MissingStdout,
    #[error("failed to read line from custom module output: {0}")]
    Read(Arc<std::io::Error>),
    #[error("failed to parse custom module output: {0} ({1})")]
    Parse(String, #[source] Arc<serde_json::Error>),
    #[error("failed to wait for custom module process: {0}")]
    Wait(Arc<std::io::Error>),
    #[error("custom module process exited unsuccessfully ({status:?})")]
    NonZeroExit { status: Option<i32> },
    #[error("custom module updates channel closed")]
    ChannelClosed,
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

#[derive(Debug)]
enum SendQueueError {
    Full(Box<app::Message>),
    Closed,
}

trait CustomUpdateSender {
    fn try_send(&mut self, message: app::Message) -> Result<(), SendQueueError>;
}

impl CustomUpdateSender for Sender<app::Message> {
    fn try_send(&mut self, message: app::Message) -> Result<(), SendQueueError> {
        Sender::try_send(self, message).map_err(|error| {
            if error.is_full() {
                SendQueueError::Full(Box::new(error.into_inner()))
            } else {
                let _ = error.into_inner();
                SendQueueError::Closed
            }
        })
    }
}

async fn send_event<S: CustomUpdateSender + Send>(
    sender: &mut S,
    module_name: &str,
    event: ServiceEvent<CustomCommandService>,
) -> Result<(), CustomCommandError> {
    let mut message = app::Message::CustomUpdate(module_name.to_owned(), Message::Event(event));

    loop {
        match sender.try_send(message) {
            Ok(()) => return Ok(()),
            Err(SendQueueError::Full(pending_message)) => {
                warn!("Custom module output channel full; yielding before retrying");
                message = *pending_message;
                yield_now().await;
            }
            Err(SendQueueError::Closed) => {
                return Err(CustomCommandError::ChannelClosed);
            }
        }
    }
}

async fn forward_custom_updates<R, S>(
    reader: &mut Lines<R>,
    module_name: &str,
    sender: &mut S,
) -> Result<(), CustomCommandError>
where
    R: AsyncBufRead + Unpin,
    S: CustomUpdateSender + Send,
{
    while let Some(line) = reader
        .next_line()
        .await
        .map_err(|err| CustomCommandError::Read(Arc::new(err)))?
    {
        match serde_json::from_str::<CustomListenData>(&line) {
            Ok(event) => {
                send_event(sender, module_name, ServiceEvent::Update(event)).await?;
            }
            Err(err) => {
                let parse_error = CustomCommandError::Parse(truncate_snippet(&line), Arc::new(err));
                error!(
                    "Custom module '{module_name}' failed to parse JSON output: {parse_error:?}"
                );
                send_event(
                    sender,
                    module_name,
                    ServiceEvent::Error(parse_error.clone()),
                )
                .await?;
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

impl Module for Custom {
    type ViewData<'a> = &'a CustomModuleDef;
    type RegistrationData<'a> = Option<&'a CustomModuleDef>;

    fn register(
        &mut self,
        _: &ModuleContext,
        config: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        self.registration = config.and_then(|definition| {
            definition
                .listen_cmd
                .as_ref()
                .map(|command| CustomRegistration {
                    name: Arc::from(definition.name.as_str()),
                    listen_command: Arc::from(command.as_str()),
                })
        });

        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
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

        Some((
            row_content,
            Some(OnModulePress::Action(Box::new(
                app::Message::LaunchCommand(config.command.clone()),
            ))),
        ))
    }

    fn subscription(&self) -> Option<Subscription<app::Message>> {
        let registration = self.registration.as_ref()?;
        let id = TypeId::of::<Self>();
        let identifier = format!("{id:?}-{}", registration.name);
        let module_name = Arc::clone(&registration.name);
        let listen_command = Arc::clone(&registration.listen_command);

        Some(Subscription::run_with_id(
            identifier,
            channel(10, move |mut output| {
                let module_name = Arc::clone(&module_name);
                let listen_command = Arc::clone(&listen_command);

                async move {
                    let module_label = module_name.as_ref();
                    if let Err(error) =
                        run_custom_listener(module_label, listen_command.as_ref(), &mut output)
                            .await
                    {
                        error!(
                            "Custom module '{module_label}' listener terminated with error: {error:?}"
                        );
                        if !matches!(error, CustomCommandError::ChannelClosed) {
                            let _ = send_event(
                                &mut output,
                                module_label,
                                ServiceEvent::Error(error.clone()),
                            )
                            .await;
                        }
                    }
                }
            }),
        ))
    }
}

async fn run_custom_listener<S: CustomUpdateSender + Send>(
    module_name: &str,
    command: &str,
    sender: &mut S,
) -> Result<(), CustomCommandError> {
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| CustomCommandError::Spawn(Arc::new(err)))?;

    let stdout = child
        .stdout
        .take()
        .ok_or(CustomCommandError::MissingStdout)?;

    let mut reader = BufReader::new(stdout).lines();

    forward_custom_updates(&mut reader, module_name, sender).await?;

    match child.wait().await {
        Ok(status) => {
            info!("Custom module '{module_name}' listener exited with status: {status}");
            if status.success() {
                Ok(())
            } else {
                Err(CustomCommandError::NonZeroExit {
                    status: status.code(),
                })
            }
        }
        Err(err) => Err(CustomCommandError::Wait(Arc::new(err))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::futures::channel::mpsc;
    use std::time::Duration;
    use tokio::{
        io::{self, AsyncWriteExt},
        time::timeout,
    };

    #[derive(Default)]
    struct RecordingSender {
        messages: Vec<app::Message>,
    }

    impl CustomUpdateSender for RecordingSender {
        fn try_send(&mut self, message: app::Message) -> Result<(), SendQueueError> {
            self.messages.push(message);
            Ok(())
        }
    }

    struct ClosedSender {
        sender: Sender<app::Message>,
    }

    impl ClosedSender {
        fn new() -> Self {
            let (sender, receiver) = mpsc::channel(1);
            drop(receiver);
            ClosedSender { sender }
        }
    }

    impl Default for ClosedSender {
        fn default() -> Self {
            ClosedSender::new()
        }
    }

    impl CustomUpdateSender for ClosedSender {
        fn try_send(&mut self, message: app::Message) -> Result<(), SendQueueError> {
            Sender::try_send(&mut self.sender, message).map_err(|error| {
                if error.is_full() {
                    SendQueueError::Full(Box::new(error.into_inner()))
                } else {
                    let _ = error.into_inner();
                    SendQueueError::Closed
                }
            })
        }
    }

    #[tokio::test]
    async fn handles_early_exit_and_closed_channel() {
        let (mut writer, reader) = io::duplex(64);
        timeout(Duration::from_secs(1), async {
            writer
                .write_all(br#"{"alt":"value","text":"ok"}\n"#)
                .await
                .expect("write output");
            writer.shutdown().await.expect("shutdown writer");
        })
        .await
        .expect("writer future completed");

        let mut closed_sender = ClosedSender::default();
        let mut lines = BufReader::new(reader).lines();
        let result = timeout(
            Duration::from_secs(1),
            forward_custom_updates(&mut lines, "test", &mut closed_sender),
        )
        .await
        .expect("forward future completed");
        assert!(matches!(result, Err(CustomCommandError::ChannelClosed)));

        let (writer, reader) = io::duplex(64);
        drop(writer);

        let mut recording_sender = RecordingSender::default();
        let mut lines = BufReader::new(reader).lines();
        let result = timeout(
            Duration::from_secs(1),
            forward_custom_updates(&mut lines, "test", &mut recording_sender),
        )
        .await
        .expect("forward future completed");

        assert!(result.is_ok());
        assert!(recording_sender.messages.is_empty());
    }
}
