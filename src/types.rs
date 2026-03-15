use std::{fmt::Display};

#[derive(Debug, Clone)]
pub struct HostPort {
    host: String,
    port: u16,
}

impl HostPort {
    pub fn new(host: impl Into<String>, port: impl Into<u16>) -> Self {
        Self { host: host.into(), port: port.into() }
    }
}

impl Display  for HostPort{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {       
        write!(f, "{}:{}", self.host, self.port)
    }
}
