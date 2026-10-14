use crate::db::Kline;
use crate::indicators::compute_markov_state;
use chrono::{Datelike, TimeZone, Timelike, Utc};

/// Calcula las características brutas para una sola vela según la temporalidad especificada.
///
/// - Para **1H** (y sub-horarias): Incluye la feature intradiaria explícita de `Hora del Día` (0..1)
///   además de las 33 features estándar (Total: 34 raw features).
/// - Para **4H**: Incluye el conjunto estándar de 33 raw features.
/// - Para **1D**: Excluye f28 y f29 (invariantes a 00:00 UTC) (Total: 31 raw features).
#[inline(always)]
pub fn compute_single_candle_features_by_tf(
    klines: &[Kline],
    k_idx: usize,
    tf: &str,
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
) -> Vec<f32> {
    let raw_dim = crate::models::specs::get_raw_feature_count_for_timeframe(tf);
    if k_idx < 19 || k_idx >= klines.len() {
        return vec![0.0f32; raw_dim];
    }

    let k = &klines[k_idx];
    let prev_close_10 = klines[k_idx.saturating_sub(10)].close;
    let avg_vol = volume_sma[k_idx];

    let f1 = if avg_vol > 0.0 { k.volume / avg_vol } else { 1.0 };
    let f2 = (k.high - k.low) / k.open;
    let f3 = (k.high - k.open) / k.open;
    let f4 = (k.open - k.low) / k.open;
    let f5 = (k.open - k.close) / k.open;
    let f6 = if prev_close_10 > 0.0 { (k.close - prev_close_10) / prev_close_10 } else { 0.0 };

    let mut sum_xy = 0.0;
    for i in 0..10 {
        let val = klines[k_idx - 9 + i].close;
        sum_xy += (i as f64 - 4.5) * val;
    }
    let slope = sum_xy / 82.5;
    let f7 = if k.close > 0.0 { slope / k.close } else { 0.0 };

    let mut highest = f64::NEG_INFINITY;
    let mut lowest = f64::INFINITY;
    for j in (k_idx - 9)..=k_idx {
        let kh = klines[j].high;
        let kl = klines[j].low;
        if kh > highest { highest = kh; }
        if kl < lowest { lowest = kl; }
    }
    let range = highest - lowest;
    let f8 = if range > 0.0 { (k.close - lowest) / range } else { 0.5 };

    let mut sum = 0.0;
    for j in (k_idx - 9)..=k_idx {
        sum += klines[j].close;
    }
    let mean = sum / 10.0;
    let mut sum_sq_diff = 0.0;
    for j in (k_idx - 9)..=k_idx {
        let diff = klines[j].close - mean;
        sum_sq_diff += diff * diff;
    }
    let std_dev = (sum_sq_diff / 10.0).sqrt();
    let f9 = if std_dev > 0.0 { (k.close - mean) / std_dev } else { 0.0 };

    let mut tr_sum = 0.0;
    for j in (k_idx - 9)..=k_idx {
        let kh = klines[j].high;
        let kl = klines[j].low;
        let prev_c = klines[j - 1].close;
        let tr = (kh - kl)
            .max((kh - prev_c).abs())
            .max((kl - prev_c).abs());
        tr_sum += tr;
    }
    let atr = tr_sum / 10.0;
    let f10 = if k.close > 0.0 { atr / k.close } else { 0.0 };

    let s_t = compute_markov_state(klines, k_idx);
    let s_prev = compute_markov_state(klines, k_idx.saturating_sub(1));
    let f11 = s_t / 5.0;
    let f12 = s_prev / 5.0;

    let f13 = if volume_sma_20[k_idx] > 0.0 { k.volume / volume_sma_20[k_idx] } else { 1.0 };
    let f14 = if volume_sma_20[k_idx] > 0.0 { volume_sma_5[k_idx] / volume_sma_20[k_idx] } else { 1.0 };
    let f15 = if f13 > 0.0 { f2 / (f13 + 1e-5) } else { 0.0 };
    let f16 = if k.close > 0.0 { macd_hist[k_idx] / k.close } else { 0.0 };

    let f17 = if ema_20[k_idx] > 0.0 { (k.close - ema_20[k_idx]) / ema_20[k_idx] } else { 0.0 };

    let mut sum_xy_prev = 0.0;
    let prev_k_idx = k_idx.saturating_sub(1);
    for i in 0..10 {
        let val = klines[prev_k_idx - 9 + i].close;
        sum_xy_prev += (i as f64 - 4.5) * val;
    }
    let slope_prev = sum_xy_prev / 82.5;
    let f18 = if k.close > 0.0 { (slope - slope_prev) / k.close } else { 0.0 };

    let f19 = if ema_200_hl2[k_idx] > 0.0 { (k.close - ema_200_hl2[k_idx]) / ema_200_hl2[k_idx] } else { 0.0 };
    let f20 = if atr_50[k_idx] > 0.0 { atr_5[k_idx] / atr_50[k_idx] } else { 1.0 };
    let f21 = if k.close > ema_200_hl2[k_idx] { 1.0 } else { 0.0 };

    let net_change = (k.close - klines[k_idx.saturating_sub(10)].close).abs();
    let mut sum_diffs = 0.0;
    for j in (k_idx - 9)..=k_idx {
        sum_diffs += (klines[j].close - klines[j - 1].close).abs();
    }
    let f22 = if sum_diffs > 0.0 { net_change / sum_diffs } else { 0.0 };

    let mut sum_vpt = 0.0;
    for j in (k_idx - 9)..=k_idx {
        sum_vpt += (klines[j].close - klines[j - 1].close) * klines[j].volume;
    }
    let f23 = if k.close > 0.0 && avg_vol > 0.0 { sum_vpt / (k.close * avg_vol) } else { 0.0 };

    let f24 = if mean > 0.0 { (highest - lowest) / mean } else { 0.0 };
    let f2_prev_10 = (klines[k_idx.saturating_sub(10)].high - klines[k_idx.saturating_sub(10)].low) / klines[k_idx.saturating_sub(10)].open;
    let f25 = if f2_prev_10 > 0.0 { (f2 - f2_prev_10) / f2_prev_10 } else { 0.0 };
    let f26 = (k.close - k.open).signum() * (k.volume + 1.0).ln();

    let mut gain_sum = 0.0;
    let mut loss_sum = 0.0;
    for i in 0..14 {
        let curr = klines[k_idx - 13 + i].close;
        let prev = klines[k_idx - 14 + i].close;
        let diff = curr - prev;
        if diff > 0.0 {
            gain_sum += diff;
        } else {
            loss_sum -= diff;
        }
    }
    let rsi = if gain_sum + loss_sum > 0.0 {
        gain_sum / (gain_sum + loss_sum)
    } else {
        0.5
    };
    let f27 = rsi - 0.5;

    let dt = Utc.timestamp_millis_opt(k.timestamp).unwrap();
    let hour = dt.hour() as f64;
    let day_of_week = dt.weekday().num_days_from_monday() as f64;

    let hour_sin = (hour * 2.0 * std::f64::consts::PI / 24.0).sin();
    let hour_cos = (hour * 2.0 * std::f64::consts::PI / 24.0).cos();
    let day_sin = (day_of_week * 2.0 * std::f64::consts::PI / 7.0).sin();
    let day_cos = (day_of_week * 2.0 * std::f64::consts::PI / 7.0).cos();
    let f28 = hour_sin;
    let f29 = hour_cos;
    let f30 = day_sin;
    let f31 = day_cos;

    let mut parkinson_sum = 0.0;
    let ln_2 = 2.0f64.ln();
    for i in 0..10 {
        let kh = klines[k_idx - 9 + i].high;
        let kl = klines[k_idx - 9 + i].low;
        let ratio = if kl > 0.0 { kh / kl } else { 1.0 };
        let ln_ratio = ratio.ln();
        parkinson_sum += ln_ratio * ln_ratio;
    }
    let f32 = (parkinson_sum / (40.0 * ln_2)).sqrt();

    let mut frac_diff = 0.0;
    let weights = [1.0, -0.4, -0.12, -0.064, -0.0416, -0.029952, -0.0229632, -0.01837056, -0.015155712, -0.0127981568];
    let log_close = k.close.ln();
    for i in 0..10 {
        let price = klines[k_idx - i].close;
        frac_diff += weights[i] * (price.ln() - log_close);
    }
    let f33 = frac_diff;

    let tf_upper = tf.to_uppercase();
    let is_daily = tf_upper == "1D";

    let mut result = Vec::with_capacity(raw_dim);
    result.extend_from_slice(&[
        f1 as f32, f2 as f32, f3 as f32, f4 as f32, f5 as f32, f6 as f32, f7 as f32, f8 as f32,
        f9 as f32, f10 as f32, f11 as f32, f12 as f32, f13 as f32, f14 as f32, f15 as f32,
        f16 as f32, f17 as f32, f18 as f32, f19 as f32, f20 as f32, f21 as f32, f22 as f32,
        f23 as f32, f24 as f32, f25 as f32, f26 as f32, f27 as f32,
    ]);

    // f28 (hour_sin) y f29 (hour_cos) se omiten en 1D porque las velas diarias son constantes a las 00:00 UTC
    if !is_daily {
        result.push(f28 as f32);
        result.push(f29 as f32);
    }

    result.extend_from_slice(&[
        f30 as f32, f31 as f32, f32 as f32, f33 as f32,
    ]);

    // Feature explícita de Hora del Día para modelos 1H e intradiarios
    if tf_upper == "1H" || tf_upper == "15M" || tf_upper == "5M" || tf_upper == "1M" {
        let hour_of_day_normalized = (hour / 23.0) as f32;
        result.push(hour_of_day_normalized);
    }

    result
}
