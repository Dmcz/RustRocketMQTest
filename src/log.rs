use std::sync::OnceLock;

use anyhow::Context;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;

static LOG_LEVEL_HANDLE: OnceLock<reload::Handle<LevelFilter, Registry>> = OnceLock::new();

pub fn init(level: &str) -> anyhow::Result<()> {
    let parsed_level = parse_level(level)?;
    init_parsed_level(parsed_level)
}


fn parse_level(level: &str) -> anyhow::Result<LevelFilter> {
    level
        .parse::<LevelFilter>()
        .with_context(|| format!("invalid log level: {level}"))
}

fn init_parsed_level(parsed_level: LevelFilter) -> anyhow::Result<()> {
    if LOG_LEVEL_HANDLE.get().is_some() {
        anyhow::bail!("logger already initialized");
    }

    std::fs::create_dir_all("logs")
        .with_context(|| "failed to create logs directory".to_string())?;

    let (filter_layer, handle) = reload::Layer::new(parsed_level);
    let console_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_target(false)
        .with_filter(filter_fn(|meta| meta.target() != "app.error.detail"));

    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(file_appender)
        .with_target(true);

    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(console_layer)
        .with(file_layer);

    subscriber
        .try_init()
        .with_context(|| "failed to initialize global logger".to_string())?;
    let _ = LOG_LEVEL_HANDLE.set(handle);

    Ok(())
}
