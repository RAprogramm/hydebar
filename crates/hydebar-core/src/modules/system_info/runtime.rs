use std::time::Duration;

use log::error;
use tokio::{
    task::JoinHandle,
    time::{MissedTickBehavior, interval},
};

use super::Message;
use crate::{ModuleContext, ModuleEventSender};

/// Interval between system information refresh ticks.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5,);

/// Manages the background polling task responsible for refreshing system
/// metrics.
#[derive(Default,)]
pub struct PollingTask
{
    handle: Option<JoinHandle<(),>,>,
}

impl PollingTask
{
    /// Create a new polling task manager with no active background work.
    pub fn new() -> Self
    {
        Self {
            handle: None,
        }
    }

    /// Abort any in-flight polling task.
    pub fn abort(&mut self,)
    {
        if let Some(handle,) = self.handle.take() {
            handle.abort();
        }
    }

    /// Spawn a periodic refresh loop bound to the provided runtime context.
    pub fn spawn(&mut self, ctx: &ModuleContext, sender: ModuleEventSender<Message,>,)
    {
        self.abort();

        let handle = ctx.runtime_handle().spawn(async move {
            let mut ticker = interval(REFRESH_INTERVAL,);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay,);
            let _ = ticker.tick().await;

            loop {
                ticker.tick().await;

                if let Err(err,) = sender.try_send(Message::Update,) {
                    error!("failed to publish system info refresh: {err}");
                }
            }
        },);

        self.handle = Some(handle,);
    }
}

impl Drop for PollingTask
{
    fn drop(&mut self,)
    {
        self.abort();
    }
}

#[cfg(test)]
mod tests
{
    use std::num::NonZeroUsize;

    use tokio::{task::yield_now, time::advance};

    use super::*;
    use crate::{
        ModuleContext,
        event_bus::{BusEvent, EventBus, ModuleEvent},
        modules::system_info::Message,
    };

    fn module_context() -> (ModuleContext, EventBus,)
    {
        let capacity = NonZeroUsize::new(16,).expect("non-zero capacity",);
        let bus = EventBus::new(capacity,);
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current(),);

        (ctx, bus,)
    }

    fn expect_system_info_update(event: Option<BusEvent,>,)
    {
        match event {
            Some(BusEvent::Module(ModuleEvent::SystemInfo(Message::Update,),),) => {}
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn schedules_periodic_refreshes()
    {
        let (ctx, bus,) = module_context();
        let mut polling = PollingTask::default();
        let mut receiver = bus.receiver();

        let sender = ctx.module_sender(ModuleEvent::SystemInfo,);
        polling.spawn(&ctx, sender,);
        yield_now().await;

        assert!(receiver.try_recv().expect("initial queue state").is_none());

        advance(REFRESH_INTERVAL,).await;
        yield_now().await;

        let event = receiver.try_recv().expect("queued refresh after interval",);
        expect_system_info_update(event,);
    }

    #[tokio::test(start_paused = true)]
    async fn respawn_replaces_previous_task()
    {
        let (ctx, bus,) = module_context();
        let mut polling = PollingTask::default();
        let mut receiver = bus.receiver();

        let sender = ctx.module_sender(ModuleEvent::SystemInfo,);
        polling.spawn(&ctx, sender.clone(),);
        yield_now().await;

        advance(REFRESH_INTERVAL,).await;
        yield_now().await;

        let first = receiver.try_recv().expect("first refresh after interval",);
        expect_system_info_update(first,);
        assert!(receiver.try_recv().expect("drain first interval").is_none());

        polling.spawn(&ctx, sender,);
        yield_now().await;

        advance(REFRESH_INTERVAL,).await;
        yield_now().await;

        let second = receiver.try_recv().expect("refresh after respawn",);
        expect_system_info_update(second,);
        assert!(receiver.try_recv().expect("no duplicate refresh").is_none());
    }
}
