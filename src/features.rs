pub mod timeframe_features;

use crate::db::Kline;
pub use timeframe_features::compute_single_candle_features_by_tf;

#[inline(always)]
pub fn compute_single_candle_raw_features(
    klines: &[Kline],
    k_idx: usize,
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
) -> [f32; 33] {
    let vec = compute_single_candle_features_by_tf(
        klines, k_idx, "4H", volume_sma, volume_sma_5, volume_sma_20,
        macd_hist, ema_20, ema_200_hl2, atr_5, atr_50,
    );
    let mut arr = [0.0f32; 33];
    for i in 0..33.min(vec.len()) {
        arr[i] = vec[i];
    }
    arr
}

pub fn precalculate_dataset_raw_features_for_tf(
    klines: &[Kline],
    tf: &str,
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
) -> ndarray::Array2<f32> {
    let n = klines.len();
    let raw_dim = crate::models::specs::get_raw_feature_count_for_timeframe(tf);
    let mut raw = ndarray::Array2::zeros((n, raw_dim));
    for i in 0..n {
        if i >= 19 {
            let feat = compute_single_candle_features_by_tf(
                klines, i, tf, volume_sma, volume_sma_5, volume_sma_20,
                macd_hist, ema_20, ema_200_hl2, atr_5, atr_50,
            );
            for c in 0..raw_dim.min(feat.len()) {
                raw[[i, c]] = feat[c];
            }
        }
    }
    raw
}

pub fn precalculate_dataset_raw_features(
    klines: &[Kline],
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
) -> ndarray::Array2<f32> {
    precalculate_dataset_raw_features_for_tf(
        klines, "4H", volume_sma, volume_sma_5, volume_sma_20,
        macd_hist, ema_20, ema_200_hl2, atr_5, atr_50,
    )
}

#[inline(always)]
pub fn extract_cnn1d_features_from_raw(
    raw_features: &ndarray::Array2<f32>,
    idx: usize,
    window_size: usize,
    posicion_actual: f32,
) -> Option<ndarray::Array2<f32>> {
    if idx < window_size + 20 || idx >= raw_features.nrows() {
        return None;
    }

    let raw_cols = raw_features.ncols();
    let total_cols = raw_cols + 1;
    let start_idx = idx + 1 - window_size;
    let mut feat_cnn = ndarray::Array2::zeros((window_size, total_cols));

    for col in 0..raw_cols {
        let mut sum = 0.0f32;
        for w in 0..window_size {
            let val = raw_features[[start_idx + w, col]];
            feat_cnn[[w, col]] = val;
            sum += val;
        }
        let mean = sum / (window_size as f32);

        let mut sum_sq_diff = 0.0f32;
        for w in 0..window_size {
            let diff = feat_cnn[[w, col]] - mean;
            sum_sq_diff += diff * diff;
        }
        let mut std = (sum_sq_diff / (window_size as f32)).sqrt();
        if std < 1e-5 {
            std = 1.0;
        }

        for w in 0..window_size {
            feat_cnn[[w, col]] = (feat_cnn[[w, col]] - mean) / std;
        }
    }

    let pos_col = total_cols - 1;
    for w in 0..window_size {
        feat_cnn[[w, pos_col]] = posicion_actual;
    }

    Some(feat_cnn)
}

pub fn extract_luzia_features_for_tf(
    klines: &[Kline],
    idx: usize,
    window_size: usize,
    tf: &str,
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
) -> Option<ndarray::Array2<f32>> {
    if idx < window_size + 20 {
        return None;
    }

    let raw_dim = crate::models::specs::get_raw_feature_count_for_timeframe(tf);
    let mut features = ndarray::Array2::zeros((window_size, raw_dim));

    for w in 0..window_size {
        let k_idx = idx - window_size + 1 + w;
        let feat = compute_single_candle_features_by_tf(
            klines, k_idx, tf, volume_sma, volume_sma_5, volume_sma_20,
            macd_hist, ema_20, ema_200_hl2, atr_5, atr_50,
        );
        for c in 0..raw_dim.min(feat.len()) {
            features[[w, c]] = feat[c];
        }
    }

    for col in 0..raw_dim {
        let mut sum = 0.0f32;
        for w in 0..window_size {
            sum += features[[w, col]];
        }
        let mean = sum / window_size as f32;
        
        let mut sum_sq_diff = 0.0f32;
        for w in 0..window_size {
            let diff = features[[w, col]] - mean;
            sum_sq_diff += diff * diff;
        }
        let mut std = (sum_sq_diff / window_size as f32).sqrt();
        if std < 1e-5 {
            std = 1.0;
        }
        
        for w in 0..window_size {
            features[[w, col]] = (features[[w, col]] - mean) / std;
        }
    }

    Some(features)
}

pub fn extract_luzia_features(
    klines: &[Kline],
    idx: usize,
    window_size: usize,
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
) -> Option<ndarray::Array2<f32>> {
    extract_luzia_features_for_tf(
        klines, idx, window_size, "4H", volume_sma, volume_sma_5, volume_sma_20,
        macd_hist, ema_20, ema_200_hl2, atr_5, atr_50,
    )
}

pub fn extract_cnn1d_features_for_tf(
    klines: &[Kline],
    idx: usize,
    window_size: usize,
    tf: &str,
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
    posicion_actual: f32,
) -> Option<ndarray::Array2<f32>> {
    let feat_luzia = extract_luzia_features_for_tf(
        klines,
        idx,
        window_size,
        tf,
        volume_sma,
        volume_sma_5,
        volume_sma_20,
        macd_hist,
        ema_20,
        ema_200_hl2,
        atr_5,
        atr_50,
    )?;
    let raw_dim = feat_luzia.ncols();
    let total_cols = raw_dim + 1;
    let mut feat_cnn = ndarray::Array2::zeros((window_size, total_cols));
    feat_cnn.slice_mut(ndarray::s![.., 0..raw_dim]).assign(&feat_luzia);
    let pos_col = total_cols - 1;
    for t in 0..window_size {
        feat_cnn[[t, pos_col]] = posicion_actual;
    }
    Some(feat_cnn)
}

pub fn extract_cnn1d_features(
    klines: &[Kline],
    idx: usize,
    window_size: usize,
    volume_sma: &[f64],
    volume_sma_5: &[f64],
    volume_sma_20: &[f64],
    macd_hist: &[f64],
    ema_20: &[f64],
    ema_200_hl2: &[f64],
    atr_5: &[f64],
    atr_50: &[f64],
    posicion_actual: f32,
) -> Option<ndarray::Array2<f32>> {
    extract_cnn1d_features_for_tf(
        klines,
        idx,
        window_size,
        "4H",
        volume_sma,
        volume_sma_5,
        volume_sma_20,
        macd_hist,
        ema_20,
        ema_200_hl2,
        atr_5,
        atr_50,
        posicion_actual,
    )
}

