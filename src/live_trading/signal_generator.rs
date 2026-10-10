use crate::cnn1d::Cnn1DModel;
use crate::db::{get_all_klines, Connection, Kline};
use crate::features::extract_cnn1d_features_for_tf;
use crate::indicators::*;
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    Long,
    Short,
    Flat,
}

#[derive(Debug, Clone)]
pub struct SignalEvaluation {
    pub signal: SignalType,
    pub p_long: f32,
    pub p_short: f32,
    pub p_flat: f32,
    pub dynamic_thr: f32,
    pub window_size: usize,
    pub horizon: usize,
    pub last_price: f64,
    pub candle_timestamp: i64,
}

pub fn evaluate_timeframe_signal_with_model(
    conn_candles: &Connection,
    tf: &str,
    live_model: &Cnn1DModel,
    window_size: usize,
    horizon: usize,
    current_pos_val: f32,
) -> Result<Option<SignalEvaluation>, Box<dyn Error>> {
    let klines = get_all_klines(conn_candles, tf)?;
    if klines.len() < window_size + 200 {
        println!(
            "⚠️ [{}] Saltado: Historial de velas insuficiente ({} de {} requeridas).",
            tf, klines.len(), window_size + 200
        );
        return Ok(None);
    }

    let volume_sma = precalculate_volume_sma(&klines);
    let (volume_sma_5, volume_sma_20) = precalculate_volume_smas(&klines);
    let macd_hist = precalculate_macd_hist(&klines);
    let ema_20 = precalculate_ema_20(&klines);
    let ema_200_hl2 = precalculate_ema_200_hl2(&klines);
    let atr_5 = precalculate_atr(&klines, 5);
    let atr_50 = precalculate_atr(&klines, 50);

    let last_idx = klines.len() - 1;
    let feat = match extract_cnn1d_features_for_tf(
        &klines,
        last_idx,
        window_size,
        tf,
        &volume_sma,
        &volume_sma_5,
        &volume_sma_20,
        &macd_hist,
        &ema_20,
        &ema_200_hl2,
        &atr_5,
        &atr_50,
        current_pos_val,
    ) {
        Some(f) => f,
        None => return Ok(None),
    };

    let probs = live_model.predict(&feat);
    let p_long = probs[0];
    let p_short = probs[1];
    let p_flat = probs[2];

    let last_kline = klines.last();
    let last_price = last_kline.map(|k: &Kline| k.close).unwrap_or(50000.0);
    let candle_timestamp = last_kline.map(|k: &Kline| k.timestamp).unwrap_or(0);

    // Umbral dinámico de volatilidad (Idéntico a online_learning.rs)
    let atr_ratio = if last_price > 0.0 { atr_50[last_idx] / last_price } else { 0.01 };
    let dynamic_thr = (0.40 + 0.15 * (atr_ratio * 100.0).clamp(0.0, 1.0)).clamp(0.40, 0.55) as f32;

    // Regla de decisión de señal (Idéntico a online_learning.rs)
    let signal = if p_long > p_short && p_long > p_flat && p_long > dynamic_thr {
        SignalType::Long
    } else if p_short > p_long && p_short > p_flat && p_short > dynamic_thr {
        SignalType::Short
    } else {
        SignalType::Flat
    };

    Ok(Some(SignalEvaluation {
        signal,
        p_long,
        p_short,
        p_flat,
        dynamic_thr,
        window_size,
        horizon,
        last_price,
        candle_timestamp,
    }))
}
