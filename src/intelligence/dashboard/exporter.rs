use crate::db::Kline;
use crate::intelligence::models::IntelligenceLevelResult;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IntelligenceLevelChartItem {
    pub level: usize,
    pub neurons_desc: String,
    pub conv1_filters: usize,
    pub conv2_filters: usize,
    pub dense_neurons: usize,
    pub total_params: usize,
    pub conv1_params: usize,
    pub conv2_params: usize,
    pub dense_params: usize,
    pub color_hex: String,
    pub eq_nom: Vec<f32>,
    pub eq_pct: Vec<f32>,
    #[serde(default)]
    pub total_profit_nom: f64,
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
    pub sortino_nom: f64,
    pub ulcer_index_nom: f64,
    pub curve_r2_nom: f64,
    pub r2_is_nom: f64,
    pub r2_oos_nom: f64,
    pub slope_ratio_nom: f64,
    pub capital_dd_ratio_nom: f64,
    pub smoothness_score_nom: f64,
    #[serde(default)]
    pub total_profit_pct: f64,
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
    pub weighted_avg_rank: f64,
    pub rank_r2: usize,
    pub rank_smoothness: usize,
    pub rank_slope: usize,
    pub rank_cap_dd: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IntelligenceComparisonExport {
    pub tf: String,
    pub start_intelligence: usize,
    pub iterations: usize,
    pub window_size: usize,
    pub kernel_size: usize,
    pub horizon: usize,
    pub leverage: f64,
    pub capital_percent: f64,
    pub is_start_date: String,
    pub oos_start_date: String,
    pub is_start_idx: usize,
    pub oos_start_idx: usize,
    pub timestamps: Vec<i64>,
    pub dates: Vec<String>,
    pub btc_prices: Vec<f64>,
    pub levels: Vec<IntelligenceLevelChartItem>,
    pub best_level: usize,
    pub best_neurons_desc: String,
}

pub fn get_palette_color(index: usize) -> String {
    let colors = [
        "#00f0ff", // Neon Cyan
        "#ff0055", // Neon Magenta / Pink
        "#ffb700", // Gold / Amber
        "#00ff66", // Neon Green
        "#b026ff", // Electric Purple
        "#ff6600", // Neon Orange
        "#0099ff", // Deep Sky Blue
        "#ff0033", // Imperial Red
        "#00ffcc", // Aquamarine
        "#e60073", // Deep Rose
        "#ffff00", // Pure Yellow
        "#39ff14", // Acid Green
    ];
    colors[index % colors.len()].to_string()
}

pub fn export_intelligence_comparison_data(
    klines: &[Kline],
    is_start_idx: usize,
    oos_start_idx: usize,
    results: &[IntelligenceLevelResult],
    tf: &str,
    start_intelligence: usize,
    iterations: usize,
    window_size: usize,
    kernel_size: usize,
    horizon: usize,
    leverage: f64,
    capital_percent: f64,
) -> IntelligenceComparisonExport {
    let mut timestamps = Vec::new();
    let mut dates = Vec::new();
    let mut btc_prices = Vec::new();

    for k in klines.iter().skip(is_start_idx) {
        timestamps.push(k.timestamp);
        let dt = Utc.timestamp_millis_opt(k.timestamp)
            .single()
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        dates.push(dt);
        btc_prices.push(k.close);
    }

    let mut levels_items = Vec::new();

    for (idx, res) in results.iter().enumerate() {
        let color = get_palette_color(idx);
        let neurons_desc = format!("{} ➔ {} ➔ {}", res.layer_info.conv1_out, res.layer_info.conv2_out, res.layer_info.dense_in);

        let eq_nom_series: Vec<f32> = res.eq_curve_nom.iter().map(|&(_, eq)| eq).collect();
        let eq_pct_series: Vec<f32> = res.eq_curve_pct.iter().map(|&(_, eq)| eq).collect();

        let nom = &res.metrics_nom;
        let pct = &res.metrics_pct;

        levels_items.push(IntelligenceLevelChartItem {
            level: res.level,
            neurons_desc,
            conv1_filters: res.layer_info.conv1_out,
            conv2_filters: res.layer_info.conv2_out,
            dense_neurons: res.layer_info.dense_in,
            total_params: res.layer_info.total_weights_count,
            conv1_params: res.layer_info.conv1_weights_count,
            conv2_params: res.layer_info.conv2_weights_count,
            dense_params: res.layer_info.dense_weights_count,
            color_hex: color,
            eq_nom: eq_nom_series,
            eq_pct: eq_pct_series,
            total_profit_nom: nom.capital_final as f64,
            net_profit_nom: nom.net_profit,
            total_return_nom_pct: ((nom.capital_final as f64 - 10000.0) / 10000.0) * 100.0,
            max_dd_nom_pct: nom.max_dd as f64,
            max_dd_nom_dollar: nom.max_dd_nominal,
            winrate_nom: nom.winrate,
            profit_factor_nom: nom.profit_factor,
            total_trades_nom: nom.total_trades,
            longs_count_nom: nom.longs_count,
            shorts_count_nom: nom.shorts_count,
            liquidations_nom: nom.liquidations,
            sortino_nom: nom.sortino_ratio,
            ulcer_index_nom: nom.ulcer_index,
            curve_r2_nom: nom.curve_r2,
            r2_is_nom: nom.r2_is,
            r2_oos_nom: nom.r2_oos,
            slope_ratio_nom: nom.slope_ratio,
            capital_dd_ratio_nom: nom.capital_dd_ratio,
            smoothness_score_nom: nom.smoothness_score,
            total_profit_pct: pct.capital_final as f64,
            net_profit_pct: pct.net_profit,
            total_return_pct_pct: ((pct.capital_final as f64 - 10000.0) / 10000.0) * 100.0,
            max_dd_pct_pct: pct.max_dd as f64,
            max_dd_pct_dollar: pct.max_dd_nominal,
            winrate_pct: pct.winrate,
            profit_factor_pct: pct.profit_factor,
            total_trades_pct: pct.total_trades,
            longs_count_pct: pct.longs_count,
            shorts_count_pct: pct.shorts_count,
            liquidations_pct: pct.liquidations,
            weighted_avg_rank: res.rank_score.weighted_avg_rank,
            rank_r2: res.rank_score.rank_r2,
            rank_smoothness: res.rank_score.rank_smoothness,
            rank_slope: res.rank_score.rank_slope,
            rank_cap_dd: res.rank_score.rank_cap_dd,
        });
    }

    let best_level = results.first().map(|r| r.level).unwrap_or(start_intelligence);
    let best_neurons = results.first().map(|r| format!("{} ➔ {} ➔ {}", r.layer_info.conv1_out, r.layer_info.conv2_out, r.layer_info.dense_in)).unwrap_or_else(|| "N/A".to_string());

    IntelligenceComparisonExport {
        tf: tf.to_string(),
        start_intelligence,
        iterations,
        window_size,
        kernel_size,
        horizon,
        leverage,
        capital_percent,
        is_start_date: "2020-04-20".to_string(),
        oos_start_date: "2023-01-01".to_string(),
        is_start_idx,
        oos_start_idx,
        timestamps,
        dates,
        btc_prices,
        levels: levels_items,
        best_level,
        best_neurons_desc: best_neurons,
    }
}
