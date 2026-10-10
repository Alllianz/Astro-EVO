use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardModelParams {
    pub tf: String,
    pub window_size: usize,
    pub kernel_size: usize,
    pub horizon: usize,
    pub weight_decay: f32,
    pub l1_lambda: f32,
    pub epoch: usize,
    pub leverage: f64,
    pub capital_percent: f64,
    pub is_start_date: String,
    pub oos_start_date: String,
    pub is_start_idx: usize,
    pub oos_start_idx: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardSummaryMetrics {
    pub capital_final_nom: f64,
    pub net_profit_nom: f64,
    pub total_return_nom_pct: f64,
    pub max_dd_nom_pct: f64,
    pub max_dd_nom_dollar: f64,
    pub winrate_nom: f64,
    pub profit_factor_nom: f64,
    pub total_trades_nom: usize,
    pub longs_count_nom: usize,
    pub shorts_count_nom: usize,
    pub liquidations_nom: usize,
    pub slope_ratio_nom: f64,
    pub is_return_nom_pct: f64,
    pub oos_return_nom_pct: f64,

    pub capital_final_pct: f64,
    pub net_profit_pct: f64,
    pub total_return_pct_pct: f64,
    pub max_dd_pct_pct: f64,
    pub max_dd_pct_dollar: f64,
    pub winrate_pct: f64,
    pub profit_factor_pct: f64,
    pub total_trades_pct: usize,
    pub longs_count_pct: usize,
    pub shorts_count_pct: usize,
    pub liquidations_pct: usize,
    pub slope_ratio_pct: f64,
    pub is_return_pct_pct: f64,
    pub oos_return_pct_pct: f64,
}
