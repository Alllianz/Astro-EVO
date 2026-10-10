use super::models::{SimulatedTrade, StatementRow, TradeAction};
use crate::db::Kline;
use chrono::{TimeZone, Utc};

/// Estima los segundos por vela según el timeframe
pub fn get_timeframe_seconds(tf: &str) -> i64 {
    match tf.to_uppercase().as_str() {
        "1M" => 60,
        "3M" => 180,
        "5M" => 300,
        "15M" => 900,
        "30M" => 1800,
        "1H" => 3600,
        "2H" => 7200,
        "4H" => 14400,
        "6H" => 21600,
        "8H" => 28800,
        "12H" => 43200,
        "1D" => 86400,
        "3D" => 259200,
        "1W" => 604800,
        _ => 86400,
    }
}

/// Determina el tamaño de 1 pip según el rango de precio del activo
pub fn get_pip_size(price: f64) -> f64 {
    if price >= 1000.0 {
        0.1 // Para BTCUSDT o XAUUSD
    } else if price >= 10.0 {
        0.01
    } else {
        0.0001 // Para pares Forex tradicionales (EURUSD, etc.)
    }
}

/// Formatea una duración en segundos al formato amigable de Myfxbook (ej: "1h 14m", "10m", "0s")
pub fn format_friendly_duration(secs: i64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let rem_secs = secs % 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m", mins)
    } else {
        format!("{}s", rem_secs)
    }
}

/// Calcula todas las 27 métricas financieras de un trade simulado con formato idéntico a Myfxbook
pub fn calculate_statement_row(
    trade: &SimulatedTrade,
    klines: &[Kline],
    timeframe: &str,
) -> StatementRow {
    let open_date = Utc
        .timestamp_millis_opt(trade.open_timestamp)
        .single()
        .map(|d| d.format("%m/%d/%Y %H:%M").to_string())
        .unwrap_or_else(|| "01/01/2020 00:00".to_string());

    let close_date = Utc
        .timestamp_millis_opt(trade.close_timestamp)
        .single()
        .map(|d| d.format("%m/%d/%Y %H:%M").to_string())
        .unwrap_or_else(|| "01/01/2020 00:00".to_string());

    let pip_size = get_pip_size(trade.open_price);

    let pips = match trade.action {
        TradeAction::Buy => (trade.close_price - trade.open_price) / pip_size,
        TradeAction::Sell => (trade.open_price - trade.close_price) / pip_size,
    };

    let gain = if trade.capital_at_open > 0.0 {
        (trade.net_profit / trade.capital_at_open) * 100.0
    } else {
        0.0
    };

    let duration_secs = ((trade.close_timestamp - trade.open_timestamp).max(0) / 1000) as i64;
    let d_days = duration_secs / 86400;
    let d_hours = (duration_secs % 86400) / 3600;
    let d_mins = (duration_secs % 3600) / 60;
    let d_secs = duration_secs % 60;
    let duration_formatted = format!("{:02}:{:02}:{:02}:{:02}", d_days, d_hours, d_mins, d_secs);

    // Análisis intra-trade vela por vela (MFE, MAE, Drawdown, tiempo rentable)
    let start_idx = trade.open_idx.min(klines.len().saturating_sub(1));
    let end_idx = trade.close_idx.min(klines.len().saturating_sub(1));

    let mut max_favorable_pips: f64 = 0.0;
    let mut max_adverse_pips: f64 = 0.0;
    let mut bars_profitable: usize = 0;

    let trade_bars = if start_idx <= end_idx && !klines.is_empty() {
        &klines[start_idx..=end_idx]
    } else {
        &[]
    };

    for k in trade_bars {
        match trade.action {
            TradeAction::Buy => {
                let pips_high = (k.high - trade.open_price) / pip_size;
                let pips_low = (k.low - trade.open_price) / pip_size;
                if pips_high > max_favorable_pips {
                    max_favorable_pips = pips_high;
                }
                if pips_low < max_adverse_pips {
                    max_adverse_pips = pips_low;
                }
                if k.close > trade.open_price {
                    bars_profitable += 1;
                }
            }
            TradeAction::Sell => {
                let pips_favorable = (trade.open_price - k.low) / pip_size;
                let pips_adverse = (trade.open_price - k.high) / pip_size;
                if pips_favorable > max_favorable_pips {
                    max_favorable_pips = pips_favorable;
                }
                if pips_adverse < max_adverse_pips {
                    max_adverse_pips = pips_adverse;
                }
                if k.close < trade.open_price {
                    bars_profitable += 1;
                }
            }
        }
    }

    let total_bars = trade_bars.len().max(1);
    let profitable_pct = (bars_profitable as f64 / total_bars as f64) * 100.0;

    let bar_secs = get_timeframe_seconds(timeframe);
    let prof_duration_secs = (bars_profitable as i64) * bar_secs;
    let profitable_duration = format_friendly_duration(prof_duration_secs);

    let max_pips = max_favorable_pips.max(0.0);
    let min_pips = max_adverse_pips.min(0.0);
    let drawdown_pips = min_pips.abs();

    // Valor monetario aproximado de 1 pip para este trade
    let pip_value = if pips.abs() > 1e-4 {
        trade.net_profit.abs() / pips.abs()
    } else {
        trade.units_lots * pip_size
    };

    let max_currency = max_pips * pip_value;
    let min_currency = min_pips * pip_value;

    let risk_reward = if drawdown_pips > 0.0 {
        max_pips / drawdown_pips
    } else {
        0.0
    };

    let entry_accuracy = if (max_pips + drawdown_pips) > 0.0 {
        (max_pips / (max_pips + drawdown_pips)) * 100.0
    } else {
        100.0
    };

    let exit_accuracy = if (max_pips + drawdown_pips) > 0.0 {
        (((pips + drawdown_pips) / (max_pips + drawdown_pips)).clamp(0.0, 1.0)) * 100.0
    } else {
        100.0
    };

    let profit_missed_pips = (pips - max_pips).min(0.0);
    let profit_missed_currency = (trade.net_profit - max_currency).min(0.0);

    let sl_fmt = if trade.sl_price > 0.0 {
        format!("{:.5}", trade.sl_price)
    } else {
        "0.00000".to_string()
    };

    let tp_fmt = if trade.tp_price > 0.0 {
        format!("{:.5}", trade.tp_price)
    } else {
        "0.00000".to_string()
    };

    StatementRow {
        open_date,
        close_date,
        symbol: trade.symbol.clone(),
        action: trade.action.to_string(),
        units_lots: format!("{:.3}", trade.units_lots),
        sl: sl_fmt,
        tp: tp_fmt,
        open_price: format!("{:.5}", trade.open_price),
        close_price: format!("{:.5}", trade.close_price),
        commission: format!("{:.4}", trade.commission),
        swap: format!("{:.4}", trade.swap),
        pips: format!("{:.1}", pips),
        profit: format!("{:.2}", trade.net_profit),
        gain: format!("{:.2}", gain),
        duration: duration_formatted,
        profitable_pct: format!("{:.1}", profitable_pct),
        profitable_duration,
        drawdown: format!("{:.1}", drawdown_pips),
        risk_reward: format!("{:.2}", risk_reward),
        max_pips: format!("{:.1}", max_pips),
        max_currency: format!("{:.2}", max_currency),
        min_pips: format!("{:.1}", min_pips),
        min_currency: format!("{:.3}", min_currency),
        entry_accuracy_pct: format!("{:.1}", entry_accuracy),
        exit_accuracy_pct: format!("{:.1}", exit_accuracy),
        profit_missed_pips: format!("{:.2}", profit_missed_pips),
        profit_missed_currency: format!("{:.2}", profit_missed_currency),
    }
}
