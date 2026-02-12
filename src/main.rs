use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

mod config;
mod strategies;
mod smtp;

use config::Config;
use strategies::create_strategies;
use smtp::handle_connection;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Load configuration from JSON file
    let config = Config::load()?;
    
    let smtp_port = config.smtp_port;
    let strategies = Arc::new(create_strategies(config.strategies)?);
    
    let strategy_names: Vec<&str> = strategies.iter().map(|s| s.name()).collect();
    
    let addr = SocketAddr::from(([0, 0, 0, 0], smtp_port));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("SMTP server listening on port {}", smtp_port);
    tracing::info!("Active strategies: {:?}", strategy_names);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let strategies = Arc::clone(&strategies);
                tokio::spawn(async move {
                    if let Err(err) = handle_connection(stream, strategies).await {
                        tracing::error!("Error handling connection: {:?}", err);
                    }
                });
            }
            Err(err) => {
                tracing::error!("Error accepting connection: {:?}", err);
            }
        }
    }
}
