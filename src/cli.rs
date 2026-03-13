mod simple_consumer;

use anyhow::Result;
use clap::{Parser, Subcommand};

use simple_consumer::SimpleConsumerArgs;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub async  fn run(self) -> Result<()> 
    {
        match self.command{
            Command::SimpleConsumer(args) => args.run().await,
        }
    }
}


#[derive(Subcommand, Debug)]
pub enum Command {
    /// 启动实时服务
    SimpleConsumer(SimpleConsumerArgs),
}


