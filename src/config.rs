use serde::{Deserialize, Serialize};
use std::path::Path;

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    pub strategies: Vec<StrategyConfig>,
}

fn default_smtp_port() -> u16 {
    2525
}

impl Default for Config {
    fn default() -> Self {
        Self {
            smtp_port: 2525,
            strategies: vec![StrategyConfig::default()],
        }
    }
}

impl Config {
    /// Load configuration from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from default location or create default
    pub fn load() -> anyhow::Result<Self> {
        let config_paths = [
            "config.json",
            "smtp-relay.json",
            "/etc/smtp-relay/config.json",
        ];

        for path in &config_paths {
            if std::path::Path::new(path).exists() {
                tracing::info!("Loading configuration from: {}", path);
                return Self::from_file(path);
            }
        }

        tracing::warn!("No configuration file found, using defaults");
        Ok(Self::default())
    }
}

/// Configuration for a single strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    #[serde(rename = "type")]
    pub strategy_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<Vec<(String, String)>>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            strategy_type: "webhook".to_string(),
            api_key: None,
            api_url: Some("http://localhost:3000/email".to_string()),
            from_address: None,
            extra_headers: None,
        }
    }
}
