use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalanceInfo {
    pub id: u32,
    pub timeframe: String,
    pub api_key: String,
    pub api_secret: String,
    pub leverage: u32,
    pub exchange: String,
    pub use_testnet: bool,
    pub wallet_balance: f64,
    pub available_margin: f64,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAccountPool {
    pub accounts: Vec<AccountBalanceInfo>,
    pub total_wallet_balance: f64,
    pub total_available_margin: f64,
    pub last_updated: i64,
}

impl UnifiedAccountPool {
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            total_wallet_balance: 0.0,
            total_available_margin: 0.0,
            last_updated: chrono::Utc::now().timestamp(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct AccountOrderAllocation {
    pub account_id: u32,
    pub user_id: String,
    pub api_key: String,
    pub api_secret: String,
    pub use_testnet: bool,
    pub balance: f64,
    pub weight: f64,
    pub order_qty: f64,
}
