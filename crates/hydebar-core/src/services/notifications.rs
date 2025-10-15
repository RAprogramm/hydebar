use std::{collections::VecDeque, sync::Arc, time::SystemTime};

use iced::{Subscription, futures::SinkExt, stream};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use zbus::{Connection, interface};

use super::{ReadOnlyService, ServiceEvent};

const MAX_NOTIFICATIONS: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Urgency {
    Low = 0,
    Normal = 1,
    Critical = 2
}

impl From<u8> for Urgency {
    fn from(value: u8) -> Self {
        match value {
            0 => Urgency::Low,
            2 => Urgency::Critical,
            _ => Urgency::Normal
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id:        u32,
    pub app_name:  String,
    pub icon:      String,
    pub summary:   String,
    pub body:      String,
    pub urgency:   Urgency,
    pub timestamp: SystemTime,
    pub actions:   Vec<String>
}

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// New notification received
    Received(Notification),
    /// Notification closed/dismissed
    Closed(u32),
    /// Action invoked on notification
    ActionInvoked(u32, String)
}

#[derive(Debug, Clone)]
pub struct NotificationStorage {
    notifications:  VecDeque<Notification>,
    next_id:        u32,
    do_not_disturb: bool,
    sounds_enabled: bool
}

impl Default for NotificationStorage {
    fn default() -> Self {
        Self {
            notifications:  VecDeque::with_capacity(MAX_NOTIFICATIONS),
            next_id:        1,
            do_not_disturb: false,
            sounds_enabled: true
        }
    }
}

impl NotificationStorage {
    pub fn add(&mut self, mut notification: Notification) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        notification.id = id;

        // Keep only MAX_NOTIFICATIONS
        if self.notifications.len() >= MAX_NOTIFICATIONS {
            self.notifications.pop_back();
        }

        self.notifications.push_front(notification);
        id
    }

    pub fn remove(&mut self, id: u32) -> Option<Notification> {
        if let Some(pos) = self.notifications.iter().position(|n| n.id == id) {
            self.notifications.remove(pos)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }

    pub fn get_all(&self) -> &VecDeque<Notification> {
        &self.notifications
    }

    pub fn unread_count(&self) -> usize {
        self.notifications.len()
    }

    pub fn set_dnd(&mut self, enabled: bool) {
        self.do_not_disturb = enabled;
    }

    pub fn is_dnd(&self) -> bool {
        self.do_not_disturb
    }

    pub fn set_sounds(&mut self, enabled: bool) {
        self.sounds_enabled = enabled;
    }

    pub fn sounds_enabled(&self) -> bool {
        self.sounds_enabled
    }

    pub fn should_show(&self, urgency: &Urgency) -> bool {
        if self.do_not_disturb {
            // Critical notifications bypass DND
            matches!(urgency, Urgency::Critical)
        } else {
            true
        }
    }
}

/// D-Bus org.freedesktop.Notifications server implementation
pub struct NotificationsServer {
    storage: std::sync::Arc<std::sync::Mutex<NotificationStorage>>
}

impl NotificationsServer {
    pub fn new(storage: std::sync::Arc<std::sync::Mutex<NotificationStorage>>) -> Self {
        Self {
            storage
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationsServer {
    /// Get server information
    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        ("hydebar", "RAprogramm", "0.6.7", "1.2")
    }

    /// Get server capabilities
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-markup".to_string(),
            "actions".to_string(),
            "icon-static".to_string(),
        ]
    }

    /// Notify - main method for sending notifications
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &mut self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: std::collections::HashMap<String, zbus::zvariant::Value<'_>>,
        expire_timeout: i32
    ) -> u32 {
        debug!(
            "Notification: {} - {} (icon: {}, timeout: {})",
            app_name, summary, app_icon, expire_timeout
        );

        // Parse urgency from hints
        let urgency = hints
            .get("urgency")
            .and_then(|v| v.downcast_ref::<u8>().ok())
            .map(Urgency::from)
            .unwrap_or(Urgency::Normal);

        let notification = Notification {
            id: 0, // Will be set by storage
            app_name: app_name.clone(),
            icon: app_icon,
            summary: summary.clone(),
            body: body.clone(),
            urgency: urgency.clone(),
            timestamp: SystemTime::now(),
            actions
        };

        let mut storage = self.storage.lock().unwrap();

        // Check if should show (DND mode)
        if !storage.should_show(&urgency) {
            debug!("Notification suppressed by DND: {}", summary);
            return 0;
        }

        // Handle replaces_id
        let id = if replaces_id > 0 {
            storage.remove(replaces_id);
            replaces_id
        } else {
            storage.add(notification)
        };

        // Play sound if enabled
        if storage.sounds_enabled() {
            Self::play_notification_sound(&urgency);
        }

        id
    }

    /// Close notification
    fn close_notification(&mut self, id: u32) {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(id);
    }
}

impl NotificationsServer {
    fn play_notification_sound(urgency: &Urgency) {
        // Use libcanberra or aplay to play sound
        let sound_name = match urgency {
            Urgency::Critical => "message-new-urgent",
            Urgency::Normal => "message-new-instant",
            Urgency::Low => "message"
        };

        // Try canberra first (standard freedesktop sound system)
        std::process::Command::new("canberra-gtk-play")
            .args(["-i", sound_name, "-d", "New notification"])
            .spawn()
            .ok();
    }
}

/// Error types for NotificationsService
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationsError {
    DBusConnection(String),
    DBusInterface(String)
}

impl std::fmt::Display for NotificationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DBusConnection(msg) => write!(f, "D-Bus connection error: {}", msg),
            Self::DBusInterface(msg) => write!(f, "D-Bus interface error: {}", msg)
        }
    }
}

impl std::error::Error for NotificationsError {}

/// Main notifications service integrating with org.freedesktop.Notifications
#[derive(Debug, Clone, Default)]
pub struct NotificationsService {
    storage: Arc<std::sync::Mutex<NotificationStorage>>
}

impl NotificationsService {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(std::sync::Mutex::new(NotificationStorage::default()))
        }
    }

    pub fn get_notifications(&self) -> Vec<Notification> {
        self.storage
            .lock()
            .unwrap()
            .get_all()
            .iter()
            .cloned()
            .collect()
    }

    pub fn unread_count(&self) -> usize {
        self.storage.lock().unwrap().unread_count()
    }

    pub fn dismiss(&mut self, id: u32) {
        self.storage.lock().unwrap().remove(id);
    }

    pub fn clear_all(&mut self) {
        self.storage.lock().unwrap().clear();
    }

    pub fn toggle_dnd(&mut self) {
        let mut storage = self.storage.lock().unwrap();
        let current = storage.is_dnd();
        storage.set_dnd(!current);
    }

    pub fn is_dnd(&self) -> bool {
        self.storage.lock().unwrap().is_dnd()
    }
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationEvent;
    type Error = NotificationsError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            NotificationEvent::Received(notification) => {
                self.storage.lock().unwrap().add(notification);
            }
            NotificationEvent::Closed(id) => {
                self.storage.lock().unwrap().remove(id);
            }
            NotificationEvent::ActionInvoked(_, _) => {
                // Actions handling can be added later
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with_id(
            std::any::TypeId::of::<NotificationsService>(),
            stream::channel(100, |mut output| async move {
                // Initialize storage
                let storage = Arc::new(std::sync::Mutex::new(NotificationStorage::default()));
                let service = NotificationsService {
                    storage: Arc::clone(&storage)
                };

                // Send init event
                if output
                    .send(ServiceEvent::Init(service.clone()))
                    .await
                    .is_err()
                {
                    error!("Failed to send notifications service init event");
                    return;
                }

                // Connect to session bus
                let connection = match Connection::session().await {
                    Ok(conn) => conn,
                    Err(err) => {
                        error!("Failed to connect to D-Bus: {err}");
                        let _ = output
                            .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                                err.to_string()
                            )))
                            .await;
                        return;
                    }
                };

                // Create notifications server
                let server = NotificationsServer::new(Arc::clone(&storage));

                // Register D-Bus interface
                if let Err(err) = connection
                    .object_server()
                    .at("/org/freedesktop/Notifications", server)
                    .await
                {
                    error!("Failed to register D-Bus interface: {err}");
                    let _ = output
                        .send(ServiceEvent::Error(NotificationsError::DBusInterface(
                            err.to_string()
                        )))
                        .await;
                    return;
                }

                // Request well-known name
                if let Err(err) = connection
                    .request_name("org.freedesktop.Notifications")
                    .await
                {
                    error!("Failed to request D-Bus name: {err}");
                    let _ = output
                        .send(ServiceEvent::Error(NotificationsError::DBusConnection(
                            err.to_string()
                        )))
                        .await;
                    return;
                }

                debug!("Notifications D-Bus service registered");

                // Keep connection alive
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_max_capacity() {
        let mut storage = NotificationStorage::default();

        // Add MAX_NOTIFICATIONS + 10
        for i in 0..MAX_NOTIFICATIONS + 10 {
            let notif = Notification {
                id:        0,
                app_name:  format!("app{}", i),
                icon:      String::new(),
                summary:   format!("Summary {}", i),
                body:      String::new(),
                urgency:   Urgency::Normal,
                timestamp: SystemTime::now(),
                actions:   vec![]
            };
            storage.add(notif);
        }

        assert_eq!(storage.get_all().len(), MAX_NOTIFICATIONS);
    }

    #[test]
    fn dnd_blocks_normal_notifications() {
        let mut storage = NotificationStorage::default();
        storage.set_dnd(true);

        assert!(!storage.should_show(&Urgency::Normal));
        assert!(!storage.should_show(&Urgency::Low));
        assert!(storage.should_show(&Urgency::Critical));
    }

    #[test]
    fn remove_notification_by_id() {
        let mut storage = NotificationStorage::default();
        let notif = Notification {
            id:        0,
            app_name:  "test".to_string(),
            icon:      String::new(),
            summary:   "Test".to_string(),
            body:      String::new(),
            urgency:   Urgency::Normal,
            timestamp: SystemTime::now(),
            actions:   vec![]
        };

        let id = storage.add(notif);
        assert_eq!(storage.unread_count(), 1);

        storage.remove(id);
        assert_eq!(storage.unread_count(), 0);
    }

    #[test]
    fn clear_all_notifications() {
        let mut storage = NotificationStorage::default();

        for i in 0..5 {
            let notif = Notification {
                id:        0,
                app_name:  format!("app{}", i),
                icon:      String::new(),
                summary:   format!("Summary {}", i),
                body:      String::new(),
                urgency:   Urgency::Normal,
                timestamp: SystemTime::now(),
                actions:   vec![]
            };
            storage.add(notif);
        }

        assert_eq!(storage.unread_count(), 5);
        storage.clear();
        assert_eq!(storage.unread_count(), 0);
    }
}
