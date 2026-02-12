pub mod config;
pub mod strategies;
pub mod smtp;

pub use config::{Config, StrategyConfig};
pub use strategies::{create_strategies, ApiStrategy, EmailData};
pub use smtp::handle_connection;
