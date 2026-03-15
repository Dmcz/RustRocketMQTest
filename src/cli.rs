mod simple_consumer;
mod producer;

use anyhow::Result;
use clap::{Parser, Subcommand};

use simple_consumer::{SimpleConsumerArgs, SimpleConsumerCommand};

use crate::cli::producer::{ProducerArgs, ProducerCommand};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Command::SimpleConsumer(args) => SimpleConsumerCommand::try_from(args)?.run().await,
            Command::Producer(args) => ProducerCommand::try_from(args)?.run().await,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    SimpleConsumer(SimpleConsumerArgs),
    Producer(ProducerArgs),
}
