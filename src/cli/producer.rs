use std::{fs, path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Args};
use rocketmq::{
    conf::{ClientOption, ProducerOption},
    model::message::MessageBuilder,
    Producer,
};
use tracing::info;
use uuid::Uuid;

use crate::types::HostPort;

#[derive(Args, Debug, Clone)]
#[command(group(
    ArgGroup::new("body_source")
        .args(["text", "file"])
        .required(true)
))]
pub(crate) struct ProducerArgs {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(long, default_value = "8081")]
    pub port: u16,    

    #[arg(long)]
    pub topic: String,

    #[arg(long, default_value="*")]
    pub tag: String,

    #[arg(long, default_value = "")]
    pub namespace: String,

    #[arg(long, value_delimiter = ',')]
    pub keys: Vec<String>,

    #[arg(long, group = "body_source")]
    pub text: Option<String>,

    #[arg(long, group = "body_source")]
    pub file: Option<PathBuf>,

    #[arg(long, default_value_t = 1)]
    pub repeat: u32,

    #[arg(long, default_value_t = 0.0)]
    pub interval: f64,
}

enum BodySource {
    Text(String),
    File(PathBuf),
}

pub(super) struct ProducerCommand {
    access: HostPort,
    topic: String,
    tag: String,
    namespace: String,
    keys: Vec<String>,
    body_source: BodySource,
    repeat: u32,
    interval: Duration,
}

impl TryFrom<ProducerArgs> for ProducerCommand {
    type Error = anyhow::Error;

    fn try_from(args: ProducerArgs) -> Result<Self> {
        let body_source = match (args.text, args.file) {
            (Some(text), None) => BodySource::Text(text),
            (None, Some(file)) => BodySource::File(file),
            _ => bail!("exactly one of --text or --file must be provided"),
        };

        if args.repeat == 0 {
            bail!("--repeat must be greater than 0");
        }

        if args.interval < 0.0 {
            bail!("--interval must be greater than or equal to 0");
        }

        Ok(Self {
            access: HostPort::new(args.host, args.port),
            topic: args.topic,
            tag: args.tag,
            namespace: args.namespace,
            keys: args.keys,
            body_source,
            repeat: args.repeat,
            interval: Duration::from_secs_f64(args.interval),
        })
    }
}

impl ProducerCommand {
    pub(super) async fn run(self) -> Result<()> {
        let Self {
            access,
            topic,
            tag,
            namespace,
            keys,
            body_source,
            repeat,
            interval,
        } = self;

        let mut producer_option = ProducerOption::default();
        producer_option.set_topics(vec![topic.clone()]);

        let mut client_option = ClientOption::default();
        client_option.set_access_url(access.to_string());
        client_option.set_namespace(namespace);

        let mut producer = Producer::new(producer_option, client_option).context("Failed to create producer")?;
        producer.start().await.context("Failed to start producer")?;

        let body = Self::read_body(&body_source).context("Failed to load message body")?;
        let batch_id = Uuid::new_v4();

        for index in 0..repeat {
            let mut builder = MessageBuilder::builder()
                .set_topic(topic.clone())
                .set_tag(tag.clone())
                .set_body(body.clone());
            
            let mut message_keys = vec![
                format!("batch={batch_id}"),
                format!("index={}", index + 1),
            ];
            if !keys.is_empty() {
                message_keys.extend(keys.clone());
            }
            builder = builder.set_keys(message_keys);

            let message = builder.build().context("Failed to bulid message")?;
            let receipt = producer.send(message).await.context("Failed to send message")?;
            info!("message sent [{}/{}]: {:?}", index + 1, repeat, receipt);

            if index + 1 < repeat && !interval.is_zero() {
                tokio::time::sleep(interval).await;
            }
        }

        Ok(())
    }

    fn read_body(body_source: &BodySource) -> Result<Vec<u8>> {
        match body_source {
            BodySource::Text(text) => Ok(text.clone().into_bytes()),
            BodySource::File(path) => fs::read(path)
                .with_context(|| format!("Failed to read message body file: {}", path.display())),
        }
    }
}
