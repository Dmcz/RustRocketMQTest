use std::{
    fmt::{Debug},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use anyhow::{Context, Result};
use clap::Args;
use rocketmq::{
    conf::{ClientOption, SimpleConsumerOption},
    model::common::{FilterExpression, FilterType},
    SimpleConsumer,
};
use tracing::info;
use tokio::{signal, task::JoinHandle, time::sleep};

use crate::types::HostPort;
use crate::timezone;

struct ShutdownSignal {
    requested: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl ShutdownSignal {
    fn install(command_name: &'static str) -> Self {
        let requested = Arc::new(AtomicBool::new(false));
        let requested_for_task = Arc::clone(&requested);

        let handle = tokio::spawn(async move {
            if signal::ctrl_c().await.is_ok() {
                requested_for_task.store(true, Ordering::SeqCst);
                info!("received Ctrl-C, shutting down {}", command_name);
            }
        });

        Self { requested, handle }
    }

    fn is_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

impl Drop for ShutdownSignal {
    fn drop(&mut self) {
        self.handle.abort();
    }
}


#[derive(Args, Debug, Clone)]
pub(crate) struct SimpleConsumerArgs {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(long, default_value = "8081")]
    pub port: u16,    

    #[arg(long, default_value = "test")]
    pub consumer_group: String,

    #[arg(long)]
    pub topic: String,

    #[arg(long, default_value="*")]
    pub tag: String,

    #[arg(long, default_value_t = false)]
    pub print_body: bool,

    #[arg(long, default_value = "")]
    pub namespace: String,

    #[arg(long, default_value = "3s", value_parser = humantime::parse_duration)]
    pub timeout: Duration,

    #[arg(long, default_value = "40s", value_parser = humantime::parse_duration)]
    pub long_polling_timeout: Duration,
}

pub(super) struct SimpleConsumerCommand {
    access: HostPort,
    consumer_group: String,
    topic: String,
    tag: String,
    print_body: bool,
    namespace: String,
    timeout: Duration,
    long_polling_timeout: Duration,
}

impl TryFrom<SimpleConsumerArgs> for SimpleConsumerCommand {
    type Error = anyhow::Error;

    fn try_from(args: SimpleConsumerArgs) -> Result<Self> {
        Ok(Self {
            access: HostPort::new(args.host, args.port),
            consumer_group: args.consumer_group,
            topic: args.topic,
            tag: args.tag,
            print_body: args.print_body,
            namespace: args.namespace,
            timeout: args.timeout,
            long_polling_timeout: args.long_polling_timeout,
        })
    }
}

impl SimpleConsumerCommand {
    pub(super) async fn run(self) -> Result<()> {
        let mut consumer_option = SimpleConsumerOption::default();
        consumer_option.set_consumer_group(self.consumer_group);
        consumer_option.set_topics(vec![self.topic.clone()]);

        let mut client_option = ClientOption::default();
        client_option.set_access_url(self.access.to_string());
        client_option.set_namespace(self.namespace);
        client_option.set_long_polling_timeout(self.long_polling_timeout);
        client_option.set_timeout(self.timeout);

        let mut consumer = SimpleConsumer::new(consumer_option, client_option).context("Failed to create simple consumer")?;
        consumer.start().await.context("Failed to start simple consumer")?;

        let shutdown = ShutdownSignal::install("simple consumer");
        let topic = self.topic;
        let tag = self.tag;
        let print_body = self.print_body;

        loop {
            if shutdown.is_requested() {
                info!("shutdown requested, stop receiving new messages");
                break;
            }

            let messages = consumer.receive_with(
                topic.clone(),
                &FilterExpression::new(FilterType::Tag, tag.clone()),
                16,
                Duration::from_secs(20),
            ).await.context("Failed to receive message.")?;

            if messages.is_empty() {
                info!("No message.");
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            info!("received {} messages.", messages.len());

            for (index, message) in messages.into_iter().enumerate() {
         
                info!(
                    "\n {}", 
                    indoc::formatdoc! {
                    "
                    message [{}],
                        message_id: {},
                        topic: {},
                        tag: {:?},
                        keys: {:?},
                        message_group: {:?},
                        delivery_timestamp: {},
                        delivery_attempt: {},
                        born_host: {},
                        born_timestamp: {},
                    ", 
                    index, 
                    message.message_id(),
                    message.topic(),
                    message.tag(),
                    message.keys(),
                    message.message_group(),
                    message.delivery_timestamp().map(timezone::format_unix_timestamp).unwrap_or_else(|| "None".to_string()),
                    message.delivery_attempt(),
                    message.born_host(),
                    timezone::format_unix_timestamp(message.born_timestamp()),
                });
                if print_body {
                    info!("    body: {}", String::from_utf8_lossy(message.body()));
                }

                consumer.ack(&message).await.context("Failed to ack.")?;
            }

            if shutdown.is_requested() {
                info!("shutdown requested, current batch finished");
                break;
            }
        }

        consumer.shutdown().await.context("Failed to shutdown simple consumer")?;

        Ok(())
    }
}
