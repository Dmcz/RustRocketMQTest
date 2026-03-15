mod cli;
mod log;
mod timezone;
mod types;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    timezone::init_from_env()?;
    log::init("info")?;

    let cli = Cli::try_parse()?;

    cli.run().await?;

    Ok(())
}
