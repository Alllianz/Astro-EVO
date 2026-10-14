pub use crate::cnn1d::{Cnn1DModel, Cnn1DOptimizerState};
use crate::db::{get_all_klines, Kline};
use crate::features::{extract_cnn1d_features_from_raw, precalculate_dataset_raw_features_for_tf};
use crate::indicators::*;
use rusqlite::Connection;
use std::error::Error;

#[derive(Clone, Debug)]
pub struct OnlineLearningConfig {
    pub tf: String,
    pub capital_percent: f64,
    pub leverage: f64,
    pub window_size: usize,
    pub kernel_size: usize,
    pub horizon: usize,
    pub learning_rate: f32,
    pub weight_decay: f32,
    pub l1_lambda: f32,
    pub seed_val: u32,
    pub pre_train_epochs: usize,
    pub intelligence: usize,
}


#[derive(Clone)]
pub struct AstroDataset {
    pub tf: String,
    pub horizon: usize,
    pub klines: Vec<Kline>,
    pub volume_sma: Vec<f64>,
    pub volume_sma_5: Vec<f64>,
    pub volume_sma_20: Vec<f64>,
    pub macd_hist: Vec<f64>,
    pub ema_20: Vec<f64>,
    pub ema_200_hl2: Vec<f64>,
    pub atr_5: Vec<f64>,
    pub atr_50: Vec<f64>,
    pub raw_features: ndarray::Array2<f32>,
    pub labels: Vec<usize>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AstroSubMetrics {

    pub capital_final: f32,
    pub net_profit: f64,
    pub total_trades: usize,
    pub longs_count: usize,
    pub shorts_count: usize,
    pub winrate: f64,
    pub profit_factor: f64,
    pub max_dd: f32,
    pub max_dd_nominal: f64,
    pub recovery_factor: f64,
    pub max_stagnation: usize,
    pub liquidations: usize,
    pub slope_is: f64,
    pub slope_oos: f64,
    pub slope_ratio: f64,
    pub capital_dd_ratio: f64,
    pub curve_r2: f64,
    pub r2_is: f64,
    pub r2_oos: f64,
    pub sortino_ratio: f64,
    pub ulcer_index: f64,
    pub smoothness_score: f64,
    pub autotuning_score: f64,
    pub img_filename: String,
    pub equity_curve: Vec<(f32, f32)>,
    #[serde(default)]
    pub trades: Vec<crate::statement::SimulatedTrade>,
}


pub fn calculate_equity_r2(curve: &[(f32, f32)]) -> f64 {
    let n = curve.len();
    if n < 2 {
        return 0.0;
    }

    let mut sum_x = 0.0f64;
    let mut sum_y = 0.0f64;
    for (i, &(_, eq)) in curve.iter().enumerate() {
        sum_x += i as f64;
        sum_y += eq as f64;
    }
    let mean_x = sum_x / (n as f64);
    let mean_y = sum_y / (n as f64);

    let mut s_xx = 0.0f64;
    let mut s_yy = 0.0f64;
    let mut s_xy = 0.0f64;

    for (i, &(_, eq)) in curve.iter().enumerate() {
        let dx = (i as f64) - mean_x;
        let dy = (eq as f64) - mean_y;
        s_xx += dx * dx;
        s_yy += dy * dy;
        s_xy += dx * dy;
    }

    if s_xx <= 1e-12 || s_yy <= 1e-12 {
        return 0.0;
    }

    let slope = s_xy / s_xx;
    if slope <= 0.0 {
        return 0.0;
    }

    let r2 = (s_xy * s_xy) / (s_xx * s_yy);
    r2.clamp(0.0, 1.0)
}

pub fn calculate_slope_ratio(slope_is: f64, slope_oos: f64) -> f64 {
    if slope_is <= 0.0 || slope_oos <= 0.0 {
        return 99.0;
    }
    slope_is / slope_oos
}

pub fn calculate_capital_max_dd_ratio(capital_final: f64, max_dd_nominal: f64) -> f64 {
    let safe_dd = if max_dd_nominal <= 0.0 { 1.0 } else { max_dd_nominal };
    if capital_final <= 0.0 {
        0.0
    } else {
        capital_final / safe_dd
    }
}

pub fn calculate_autotuning_score(slope_ratio: f64, capital_dd_ratio: f64, curve_r2: f64) -> f64 {
    slope_ratio * capital_dd_ratio * curve_r2
}

pub fn generate_astro_labels(klines: &[Kline], horizon: usize) -> Vec<usize> {
    let atr_50 = precalculate_atr(klines, 50);
    let mut labels = vec![2; klines.len()];
    for idx in 200..klines.len() {
        if idx + horizon < klines.len() {
            let current_price = klines[idx].close;
            let mut max_p = current_price;
            let mut min_p = current_price;
            for i in 1..=horizon {
                let k = &klines[idx + i];
                if k.high > max_p { max_p = k.high; }
                if k.low < min_p { min_p = k.low; }
            }
            let return_long = (max_p - current_price) / current_price;
            let return_short = (current_price - min_p) / current_price;
            
            let current_atr = atr_50[idx];
            let atr_ratio = if current_price > 0.0 { current_atr / current_price } else { 0.01 };
            let threshold = 0.5 * atr_ratio;

            if return_long > threshold && return_long > return_short {
                labels[idx] = 0; // LONG
            } else if return_short > threshold && return_short > return_long {
                labels[idx] = 1; // SHORT
            } else {
                labels[idx] = 2; // FLAT
            }
        }
    }
    labels
}

pub fn generate_astro_labels_for_tf(klines: &[Kline], tf: &str, horizon: usize) -> Vec<usize> {
    if tf.eq_ignore_ascii_case("1D") {
        // En 1D se utiliza el Método de la Triple Barrera para evitar liquidaciones tempranas y sesgos de orden temporal
        let cfg = crate::intelligence::TripleBarrierConfig::for_timeframe("1D", horizon, 10.0);
        crate::intelligence::generate_triple_barrier_labels(klines, &cfg)
    } else {
        generate_astro_labels(klines, horizon)
    }
}

pub fn prepare_dataset_with_horizon(conn_candles: &Connection, tf: &str, horizon: usize) -> Result<AstroDataset, Box<dyn Error>> {
    println!("Cargando velas para Astro Dataset en {} (Horizonte H={})...", tf, horizon);
    let klines = get_all_klines(conn_candles, tf)?;
    if klines.is_empty() {
        return Err("No hay velas en el timeframe especificado.".into());
    }

    let volume_sma = precalculate_volume_sma(&klines);
    let (volume_sma_5, volume_sma_20) = precalculate_volume_smas(&klines);
    let macd_hist = precalculate_macd_hist(&klines);
    let ema_20 = precalculate_ema_20(&klines);
    let ema_200_hl2 = precalculate_ema_200_hl2(&klines);
    let atr_5 = precalculate_atr(&klines, 5);
    let atr_50 = precalculate_atr(&klines, 50);

    println!("Etiquetando dataset Astro ({} velas, H={})...", klines.len(), horizon);
    let labels = generate_astro_labels_for_tf(&klines, tf, horizon);

    println!("Precalculando matriz de features de alta velocidad para {} ({} velas)...", tf, klines.len());
    let raw_features = precalculate_dataset_raw_features_for_tf(
        &klines,
        tf,
        &volume_sma,
        &volume_sma_5,
        &volume_sma_20,
        &macd_hist,
        &ema_20,
        &ema_200_hl2,
        &atr_5,
        &atr_50,
    );

    Ok(AstroDataset {
        tf: tf.to_string(),
        horizon,
        klines,
        volume_sma,
        volume_sma_5,
        volume_sma_20,
        macd_hist,
        ema_20,
        ema_200_hl2,
        atr_5,
        atr_50,
        raw_features,
        labels,
    })
}

impl AstroDataset {
    pub fn with_horizon(&self, new_horizon: usize) -> Self {
        if self.horizon == new_horizon {
            return self.clone();
        }
        let labels = generate_astro_labels_for_tf(&self.klines, &self.tf, new_horizon);
        Self {
            tf: self.tf.clone(),
            horizon: new_horizon,
            klines: self.klines.clone(),
            volume_sma: self.volume_sma.clone(),
            volume_sma_5: self.volume_sma_5.clone(),
            volume_sma_20: self.volume_sma_20.clone(),
            macd_hist: self.macd_hist.clone(),
            ema_20: self.ema_20.clone(),
            ema_200_hl2: self.ema_200_hl2.clone(),
            atr_5: self.atr_5.clone(),
            atr_50: self.atr_50.clone(),
            raw_features: self.raw_features.clone(),
            labels,
        }
    }
}

pub fn run_online_learning_simulation_with_model(
    dataset: &AstroDataset,
    config: &OnlineLearningConfig,
    mut model: Cnn1DModel,
) -> Result<(AstroSubMetrics, AstroSubMetrics, Cnn1DModel, Vec<(f32, f32)>, Vec<(f32, f32)>), Box<dyn Error>> {
    let klines = &dataset.klines;
    if klines.len() < config.window_size + 205 {
        return Err("No hay suficientes velas para el backtest de Online Learning.".into());
    }

    let is_start_ts = parse_date_to_timestamp("20-04-2020").unwrap_or(0);
    let oos_start_ts = parse_date_to_timestamp("01-01-2023").unwrap_or(0);

    let mut is_start_idx = klines.binary_search_by_key(&is_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    let min_idx = config.window_size + 200;
    if is_start_idx < min_idx {
        is_start_idx = min_idx;
    }

    let mut oos_start_idx = klines.binary_search_by_key(&oos_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    if oos_start_idx <= is_start_idx || oos_start_idx >= klines.len() {
        oos_start_idx = is_start_idx + 100;
    }

    let mut state = Cnn1DOptimizerState::new_for_model(&model);

    let mut cap_nom = 10000.0f64;
    let mut pos_nom = 0.0f32;
    let mut ent_nom = 0.0f64;
    let mut eq_curve_nom = Vec::new();
    eq_curve_nom.push((is_start_idx as f32, 10000.0f32));
    let mut trade_pnls_nom = Vec::new();
    let mut longs_nom = 0;
    let mut shorts_nom = 0;
    let mut liq_nom = 0;
    let mut max_stag_nom = 0;
    let mut cur_stag_nom = 0;
    let mut peak_nom = 10000.0f64;
    let mut max_dd_nom = 0.0f64;

    let mut trades_nom: Vec<crate::statement::SimulatedTrade> = Vec::new();
    let mut trade_id_counter: usize = 1;
    let mut cur_trade_open_idx: usize = 0;
    let mut cur_trade_open_ts: i64 = 0;
    let mut cur_trade_cap_at_open: f64 = 10000.0;
    let mut cur_trade_action = crate::statement::TradeAction::Buy;
    let mut cur_trade_units: f64 = 0.0;
    let mut cur_trade_comision_open: f64 = 0.0;
    let mut cur_trade_sl: f64 = 0.0;

    let mut cap_pct = 10000.0f64;
    let mut cap_pct_at_open = 10000.0f64;
    let mut pos_pct = 0.0f32;
    let mut ent_pct = 0.0f64;
    let mut eq_curve_pct = Vec::new();
    eq_curve_pct.push((is_start_idx as f32, 10000.0f32));
    let mut trade_pnls_pct = Vec::new();
    let mut longs_pct = 0;
    let mut shorts_pct = 0;
    let mut liq_pct = 0;
    let mut max_stag_pct = 0;
    let mut cur_stag_pct = 0;
    let mut peak_pct = 10000.0f64;
    let mut max_dd_nom_pct = 0.0f64;
    let mut max_dd_pct_rel = 0.0f64;
    let mut replay_buffer: Vec<(ndarray::Array2<f32>, usize)> = Vec::new();

    for i in is_start_idx..klines.len() {
        if pos_nom != 0.0 {
            let margen_nom = 10000.0 * (config.capital_percent / 100.0);
            let comision_apertura = margen_nom * config.leverage * 0.0005;
            let pnl_pct_limit = (1.0 / config.leverage) - 0.0005;
            let precio_liq = if pos_nom == 1.0 {
                ent_nom * (1.0 - pnl_pct_limit)
            } else {
                ent_nom * (1.0 + pnl_pct_limit)
            };

            let se_liquida = if pos_nom == 1.0 {
                klines[i].low <= precio_liq
            } else {
                klines[i].high >= precio_liq
            };

            if se_liquida {
                let net_pnl = -margen_nom - comision_apertura;
                trade_pnls_nom.push(net_pnl);
                if pos_nom == 1.0 { longs_nom += 1; } else { shorts_nom += 1; }
                cap_nom -= margen_nom;
                if cap_nom <= 0.0 { cap_nom = 0.0; }

                trades_nom.push(crate::statement::SimulatedTrade {
                    trade_id: trade_id_counter,
                    open_idx: cur_trade_open_idx,
                    close_idx: i,
                    open_timestamp: cur_trade_open_ts,
                    close_timestamp: klines[i].timestamp,
                    symbol: "BTCUSDT".to_string(),
                    action: cur_trade_action,
                    units_lots: cur_trade_units,
                    sl_price: cur_trade_sl,
                    tp_price: 0.0,
                    open_price: ent_nom,
                    close_price: precio_liq,
                    commission: -comision_apertura,
                    swap: 0.0,
                    net_profit: net_pnl,
                    capital_at_open: cur_trade_cap_at_open,
                    is_liquidation: true,
                });
                trade_id_counter += 1;

                pos_nom = 0.0;
                liq_nom += 1;
            }
        }

        if pos_pct != 0.0 {
            let margen_pct = cap_pct_at_open * (config.capital_percent / 100.0);
            let comision_apertura = margen_pct * config.leverage * 0.0005;
            let pnl_pct_limit = (1.0 / config.leverage) - 0.0005;
            let precio_liq = if pos_pct == 1.0 {
                ent_pct * (1.0 - pnl_pct_limit)
            } else {
                ent_pct * (1.0 + pnl_pct_limit)
            };

            let se_liquida = if pos_pct == 1.0 {
                klines[i].low <= precio_liq
            } else {
                klines[i].high >= precio_liq
            };

            if se_liquida {
                let net_pnl = -margen_pct - comision_apertura;
                trade_pnls_pct.push(net_pnl);
                if pos_pct == 1.0 { longs_pct += 1; } else { shorts_pct += 1; }
                cap_pct -= margen_pct;
                if cap_pct <= 0.0 { cap_pct = 0.0; }
                pos_pct = 0.0;
                liq_pct += 1;
            }
        }

        if let Some(feat) = extract_cnn1d_features_from_raw(
            &dataset.raw_features,
            i,
            model.window_size,
            pos_nom,
        ) {
            let probs = model.predict(&feat);
            let p_long = probs[0];
            let p_short = probs[1];
            let p_flat = probs[2];

            let atr_ratio = if klines[i].close > 0.0 { dataset.atr_50[i] / klines[i].close } else { 0.01 };
            let dynamic_thr = (0.40 + 0.15 * (atr_ratio * 100.0).clamp(0.0, 1.0)).clamp(0.40, 0.55) as f32;

            let pos_deseada;
            if p_long > p_short && p_long > p_flat && p_long > dynamic_thr {
                pos_deseada = 1.0;
            } else if p_short > p_long && p_short > p_flat && p_short > dynamic_thr {
                pos_deseada = -1.0;
            } else {
                pos_deseada = 0.0;
            }

            let pos_deseada_nom = if cap_nom <= 0.0 { 0.0 } else { pos_deseada };
            let pos_deseada_pct = if cap_pct <= 0.0 { 0.0 } else { pos_deseada };

            if pos_deseada_nom != pos_nom {
                if pos_nom != 0.0 {
                    let pnl_pct_trade = if pos_nom == 1.0 { (klines[i].close - ent_nom) / ent_nom } else { (ent_nom - klines[i].close) / ent_nom };
                    let pnl = 10000.0 * (config.capital_percent / 100.0) * pnl_pct_trade * config.leverage;
                    let vol_nocional = 10000.0 * (config.capital_percent / 100.0) * config.leverage * (klines[i].close / ent_nom);
                    let comision_cierre = vol_nocional * 0.0005;
                    let net_pnl = pnl - comision_cierre;
                    trade_pnls_nom.push(net_pnl);
                    if pos_nom == 1.0 { longs_nom += 1; } else { shorts_nom += 1; }
                    cap_nom += net_pnl;
                    if cap_nom <= 0.0 { cap_nom = 0.0; liq_nom += 1; }

                    let total_commission = cur_trade_comision_open + comision_cierre;
                    let trade_profit = net_pnl - cur_trade_comision_open;

                    trades_nom.push(crate::statement::SimulatedTrade {
                        trade_id: trade_id_counter,
                        open_idx: cur_trade_open_idx,
                        close_idx: i,
                        open_timestamp: cur_trade_open_ts,
                        close_timestamp: klines[i].timestamp,
                        symbol: "BTCUSDT".to_string(),
                        action: cur_trade_action,
                        units_lots: cur_trade_units,
                        sl_price: cur_trade_sl,
                        tp_price: 0.0,
                        open_price: ent_nom,
                        close_price: klines[i].close,
                        commission: -total_commission,
                        swap: 0.0,
                        net_profit: trade_profit,
                        capital_at_open: cur_trade_cap_at_open,
                        is_liquidation: false,
                    });
                    trade_id_counter += 1;
                }
                if pos_deseada_nom != 0.0 {
                    ent_nom = klines[i].close;
                    let comision_apertura = 10000.0 * (config.capital_percent / 100.0) * config.leverage * 0.0005;
                    cap_nom -= comision_apertura;
                    if cap_nom <= 0.0 { cap_nom = 0.0; }

                    cur_trade_open_idx = i;
                    cur_trade_open_ts = klines[i].timestamp;
                    cur_trade_cap_at_open = cap_nom + comision_apertura;
                    cur_trade_action = if pos_deseada_nom == 1.0 { crate::statement::TradeAction::Buy } else { crate::statement::TradeAction::Sell };
                    let notional = 10000.0 * (config.capital_percent / 100.0) * config.leverage;
                    cur_trade_units = if ent_nom > 0.0 { notional / ent_nom } else { 0.0 };
                    cur_trade_comision_open = comision_apertura;
                    let pnl_pct_limit = (1.0 / config.leverage) - 0.0005;
                    cur_trade_sl = if pos_deseada_nom == 1.0 {
                        ent_nom * (1.0 - pnl_pct_limit)
                    } else {
                        ent_nom * (1.0 + pnl_pct_limit)
                    };
                }
                pos_nom = pos_deseada_nom;
            }

            let floating_pnl_nom = if pos_nom != 0.0 {
                let pnl_pct_trade = if pos_nom == 1.0 { (klines[i].close - ent_nom) / ent_nom } else { (ent_nom - klines[i].close) / ent_nom };
                10000.0 * (config.capital_percent / 100.0) * pnl_pct_trade * config.leverage
            } else { 0.0 };

            let equidad_nom = (cap_nom + floating_pnl_nom).max(0.0);
            eq_curve_nom.push((i as f32, equidad_nom as f32));

            if equidad_nom > peak_nom {
                peak_nom = equidad_nom;
                if cur_stag_nom > max_stag_nom { max_stag_nom = cur_stag_nom; }
                cur_stag_nom = 0;
            } else { cur_stag_nom += 1; }
            let dd_nom = peak_nom - equidad_nom;
            if dd_nom > max_dd_nom { max_dd_nom = dd_nom; }

            if pos_deseada_pct != pos_pct {
                if pos_pct != 0.0 {
                    let pnl_pct_trade = if pos_pct == 1.0 { (klines[i].close - ent_pct) / ent_pct } else { (ent_pct - klines[i].close) / ent_pct };
                    let pnl = cap_pct_at_open * (config.capital_percent / 100.0) * pnl_pct_trade * config.leverage;
                    let vol_nocional = cap_pct_at_open * (config.capital_percent / 100.0) * config.leverage * (klines[i].close / ent_pct);
                    let net_pnl = pnl - (vol_nocional * 0.0005);
                    trade_pnls_pct.push(net_pnl);
                    if pos_pct == 1.0 { longs_pct += 1; } else { shorts_pct += 1; }
                    cap_pct += net_pnl;
                    if cap_pct <= 0.0 { cap_pct = 0.0; liq_pct += 1; }
                }
                if pos_deseada_pct != 0.0 {
                    ent_pct = klines[i].close;
                    cap_pct_at_open = cap_pct;
                    let comision_apertura = cap_pct_at_open * (config.capital_percent / 100.0) * config.leverage * 0.0005;
                    cap_pct -= comision_apertura;
                    if cap_pct <= 0.0 { cap_pct = 0.0; }
                }
                pos_pct = pos_deseada_pct;
            }

            let floating_pnl_pct = if pos_pct != 0.0 {
                let pnl_pct_trade = if pos_pct == 1.0 { (klines[i].close - ent_pct) / ent_pct } else { (ent_pct - klines[i].close) / ent_pct };
                cap_pct_at_open * (config.capital_percent / 100.0) * pnl_pct_trade * config.leverage
            } else { 0.0 };

            let equidad_pct = (cap_pct + floating_pnl_pct).max(0.0);
            eq_curve_pct.push((i as f32, equidad_pct as f32));

            if equidad_pct > peak_pct {
                peak_pct = equidad_pct;
                if cur_stag_pct > max_stag_pct { max_stag_pct = cur_stag_pct; }
                cur_stag_pct = 0;
            } else { cur_stag_pct += 1; }
            let dd_pct = peak_pct - equidad_pct;
            if dd_pct > max_dd_nom_pct { max_dd_nom_pct = dd_pct; }
            let dd_p = if peak_pct > 0.0 { (dd_pct / peak_pct) * 100.0 } else { 0.0 };
            if dd_p > max_dd_pct_rel { max_dd_pct_rel = dd_p; }

            let horizon = config.horizon;
            if i >= is_start_idx + horizon {
                let train_idx = i - horizon;
                if let Some(feat_past) = extract_cnn1d_features_from_raw(
                    &dataset.raw_features,
                    train_idx,
                    model.window_size,
                    0.0,
                ) {
                    let target_label_past = dataset.labels[train_idx];

                    // Learning Rate Adaptativo por Performance / Drawdown (Opción C):
                    // Si la equidad cae respecto a su pico histórico (Drawdown), se incrementa dinámicamente
                    // la velocidad de aprendizaje (hasta 3.0x) para recalibrar los pesos rápidamente.
                    let dd_pct_float = if peak_nom > 0.0 { ((peak_nom - equidad_nom) / peak_nom).clamp(0.0, 0.5) } else { 0.0 };
                    let adaptive_perf_factor = 1.0 + (dd_pct_float * 4.0); // 0% DD -> 1.0x, 10% DD -> 1.4x, 25% DD -> 2.0x, 50% DD -> 3.0x
                    let current_lr = config.learning_rate * adaptive_perf_factor as f32;

                    model.train_step_smoothed(&feat_past, target_label_past, current_lr, config.weight_decay, config.l1_lambda, &mut state, 0.15);
                    
                    if replay_buffer.len() >= 256 {
                        replay_buffer.remove(0);
                    }
                    replay_buffer.push((feat_past, target_label_past));

                    if replay_buffer.len() >= 10 {
                        let b_len = replay_buffer.len();
                        let sample_indices = [(i * 7) % b_len, (i * 13) % b_len, (i * 29) % b_len];
                        for &r_idx in &sample_indices {
                            let (r_feat, r_target) = &replay_buffer[r_idx];
                            model.train_step_smoothed(r_feat, *r_target, current_lr, config.weight_decay, config.l1_lambda, &mut state, 0.15);
                        }
                    }
                }
            }
        }
    }

    let last_idx = klines.len().saturating_sub(1);
    if pos_nom != 0.0 && last_idx >= cur_trade_open_idx {
        let pnl_pct_trade = if pos_nom == 1.0 { (klines[last_idx].close - ent_nom) / ent_nom } else { (ent_nom - klines[last_idx].close) / ent_nom };
        let pnl = 10000.0 * (config.capital_percent / 100.0) * pnl_pct_trade * config.leverage;
        let vol_nocional = 10000.0 * (config.capital_percent / 100.0) * config.leverage * (klines[last_idx].close / ent_nom);
        let comision_cierre = vol_nocional * 0.0005;
        let net_pnl = pnl - comision_cierre;
        trade_pnls_nom.push(net_pnl);
        if pos_nom == 1.0 { longs_nom += 1; } else { shorts_nom += 1; }
        cap_nom += net_pnl;
        if cap_nom <= 0.0 { liq_nom += 1; }

        let total_commission = cur_trade_comision_open + comision_cierre;
        let trade_profit = net_pnl - cur_trade_comision_open;

        trades_nom.push(crate::statement::SimulatedTrade {
            trade_id: trade_id_counter,
            open_idx: cur_trade_open_idx,
            close_idx: last_idx,
            open_timestamp: cur_trade_open_ts,
            close_timestamp: klines[last_idx].timestamp,
            symbol: "BTCUSDT".to_string(),
            action: cur_trade_action,
            units_lots: cur_trade_units,
            sl_price: cur_trade_sl,
            tp_price: 0.0,
            open_price: ent_nom,
            close_price: klines[last_idx].close,
            commission: -total_commission,
            swap: 0.0,
            net_profit: trade_profit,
            capital_at_open: cur_trade_cap_at_open,
            is_liquidation: false,
        });
    }

    if cur_stag_nom > max_stag_nom { max_stag_nom = cur_stag_nom; }
    if cur_stag_pct > max_stag_pct { max_stag_pct = cur_stag_pct; }

    let capital_final_nom = eq_curve_nom.last().map(|&(_, eq)| eq).unwrap_or(10000.0f32);
    let max_dd_pct_nom = ((max_dd_nom / 10000.0) * 100.0) as f32;

    let total_trades_nom = trade_pnls_nom.len();
    let wins_nom = trade_pnls_nom.iter().filter(|&&p| p > 0.0).count();
    let winrate_nom = if total_trades_nom > 0 { (wins_nom as f64 / total_trades_nom as f64) * 100.0 } else { 0.0 };
    let gross_profit_nom: f64 = trade_pnls_nom.iter().filter(|&&p| p > 0.0).sum();
    let gross_loss_nom: f64 = trade_pnls_nom.iter().filter(|&&p| p < 0.0).sum::<f64>().abs();
    let profit_factor_nom = if gross_loss_nom > 0.0 { gross_profit_nom / gross_loss_nom } else { if gross_profit_nom > 0.0 { f64::INFINITY } else { 0.0 } };
    let net_profit_nom = (capital_final_nom as f64) - 10000.0;

    let capital_final_pct = eq_curve_pct.last().map(|&(_, eq)| eq).unwrap_or(10000.0f32);
    let max_dd_pct_pct = max_dd_pct_rel as f32;

    let total_trades_pct = trade_pnls_pct.len();
    let wins_pct = trade_pnls_pct.iter().filter(|&&p| p > 0.0).count();
    let winrate_pct = if total_trades_pct > 0 { (wins_pct as f64 / total_trades_pct as f64) * 100.0 } else { 0.0 };
    let gross_profit_pct: f64 = trade_pnls_pct.iter().filter(|&&p| p > 0.0).sum();
    let gross_loss_pct: f64 = trade_pnls_pct.iter().filter(|&&p| p < 0.0).sum::<f64>().abs();
    let profit_factor_pct = if gross_loss_pct > 0.0 { gross_profit_pct / gross_loss_pct } else { if gross_profit_pct > 0.0 { f64::INFINITY } else { 0.0 } };
    let net_profit_pct = (capital_final_pct as f64) - 10000.0;

    let is_candles = (oos_start_idx - is_start_idx) as f64;
    let oos_candles = (klines.len() - oos_start_idx) as f64;

    let eq_at_oos_start_nom = eq_curve_nom.iter()
        .find(|&&(x, _)| (x as usize) >= oos_start_idx)
        .map(|&(_, eq)| eq)
        .unwrap_or(10000.0f32);
    let is_net_profit_nom = (eq_at_oos_start_nom - 10000.0) as f64;
    let oos_net_profit_nom = (capital_final_nom - eq_at_oos_start_nom) as f64;
    let slope_is_nom = if is_candles > 0.0 { is_net_profit_nom / is_candles } else { 0.0 };
    let slope_oos_nom = if oos_candles > 0.0 { oos_net_profit_nom / oos_candles } else { 0.0 };
    let slope_ratio_nom = calculate_slope_ratio(slope_is_nom, slope_oos_nom);

    let capital_dd_ratio_nom = calculate_capital_max_dd_ratio(capital_final_nom as f64, max_dd_nom);
    let eval_lin_nom = crate::metrics::evaluate_equity_linearity(&eq_curve_nom, oos_start_idx);
    let smooth_nom = crate::metrics::calculate_equity_smoothness(&eq_curve_nom, &trade_pnls_nom, eval_lin_nom.composite_linearity_score);
    let autotuning_score_nom = calculate_autotuning_score(slope_ratio_nom, capital_dd_ratio_nom, eval_lin_nom.composite_linearity_score);

    let img_filename = format!("online_learning_{}_W{}_K{}_H{}.png", config.tf, config.window_size, config.kernel_size, config.horizon);

    let m_nom = AstroSubMetrics {
        capital_final: capital_final_nom,
        net_profit: net_profit_nom,
        total_trades: total_trades_nom,
        longs_count: longs_nom,
        shorts_count: shorts_nom,
        winrate: winrate_nom,
        profit_factor: profit_factor_nom,
        max_dd: max_dd_pct_nom,
        max_dd_nominal: max_dd_nom,
        recovery_factor: if max_dd_nom > 0.0 { net_profit_nom / max_dd_nom } else { 0.0 },
        max_stagnation: max_stag_nom,
        liquidations: liq_nom,
        slope_is: slope_is_nom,
        slope_oos: slope_oos_nom,
        slope_ratio: slope_ratio_nom,
        capital_dd_ratio: capital_dd_ratio_nom,
        curve_r2: eval_lin_nom.r2_global,
        r2_is: eval_lin_nom.r2_is,
        r2_oos: eval_lin_nom.r2_oos,
        sortino_ratio: smooth_nom.sortino_ratio,
        ulcer_index: smooth_nom.ulcer_index,
        smoothness_score: smooth_nom.smoothness_score,
        autotuning_score: autotuning_score_nom,
        img_filename: img_filename.clone(),
        equity_curve: eq_curve_nom.clone(),
        trades: trades_nom,
    };

    let eq_at_oos_start_pct = eq_curve_pct.iter()
        .find(|&&(x, _)| (x as usize) >= oos_start_idx)
        .map(|&(_, eq)| eq)
        .unwrap_or(10000.0f32);
    let is_net_profit_pct = (eq_at_oos_start_pct - 10000.0) as f64;
    let oos_net_profit_pct = (capital_final_pct - eq_at_oos_start_pct) as f64;
    let slope_is_pct = if is_candles > 0.0 { is_net_profit_pct / is_candles } else { 0.0 };
    let slope_oos_pct = if oos_candles > 0.0 { oos_net_profit_pct / oos_candles } else { 0.0 };
    let slope_ratio_pct = calculate_slope_ratio(slope_is_pct, slope_oos_pct);

    let capital_dd_ratio_pct = calculate_capital_max_dd_ratio(capital_final_pct as f64, max_dd_nom_pct);
    let eval_lin_pct = crate::metrics::evaluate_equity_linearity(&eq_curve_pct, oos_start_idx);
    let smooth_pct = crate::metrics::calculate_equity_smoothness(&eq_curve_pct, &trade_pnls_pct, eval_lin_pct.composite_linearity_score);
    let autotuning_score_pct = calculate_autotuning_score(slope_ratio_pct, capital_dd_ratio_pct, eval_lin_pct.composite_linearity_score);

    let m_pct = AstroSubMetrics {
        capital_final: capital_final_pct,
        net_profit: net_profit_pct,
        total_trades: total_trades_pct,
        longs_count: longs_pct,
        shorts_count: shorts_pct,
        winrate: winrate_pct,
        profit_factor: profit_factor_pct,
        max_dd: max_dd_pct_pct,
        max_dd_nominal: max_dd_nom_pct,
        recovery_factor: if max_dd_nom_pct > 0.0 { net_profit_pct / max_dd_nom_pct } else { 0.0 },
        max_stagnation: max_stag_pct,
        liquidations: liq_pct,
        slope_is: slope_is_pct,
        slope_oos: slope_oos_pct,
        slope_ratio: slope_ratio_pct,
        capital_dd_ratio: capital_dd_ratio_pct,
        curve_r2: eval_lin_pct.r2_global,
        r2_is: eval_lin_pct.r2_is,
        r2_oos: eval_lin_pct.r2_oos,
        sortino_ratio: smooth_pct.sortino_ratio,
        ulcer_index: smooth_pct.ulcer_index,
        smoothness_score: smooth_pct.smoothness_score,
        autotuning_score: autotuning_score_pct,
        img_filename,
        equity_curve: eq_curve_pct.clone(),
        trades: Vec::new(),
    };

    Ok((m_nom, m_pct, model, eq_curve_nom, eq_curve_pct))
}

pub fn run_online_learning_incremental_epochs(
    dataset: &AstroDataset,
    base_config: &OnlineLearningConfig,
    epochs_to_run: &[usize],
) -> Result<Vec<(usize, AstroSubMetrics, AstroSubMetrics, Cnn1DModel, Vec<(f32, f32)>, Vec<(f32, f32)>)>, Box<dyn Error>> {
    let klines = &dataset.klines;
    if klines.len() < base_config.window_size + 205 {
        return Err("No hay suficientes velas para el backtest de Online Learning.".into());
    }

    let is_start_ts = parse_date_to_timestamp("20-04-2020").unwrap_or(0);
    let oos_start_ts = parse_date_to_timestamp("01-01-2023").unwrap_or(0);

    let mut is_start_idx = klines.binary_search_by_key(&is_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    let min_idx = base_config.window_size + 200;
    if is_start_idx < min_idx {
        is_start_idx = min_idx;
    }

    let mut oos_start_idx = klines.binary_search_by_key(&oos_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    if oos_start_idx <= is_start_idx || oos_start_idx >= klines.len() {
        oos_start_idx = is_start_idx + 100;
    }

    let mut sorted_epochs = epochs_to_run.to_vec();
    sorted_epochs.sort_unstable();
    sorted_epochs.dedup();

    let mut model = if base_config.intelligence > 0 {
        Cnn1DModel::new_with_intelligence(base_config.tf.clone(), base_config.window_size, base_config.kernel_size, base_config.seed_val, base_config.intelligence)
    } else {
        Cnn1DModel::new_for_timeframe(base_config.tf.clone(), base_config.window_size, base_config.kernel_size, base_config.seed_val)
    };
    let mut state = Cnn1DOptimizerState::new_for_model(&model);


    let is_samples: Vec<(ndarray::Array2<f32>, usize)> = (is_start_idx..oos_start_idx)
        .filter_map(|idx| {
            let feat = extract_cnn1d_features_from_raw(
                &dataset.raw_features,
                idx,
                model.window_size,
                0.0,
            )?;
            Some((feat, dataset.labels[idx]))
        })
        .collect();

    let mut results = Vec::new();
    let mut current_epoch = 0;

    for &target_epoch in &sorted_epochs {
        let delta = target_epoch.saturating_sub(current_epoch);
        if delta > 0 {
            println!("🔄 Pre-entrenando red para Época {} (pasos adicionales: {})...", target_epoch, delta);
            for step in 0..delta {
                for (feat, target) in &is_samples {
                    model.train_step_smoothed(feat, *target, base_config.learning_rate, base_config.weight_decay, base_config.l1_lambda, &mut state, 0.15);
                }
                println!("   ↳ Paso de pre-entrenamiento {}/{} completado.", step + 1, delta);
            }
            current_epoch = target_epoch;
        }

        println!("⚡ Ejecutando simulación Walk-Forward causa-efecto para Época {}...", target_epoch);
        let mut cfg_epoch = base_config.clone();
        cfg_epoch.pre_train_epochs = target_epoch;

        let (m_nom, m_pct, _trained_model, eq_nom, eq_pct) = run_online_learning_simulation_with_model(dataset, &cfg_epoch, model.clone())?;
        println!("✅ Época {} finalizada -> Profit Nominal: ${:.2} | WinRate: {:.2}% | Pendiente IS/OOS: {:.4} | Cap/MaxDD Ratio: {:.4}", target_epoch, m_nom.net_profit, m_nom.winrate, m_nom.slope_ratio, m_nom.capital_dd_ratio);
        results.push((target_epoch, m_nom, m_pct, model.clone(), eq_nom, eq_pct));
    }

    Ok(results)
}

#[derive(Clone, Debug)]
pub struct LeverageGridResult {
    pub leverage: f64,
    pub slope_ratio: f64,
    pub capital_dd_ratio: f64,
    pub net_profit: f64,
    pub max_dd_pct: f32,
    pub liquidations: usize,
    pub winrate: f64,
    pub profit_factor: f64,
    pub rank_slope: usize,
    pub rank_cap_dd: usize,
    pub rank_r2: usize,
    pub rank_smoothness: usize,
    pub avg_rank: f64,
    pub metrics_nom: AstroSubMetrics,
    pub metrics_pct: AstroSubMetrics,
    pub eq_nom: Vec<(f32, f32)>,
    pub eq_pct: Vec<(f32, f32)>,
}

pub fn run_leverage_grid_search_for_best_model(
    dataset: &AstroDataset,
    base_config: &OnlineLearningConfig,
    model: &Cnn1DModel,
    lev_start: usize,
    lev_end: usize,
) -> Result<Vec<LeverageGridResult>, Box<dyn Error>> {
    println!("\n🔍 [Rejilla Posterior de Apalancamiento] Evaluando eficiencias de {}X a {}X en 2 Lotes...", lev_start, lev_end);

    let mut raw_items = Vec::new();

    // Determinamos la división en 2 lotes (por defecto Lote 1: 10X..17X, Lote 2: 18X..25X)
    let mid_point = if lev_start == 10 && lev_end == 25 {
        17
    } else {
        lev_start + (lev_end - lev_start) / 2
    };

    // Lote 1: lev_start ..= mid_point
    println!("  ⚡ [Lote 1/2] Evaluando apalancamientos de {}X a {}X...", lev_start, mid_point);
    for lev_val in lev_start..=mid_point {
        let lev = lev_val as f64;
        let mut cfg = base_config.clone();
        cfg.leverage = lev;

        let (m_nom, m_pct, _trained_model, eq_nom, eq_pct) = run_online_learning_simulation_with_model(dataset, &cfg, model.clone())?;
        raw_items.push((lev, m_nom, m_pct, eq_nom, eq_pct));
    }

    // Lote 2: (mid_point + 1) ..= lev_end
    if mid_point < lev_end {
        println!("  ⚡ [Lote 2/2] Evaluando apalancamientos de {}X a {}X...", mid_point + 1, lev_end);
        for lev_val in (mid_point + 1)..=lev_end {
            let lev = lev_val as f64;
            let mut cfg = base_config.clone();
            cfg.leverage = lev;

            let (m_nom, m_pct, _trained_model, eq_nom, eq_pct) = run_online_learning_simulation_with_model(dataset, &cfg, model.clone())?;
            raw_items.push((lev, m_nom, m_pct, eq_nom, eq_pct));
        }
    }

    // Ranking Multicriterio Integrado (Slope Ratio, Capital/MaxDD, Linealidad R² y Suavidad)
    let rank_map = crate::metrics::compute_multicriteria_rankings(
        &raw_items,
        |item| format!("{:.0}", item.0),
        |item| item.1.slope_ratio,
        |item| item.1.capital_dd_ratio,
        |item| item.1.curve_r2,
        |item| item.1.smoothness_score,
    );

    let mut final_results = Vec::new();
    for (lev, m_nom, m_pct, eq_nom, eq_pct) in raw_items {
        let key = format!("{:.0}", lev);
        let rank_score = rank_map.get(&key).cloned().unwrap_or(crate::metrics::CandidateRankScore {
            candidate_id: key,
            rank_slope: 999,
            rank_cap_dd: 999,
            rank_r2: 999,
            rank_smoothness: 999,
            weighted_avg_rank: 999.0,
        });

        final_results.push(LeverageGridResult {
            leverage: lev,
            slope_ratio: m_nom.slope_ratio,
            capital_dd_ratio: m_nom.capital_dd_ratio,
            net_profit: m_nom.net_profit,
            max_dd_pct: m_nom.max_dd,
            liquidations: m_nom.liquidations,
            winrate: m_nom.winrate,
            profit_factor: m_nom.profit_factor,
            rank_slope: rank_score.rank_slope,
            rank_cap_dd: rank_score.rank_cap_dd,
            rank_r2: rank_score.rank_r2,
            rank_smoothness: rank_score.rank_smoothness,
            avg_rank: rank_score.weighted_avg_rank,
            metrics_nom: m_nom,
            metrics_pct: m_pct,
            eq_nom,
            eq_pct,
        });
    }

    // Ordenar de menor a mayor avg_rank (el promedio de posiciones más bajo gana)
    final_results.sort_by(|a, b| a.avg_rank.partial_cmp(&b.avg_rank).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n🏆 [RANKING DE APALANCAMIENTO MÁS EFICIENTE (10X - 25X)]");
    println!("{:<4} | {:<12} | {:<12} | {:<15} | {:<12} | {:<10} | {:<8} | {}",
        "Pos", "Apalancamiento", "Avg Rank", "Capital/MaxDD", "Slope Ratio", "Net Profit", "Max DD %", "Liquidaciones"
    );
    println!("---------------------------------------------------------------------------------------------------------");
    for (idx, item) in final_results.iter().enumerate() {
        println!("{:<4} | {:<12.0}X | {:<12.2} | {:<15.4} | {:<12.4} | ${:<9.2} | {:<8.2}% | {}",
            idx + 1,
            item.leverage,
            item.avg_rank,
            item.capital_dd_ratio,
            item.slope_ratio,
            item.net_profit,
            item.max_dd_pct,
            item.liquidations
        );
    }

    Ok(final_results)
}
