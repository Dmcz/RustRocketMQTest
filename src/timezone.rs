use std::sync::OnceLock;

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use chrono_tz::Tz;

static TIMEZONE: OnceLock<Tz> = OnceLock::new();

pub fn init_from_env() -> Result<()> {
    let tz_name = std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string());
    let timezone = tz_name
        .parse::<Tz>()
        .with_context(|| format!("invalid TZ value: {tz_name}"))?;

    TIMEZONE
        .set(timezone)
        .map_err(|_| anyhow::anyhow!("timezone already initialized"))?;

    Ok(())
}

pub fn current() -> Tz {
    *TIMEZONE.get_or_init(|| chrono_tz::UTC)
}

pub fn format_unix_timestamp(ts: i64) -> String {
    match chrono::DateTime::<Utc>::from_timestamp(ts, 0) {
        Some(dt) => dt
            .with_timezone(&current())
            .to_rfc3339_opts(SecondsFormat::Secs, false),
        None => format!("invalid timestamp {ts}"),
    }
}
