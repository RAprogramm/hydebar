use std::process::Command;

use iced::{
    Alignment, Element,
    widget::{Column, Row, button, container, text}
};
use log::{debug, error};

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext,
    components::icons::{Icons, icon},
    menu::MenuType
};

/// Screenshot action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotAction {
    Area,
    Window,
    Fullscreen
}

/// Message emitted by the screenshot module.
#[derive(Debug, Clone)]
pub enum ScreenshotMessage {
    TakeScreenshot(ScreenshotAction),
    StartRecording,
    StopRecording
}

/// Screenshot and recording module.
#[derive(Debug, Default)]
pub struct Screenshot {
    pub is_recording: bool
}

impl Screenshot {
    /// Take a screenshot with the specified action.
    pub fn take_screenshot(&self, action: ScreenshotAction) {
        let screenshot_dir = dirs::picture_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("Screenshots");

        // Create directory if it doesn't exist
        if let Err(err) = std::fs::create_dir_all(&screenshot_dir) {
            error!("Failed to create screenshots directory: {err}");
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = screenshot_dir.join(format!("screenshot_{}.png", timestamp,));

        let result = match action {
            ScreenshotAction::Area => {
                // Use slurp to select area, then grim to capture
                debug!("Taking area screenshot");
                let slurp_output = Command::new("slurp").output();

                match slurp_output {
                    Ok(output) if output.status.success() => {
                        let geometry = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        Command::new("grim")
                            .arg("-g")
                            .arg(geometry)
                            .arg(&filename)
                            .spawn()
                    }
                    Ok(_) => {
                        debug!("Slurp cancelled by user");
                        return;
                    }
                    Err(err) => {
                        error!("Failed to run slurp: {err}");
                        return;
                    }
                }
            }
            ScreenshotAction::Window => {
                // TODO: Get active window geometry from Hyprland
                debug!("Taking window screenshot (fullscreen for now)");
                Command::new("grim").arg(&filename).spawn()
            }
            ScreenshotAction::Fullscreen => {
                debug!("Taking fullscreen screenshot");
                Command::new("grim").arg(&filename).spawn()
            }
        };

        match result {
            Ok(_) => {
                debug!("Screenshot saved to: {}", filename.display());
                // TODO: Send notification
            }
            Err(err) => error!("Failed to take screenshot: {err}")
        }
    }

    /// Start screen recording.
    pub fn start_recording(&mut self) {
        if self.is_recording {
            error!("Recording already in progress");
            return;
        }

        let video_dir = dirs::video_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("Recordings");

        // Create directory if it doesn't exist
        if let Err(err) = std::fs::create_dir_all(&video_dir) {
            error!("Failed to create recordings directory: {err}");
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = video_dir.join(format!("recording_{}.mp4", timestamp,));

        debug!("Starting recording to: {}", filename.display());

        match Command::new("wf-recorder").arg("-f").arg(&filename).spawn() {
            Ok(_) => {
                self.is_recording = true;
                debug!("Recording started");
                // TODO: Send notification
            }
            Err(err) => error!("Failed to start recording: {err}")
        }
    }

    /// Stop screen recording.
    pub fn stop_recording(&mut self) {
        if !self.is_recording {
            error!("No recording in progress");
            return;
        }

        debug!("Stopping recording");

        // Send SIGINT to wf-recorder to stop recording gracefully
        match Command::new("pkill")
            .arg("-SIGINT")
            .arg("wf-recorder")
            .spawn()
        {
            Ok(_) => {
                self.is_recording = false;
                debug!("Recording stopped");
                // TODO: Send notification
            }
            Err(err) => error!("Failed to stop recording: {err}")
        }
    }

    /// Update the module state based on messages.
    pub fn update(&mut self, message: ScreenshotMessage) {
        match message {
            ScreenshotMessage::TakeScreenshot(action) => {
                self.take_screenshot(action);
            }
            ScreenshotMessage::StartRecording => {
                self.start_recording();
            }
            ScreenshotMessage::StopRecording => {
                self.stop_recording();
            }
        }
    }

    /// Render screenshot actions menu.
    pub fn menu_view(&self, _opacity: f32) -> Element<'_, ScreenshotMessage> {
        let mut content = Column::new().spacing(8).padding(12);

        // Screenshot section
        content = content.push(text("Screenshot").size(16));

        let screenshot_buttons = Column::new()
            .push(
                button(
                    Row::new()
                        .push(text("📐 Select Area"))
                        .spacing(8)
                        .align_y(Alignment::Center)
                )
                .on_press(ScreenshotMessage::TakeScreenshot(ScreenshotAction::Area))
                .width(iced::Length::Fill)
            )
            .push(
                button(
                    Row::new()
                        .push(text("🪟 Current Window"))
                        .spacing(8)
                        .align_y(Alignment::Center)
                )
                .on_press(ScreenshotMessage::TakeScreenshot(ScreenshotAction::Window))
                .width(iced::Length::Fill)
            )
            .push(
                button(
                    Row::new()
                        .push(text("🖥️ Fullscreen"))
                        .spacing(8)
                        .align_y(Alignment::Center)
                )
                .on_press(ScreenshotMessage::TakeScreenshot(
                    ScreenshotAction::Fullscreen
                ))
                .width(iced::Length::Fill)
            )
            .spacing(4);

        content = content.push(screenshot_buttons);

        // Recording section
        content = content.push(text("Recording").size(16));

        let recording_button = if self.is_recording {
            button(
                Row::new()
                    .push(text("⏹️ Stop Recording"))
                    .spacing(8)
                    .align_y(Alignment::Center)
            )
            .on_press(ScreenshotMessage::StopRecording)
            .width(iced::Length::Fill)
        } else {
            button(
                Row::new()
                    .push(text("🔴 Start Recording"))
                    .spacing(8)
                    .align_y(Alignment::Center)
            )
            .on_press(ScreenshotMessage::StartRecording)
            .width(iced::Length::Fill)
        };

        content = content.push(recording_button);

        container(content).into()
    }
}

impl<M> Module<M> for Screenshot
where
    M: 'static + Clone + From<ScreenshotMessage>
{
    type ViewData<'a> = ();
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        _: &ModuleContext,
        _: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Render camera icon with recording indicator.
    fn view(
        &self,
        _: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let content = if self.is_recording {
            Row::new()
                .push(icon(Icons::Point)) // Red dot for recording
                .push(text("📷"))
                .spacing(4)
                .align_y(Alignment::Center)
        } else {
            Row::new().push(text("📷"))
        };

        Some((
            container(content).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Screenshot))
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::event_bus::EventBus;

    #[test]
    fn default_creates_not_recording() {
        let screenshot = Screenshot::default();
        assert!(!screenshot.is_recording);
    }

    #[test]
    fn register_succeeds() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut screenshot = Screenshot::default();

        let result =
            <Screenshot as Module<ScreenshotMessage>>::register(&mut screenshot, &ctx, ());
        assert!(result.is_ok());
    }
}
