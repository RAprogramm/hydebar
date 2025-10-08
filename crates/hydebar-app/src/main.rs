#![allow(mismatched_lifetime_syntaxes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::double_ended_iterator_last)]

use std::{backtrace::Backtrace, borrow::Cow, num::NonZeroUsize, panic, path::PathBuf, sync::Arc};

use clap::{Parser, command};
use flexi_logger::{Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, Logger, Naming};
use hydebar_core::{
    adapters::hyprland_client::HyprlandClient,
    config::{ConfigLoadError, ConfigManager, get_config},
    event_bus::EventBus,
};
use hydebar_gui::{App, get_log_spec};
use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::Font;
use log::{debug, error};
use tokio::runtime::Handle;

const ICON_FONT: &[u8] = include_bytes!("../../../assets/SymbolsNerdFont-Regular.ttf");

#[derive(Parser, Debug,)]
#[command(version, about, long_about = None)]
struct Args
{
    #[arg(short, long, value_parser = clap::value_parser!(PathBuf))]
    config_path: Option<PathBuf,>,
}

#[derive(Debug,)]
enum MainError
{
    Logger(flexi_logger::FlexiLoggerError,),
    Config(ConfigLoadError,),
    Iced(iced::Error,),
    BusCapacity,
}

impl std::fmt::Display for MainError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result
    {
        match self {
            Self::Logger(err,) => write!(f, "failed to initialize logger: {}", err),
            Self::Config(err,) => write!(f, "configuration error: {}", err),
            Self::Iced(err,) => write!(f, "iced runtime error: {}", err),
            Self::BusCapacity => write!(f, "invalid event bus capacity"),
        }
    }
}

impl std::error::Error for MainError
{
    fn source(&self,) -> Option<&(dyn std::error::Error + 'static),>
    {
        match self {
            Self::Logger(err,) => Some(err,),
            Self::Config(err,) => Some(err,),
            Self::Iced(err,) => Some(err,),
            Self::BusCapacity => None,
        }
    }
}

impl From<flexi_logger::FlexiLoggerError,> for MainError
{
    fn from(err: flexi_logger::FlexiLoggerError,) -> Self
    {
        Self::Logger(err,)
    }
}

impl From<ConfigLoadError,> for MainError
{
    fn from(err: ConfigLoadError,) -> Self
    {
        Self::Config(err,)
    }
}

impl From<iced::Error,> for MainError
{
    fn from(err: iced::Error,) -> Self
    {
        Self::Iced(err,)
    }
}

#[tokio::main]
async fn main() -> Result<(), MainError,>
{
    run().await
}

async fn run() -> Result<(), MainError,>
{
    let args = Args::parse();
    debug!("args: {args:?}");

    let logger = Logger::with(LogSpecBuilder::new().default(log::LevelFilter::Info,).build(),)
        .log_to_file(FileSpec::default().directory("/tmp/hydebar",),)
        .duplicate_to_stdout(flexi_logger::Duplicate::All,)
        .rotate(Criterion::Age(Age::Day,), Naming::Timestamps, Cleanup::KeepLogFiles(7,),);
    let logger = if cfg!(debug_assertions) {
        logger.duplicate_to_stdout(flexi_logger::Duplicate::All,)
    } else {
        logger
    };
    let logger = logger.start()?;
    panic::set_hook(Box::new(|info| {
        let b = Backtrace::capture();
        error!("Panic: {info} \n {b}");
    },),);

    let (config, config_path,) = get_config(args.config_path,)?;
    let config_manager = Arc::new(ConfigManager::new(config.clone(),),);

    logger.set_new_spec(get_log_spec(&config.log_level,),);

    let font = match config.appearance.font_name {
        Some(ref font_name,) => Font::with_name(Box::leak(font_name.clone().into_boxed_str(),),),
        None => Font::DEFAULT,
    };

    let hyprland: Arc<dyn HyprlandPort,> = Arc::new(HyprlandClient::new(),);

    let bus_capacity = NonZeroUsize::new(256,).ok_or(MainError::BusCapacity,)?;
    let event_bus = EventBus::new(bus_capacity,);
    let event_sender = event_bus.sender();
    let runtime_handle = Handle::current();
    let bus_receiver = event_bus.receiver();

    iced::daemon(App::title, App::update, App::view,)
        .subscription(App::subscription,)
        .theme(App::theme,)
        .style(App::style,)
        .scale_factor(App::scale_factor,)
        .font(Cow::from(ICON_FONT,),)
        .default_font(font,)
        .run_with(App::new((
            logger,
            config,
            config_manager,
            config_path,
            hyprland,
            event_sender,
            runtime_handle,
            bus_receiver,
        ),),)
        .map_err(MainError::from,)
}
