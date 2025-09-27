#![allow(mismatched_lifetime_syntaxes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::double_ended_iterator_last)]

use clap::{Parser, command};
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use hydebar_core::{
    ModuleContext,
    adapters::hyprland_client::HyprlandClient,
    config::{ConfigLoadError, ConfigManager, get_config},
    event_bus::EventBus,
};
use hydebar_gui::{App, get_log_spec};
use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::Font;
use log::{debug, error};
use std::panic;
use std::path::PathBuf;
use std::{backtrace::Backtrace, borrow::Cow, num::NonZeroUsize, sync::Arc};
use thiserror::Error;

const ICON_FONT: &[u8] = include_bytes!("../../assets/SymbolsNerdFont-Regular.ttf");

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_parser = clap::value_parser!(PathBuf))]
    config_path: Option<PathBuf>,
}

#[derive(Debug, Error)]
enum MainError {
    #[error("failed to initialize logger: {0}")]
    Logger(#[from] flexi_logger::FlexiLoggerError),
    #[error("configuration error: {0}")]
    Config(#[from] ConfigLoadError),
    #[error("iced runtime error: {0}")]
    Iced(#[from] iced::Error),
    #[error("invalid event bus capacity")]
    BusCapacity,
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    run().await
}

async fn run() -> Result<(), MainError> {
    let args = Args::parse();
    debug!("args: {args:?}");

    let logger = Logger::with(
        LogSpecBuilder::new()
            .default(log::LevelFilter::Info)
            .build(),
    )
    .log_to_file(FileSpec::default().directory("/tmp/hydebar"))
    .duplicate_to_stdout(flexi_logger::Duplicate::All)
    .rotate(
        Criterion::Age(Age::Day),
        Naming::Timestamps,
        Cleanup::KeepLogFiles(7),
    );
    let logger = if cfg!(debug_assertions) {
        logger.duplicate_to_stdout(flexi_logger::Duplicate::All)
    } else {
        logger
    };
    let logger = logger.start()?;
    panic::set_hook(Box::new(|info| {
        let b = Backtrace::capture();
        error!("Panic: {info} \n {b}");
    }));

    let (config, config_path) = get_config(args.config_path)?;
    let config_manager = Arc::new(ConfigManager::new(config.clone()));

    logger.set_new_spec(get_log_spec(&config.log_level))?;

    let font = match config.appearance.font_name {
        Some(ref font_name) => Font::with_name(Box::leak(font_name.clone().into_boxed_str())),
        None => Font::DEFAULT,
    };

    let hyprland: Arc<dyn HyprlandPort> = Arc::new(HyprlandClient::new());

    let bus_capacity = NonZeroUsize::new(256).ok_or(MainError::BusCapacity)?;
    let event_bus = EventBus::new(bus_capacity);
    let module_context = ModuleContext::new(event_bus.sender(), tokio::runtime::Handle::current());
    let bus_receiver = event_bus.receiver();

    iced::daemon(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .style(App::style)
        .scale_factor(App::scale_factor)
        .font(Cow::from(ICON_FONT))
        .default_font(font)
        .run_with(App::new((
            logger,
            config,
            config_manager,
            config_path,
            hyprland,
            module_context,
            bus_receiver,
        )))
        .map_err(MainError::from)
}
