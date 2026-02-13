pub mod webhook;
pub mod resend;

use webhook::WebhookStrategy;
use resend::ResendStrategy;
use crate::config::StrategyConfig;

/// Email data structure passed to API strategies
#[derive(Debug, Clone)]
pub struct EmailData {
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub raw_data: String,
}

/// Enum representing all available API strategies
#[derive(Debug, Clone)]
pub enum ApiStrategy {
    Webhook(WebhookStrategy),
    Resend(ResendStrategy),
}

impl ApiStrategy {
    /// Send an email using this strategy
    pub async fn send_email(&self, email: EmailData) -> anyhow::Result<()> {
        match self {
            ApiStrategy::Webhook(s) => s.send_email(email).await,
            ApiStrategy::Resend(s) => s.send_email(email).await,
        }
    }
    
    /// Get the name of this strategy
    pub fn name(&self) -> &'static str {
        match self {
            ApiStrategy::Webhook(_) => "webhook",
            ApiStrategy::Resend(_) => "resend",
        }
    }
}

/// Factory function to create a strategy from configuration
pub fn create_strategy(config: StrategyConfig) -> anyhow::Result<ApiStrategy> {
    match config.strategy_type.as_str() {
        "webhook" | "http" | "generic" => {
            let url = config.api_url
                .clone()
                .unwrap_or_else(|| "http://localhost:3000/email".to_string());
            Ok(ApiStrategy::Webhook(WebhookStrategy::new(url, config.extra_headers)?))
        }
        "resend" => {
            let api_key = config.api_key
                .ok_or_else(|| anyhow::anyhow!("api_key is required for resend strategy"))?;
            Ok(ApiStrategy::Resend(ResendStrategy::new(api_key)?))
        }
        _ => {
            anyhow::bail!("Unknown API strategy: {}", config.strategy_type)
        }
    }
}

/// Create all strategies from configuration
pub fn create_strategies(configs: Vec<StrategyConfig>) -> anyhow::Result<Vec<ApiStrategy>> {
    let mut strategies = Vec::new();
    
    for config in configs {
        strategies.push(create_strategy(config)?);
    }
    
    Ok(strategies)
}
