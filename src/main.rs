mod cli;
mod log;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::init("info")?;

    let cli = Cli::try_parse()?;

    cli.run().await?;

    Ok(())
}