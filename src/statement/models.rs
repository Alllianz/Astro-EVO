use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeAction {
    Buy,
    Sell,
}

impl std::fmt::Display for TradeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeAction::Buy => write!(f, "Buy"),
            TradeAction::Sell => write!(f, "Sell"),
        }
    }
}

/// Representa una operación individual simulada durante el entrenamiento/backtest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedTrade {
    pub trade_id: usize,
    pub open_idx: usize,
    pub close_idx: usize,
    pub open_timestamp: i64,
    pub close_timestamp: i64,
    pub symbol: String,
    pub action: TradeAction,
    pub units_lots: f64,
    pub sl_price: f64,
    pub tp_price: f64,
    pub open_price: f64,
    pub close_price: f64,
    pub commission: f64,
    pub swap: f64,
    pub net_profit: f64,
    pub capital_at_open: f64,
    pub is_liquidation: bool,
}

/// Fila formateada lista para exportar al CSV con el formato exacto del statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementRow {
    pub open_date: String,
    pub close_date: String,
    pub symbol: String,
    pub action: String,
    pub units_lots: String,
    pub sl: String,
    pub tp: String,
    pub open_price: String,
    pub close_price: String,
    pub commission: String,
    pub swap: String,
    pub pips: String,
    pub profit: String,
    pub gain: String,
    pub duration: String,
    pub profitable_pct: String,
    pub profitable_duration: String,
    pub drawdown: String,
    pub risk_reward: String,
    pub max_pips: String,
    pub max_currency: String,
    pub min_pips: String,
    pub min_currency: String,
    pub entry_accuracy_pct: String,
    pub exit_accuracy_pct: String,
    pub profit_missed_pips: String,
    pub profit_missed_currency: String,
}
