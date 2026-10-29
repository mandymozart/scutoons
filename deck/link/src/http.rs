//! The real transport: JSON over HTTPS with ureq (blocking – the link runs on its own thread or in a small CLI).

use crate::link::{Reply, Transport};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::time::Duration;

/// The production API (server/docs/API.md "Hosts")
pub const DEFAULT_API: &str = "https://api.scutoons.com/api";
/// The viewer, where the member pairs (`pairing_url`)
pub const DEFAULT_APP: &str = "https://scutoons.com";

pub struct HttpTransport {
    base: String,
    agent: ureq::Agent,
}

impl HttpTransport {
    pub fn new(base: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .build();
        Self {
            base: base.trim_end_matches('/').to_string(),
            agent,
        }
    }
}

impl Transport for HttpTransport {
    fn post(&self, path: &str, body: &Value) -> Result<Reply> {
        let result = self
            .agent
            .post(&format!("{}{path}", self.base))
            .set("Content-Type", "application/json")
            .set("Accept", "application/json")
            .send_string(&body.to_string());
        let (status, response) = match result {
            Ok(response) => (response.status(), response),
            Err(ureq::Error::Status(status, response)) => (status, response),
            Err(error) => return Err(anyhow!("{path}: {error}")),
        };
        let text = response.into_string()?;
        let body = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok(Reply { status, body })
    }
}
