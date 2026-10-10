use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub capital_percent: f64,
    pub leverage: f64,
    pub trade_webhook: String,
    pub status_webhook: String,
    pub server_name: String,
    pub status_interval_minutes: u64,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub id: u32,
    pub timeframe: String,
    pub api_key: String,
    pub api_secret: String,
    pub leverage: u32,
    pub exchange: String,
    pub use_testnet: bool,
}
