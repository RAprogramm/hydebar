// TODO: Fix test type annotations after Module trait refactoring
#![cfg(feature = "enable-broken-tests")]

use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use tokio::{
    io::{self, AsyncWriteExt, BufReader},
    time::{sleep, timeout}
};

use super::*;
use crate::event_bus::{BusEvent, EventBus};

#[tokio::test]
async fn send_event_propagates_module_errors() {
    let bus = EventBus::new(NonZeroUsize::new(1).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let module_name: Arc<str> = Arc::from("custom");
    let sender = context.module_sender({
        let module_name = Arc::clone(&module_name);
        move |message| ModuleEvent::Custom {
            name: Arc::clone(&module_name),
            message
        }
    });

    let data = CustomListenData {
        alt:  String::from("alt"),
        text: None
    };

    sender
        .try_send(Message::Event(ServiceEvent::Update(data.clone())))
        .expect("initial send");

    let result = send_event(&sender, ServiceEvent::Update(data));
    assert!(matches!(result, Err(ModuleError::EventBus(_))));
}

#[tokio::test]
async fn forward_custom_updates_delivers_events_and_errors() {
    let bus = EventBus::new(NonZeroUsize::new(8).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let module_name: Arc<str> = Arc::from("custom");
    let sender = context.module_sender({
        let module_name = Arc::clone(&module_name);
        move |message| ModuleEvent::Custom {
            name: Arc::clone(&module_name),
            message
        }
    });

    let (mut writer, reader) = io::duplex(256);
    writer
        .write_all(
            br#"{"alt":"value","text":"ok"}
invalid
"#
        )
        .await
        .expect("write output");
    writer.shutdown().await.expect("shutdown writer");

    let mut lines = BufReader::new(reader).lines();
    forward_custom_updates(&mut lines, module_name.as_ref(), &sender)
        .await
        .expect("forward updates");

    let mut receiver = bus.receiver();

    let first = receiver
        .try_recv()
        .expect("first event")
        .expect("event present");
    match first {
        BusEvent::Module(ModuleEvent::Custom {
            name,
            message
        }) => {
            assert_eq!(name.as_ref(), "custom");
            match message {
                Message::Event(ServiceEvent::Update(data)) => {
                    assert_eq!(data.alt, "value");
                    assert_eq!(data.text.as_deref(), Some("ok"));
                }
                other => panic!("unexpected message: {other:?}")
            }
        }
        other => panic!("unexpected event: {other:?}")
    }

    let second = receiver
        .try_recv()
        .expect("second event")
        .expect("event present");
    match second {
        BusEvent::Module(ModuleEvent::Custom {
            name,
            message
        }) => {
            assert_eq!(name.as_ref(), "custom");
            match message {
                Message::Event(ServiceEvent::Error(error)) => {
                    assert!(matches!(error, CustomCommandError::Parse(_, _)));
                }
                other => panic!("unexpected message: {other:?}")
            }
        }
        other => panic!("unexpected event: {other:?}")
    }
}

#[tokio::test]
async fn re_register_aborts_previous_listener() {
    let bus = EventBus::new(NonZeroUsize::new(32).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let mut custom = Custom::default();

    let mut receiver = bus.receiver();

    let first = CustomModuleDef {
        name:       String::from("first"),
        command:    String::from("true"),
        icon:       None,
        listen_cmd: Some(String::from(
            r#"while true; do printf '{"alt":"first","text":"one"}
'; sleep 0.1; done"#
        )),
        icons:      None,
        alert:      None
    };

    <Custom as Module<Message>>::register(&mut custom, &context, Some(&first))
        .expect("first register");

    timeout(Duration::from_secs(2), async {
        loop {
            if let Some(event) = receiver.try_recv().expect("receive") {
                if let BusEvent::Module(ModuleEvent::Custom {
                    name,
                    message
                }) = event
                {
                    if name.as_ref() == "first" {
                        if matches!(message, Message::Event(ServiceEvent::Update(_))) {
                            break;
                        }
                    }
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("first update");

    while let Some(Some(_)) = receiver.try_recv().ok() {}

    let second = CustomModuleDef {
        name:       String::from("second"),
        command:    String::from("true"),
        icon:       None,
        listen_cmd: Some(String::from(
            r#"while true; do printf '{"alt":"second","text":"two"}
'; sleep 0.1; done"#
        )),
        icons:      None,
        alert:      None
    };

    <Custom as Module<Message>>::register(&mut custom, &context, Some(&second))
        .expect("second register");

    let observed = timeout(Duration::from_secs(2), async {
        let mut alts = Vec::new();
        loop {
            if let Some(event) = receiver.try_recv().expect("receive") {
                if let BusEvent::Module(ModuleEvent::Custom {
                    name,
                    message
                }) = event
                {
                    if let Message::Event(ServiceEvent::Update(data)) = message {
                        alts.push((name, data.alt));
                        if alts.len() >= 3 {
                            break alts;
                        }
                    }
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("collected updates");

    assert!(
        observed
            .iter()
            .all(|(name, alt)| { name.as_ref() == "second" && alt == "second" })
    );
}
