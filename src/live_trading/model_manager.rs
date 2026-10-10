use crate::cnn1d::{Cnn1DModel, Cnn1DOptimizerState};
use crate::db::{get_all_klines, get_latest_cnn1d_model_for_tf, Connection};
use crate::features::extract_cnn1d_features_for_tf;
use crate::indicators::*;
use crate::online_learning::generate_astro_labels;
use chrono::TimeZone;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub timestamp: i64,
    pub close: f64,
    pub p_long: f32,
    pub p_short: f32,
    pub p_flat: f32,
    pub dynamic_thr: f32,
    pub signal: crate::live_trading::signal_generator::SignalType,
    pub is_change: bool,
}

pub struct LiveModelInstance {
    pub tf: String,
    pub live_model: Cnn1DModel,
    pub optimizer_state: Cnn1DOptimizerState,
    pub thr_long: f32,
    pub thr_short: f32,
    pub window_size: usize,
    pub horizon: usize,
    pub weight_decay: f32,
    pub l1_lambda: f32,
    pub learning_rate: f32,
    pub replay_buffer: Vec<(ndarray::Array2<f32>, usize)>,
    pub peak_wallet_balance: f64,
    pub last_trained_idx: usize,
}

pub struct LiveModelManager {
    pub instances: HashMap<String, LiveModelInstance>,
}

impl LiveModelManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    /// Carga el modelo base de models.db para `tf` (sólo lectura), clona el modelo en memoria
    /// y entrena el clon sobre las velas post-OOS de `tf` con Replay Buffer y LR adaptativo.
    /// ¡ESTRICTAMENTE NUNCA SOBREESCRIBE models.db!
    pub fn get_or_init_live_model(
        &mut self,
        conn_candles: &Connection,
        conn_models: &Connection,
        tf: &str,
        current_wallet_balance: f64,
    ) -> Result<Option<&mut LiveModelInstance>, Box<dyn Error>> {
        if !self.instances.contains_key(tf) {
            // 1. Cargar modelo base original de SQLite (sólo lectura)
            let (base_model, thr_long, thr_short, window_size, horizon, weight_decay, l1_lambda) = 
                match get_latest_cnn1d_model_for_tf(conn_models, tf)? {
                    Some(m) => m,
                    None => return Ok(None),
                };

            // 2. Clonar el modelo en memoria (el original queda intacto en SQLite)
            let mut live_model = base_model.clone();
            let mut optimizer_state = Cnn1DOptimizerState::new_for_model(&live_model);
            let mut replay_buffer: Vec<(ndarray::Array2<f32>, usize)> = Vec::new();
            let base_lr = 0.001f32;
            let peak_wallet_balance = current_wallet_balance.max(10000.0);

            let klines = get_all_klines(conn_candles, tf)?;
            let mut last_trained_idx = 0;

            if klines.len() >= window_size + 200 {
                // 3. Puesta al día (Catch-up) en memoria desde el inicio hasta la última vela con etiqueta vencida
                println!(
                    "🧠 [Live Model Catch-up] Poniendo al día clon en memoria para {} sobre {} velas...",
                    tf, klines.len()
                );
                
                let volume_sma = precalculate_volume_sma(&klines);
                let (volume_sma_5, volume_sma_20) = precalculate_volume_smas(&klines);
                let macd_hist = precalculate_macd_hist(&klines);
                let ema_20 = precalculate_ema_20(&klines);
                let ema_200_hl2 = precalculate_ema_200_hl2(&klines);
                let atr_5 = precalculate_atr(&klines, 5);
                let atr_50 = precalculate_atr(&klines, 50);

                let labels = generate_astro_labels(&klines, horizon);

                let oos_start_ts = parse_date_to_timestamp("01-01-2023").unwrap_or(0);
                let mut start_catchup_idx = klines.binary_search_by_key(&oos_start_ts, |k| k.timestamp)
                    .unwrap_or_else(|idx| idx);
                let min_idx = window_size + 200;
                if start_catchup_idx < min_idx {
                    start_catchup_idx = min_idx;
                }

                let mut trained_steps = 0;
                let mut decision_history: Vec<DecisionRecord> = Vec::new();
                let mut current_sim_pos = 0.0f32;
                let mut flips_count = 0;

                for i in start_catchup_idx..klines.len() {
                    // 1. Entrenar vela pasada si su horizonte ya venció
                    if i >= start_catchup_idx + horizon {
                        let train_idx = i - horizon;
                        if let Some(feat_past) = extract_cnn1d_features_for_tf(
                            &klines, train_idx, live_model.window_size, tf,
                            &volume_sma, &volume_sma_5, &volume_sma_20,
                            &macd_hist, &ema_20, &ema_200_hl2,
                            &atr_5, &atr_50, 0.0
                        ) {
                            let target_label = labels[train_idx];
                            live_model.train_step_smoothed(&feat_past, target_label, base_lr, weight_decay, l1_lambda, &mut optimizer_state, 0.15);
                            
                            if replay_buffer.len() >= 256 {
                                replay_buffer.remove(0);
                            }
                            replay_buffer.push((feat_past, target_label));

                            if replay_buffer.len() >= 10 {
                                let b_len = replay_buffer.len();
                                let sample_indices = [(train_idx * 7) % b_len, (train_idx * 13) % b_len, (train_idx * 29) % b_len];
                                for &r_idx in &sample_indices {
                                    let (r_feat, r_target) = &replay_buffer[r_idx];
                                    live_model.train_step_smoothed(r_feat, *r_target, base_lr, weight_decay, l1_lambda, &mut optimizer_state, 0.15);
                                }
                            }

                            last_trained_idx = train_idx;
                            trained_steps += 1;
                        }
                    }

                    // 2. Evaluar decisión del modelo en la vela i
                    if let Some(feat_curr) = extract_cnn1d_features_for_tf(
                        &klines, i, live_model.window_size, tf,
                        &volume_sma, &volume_sma_5, &volume_sma_20,
                        &macd_hist, &ema_20, &ema_200_hl2,
                        &atr_5, &atr_50, current_sim_pos
                    ) {
                        let probs = live_model.predict(&feat_curr);
                        let p_long = probs[0];
                        let p_short = probs[1];
                        let p_flat = probs[2];

                        let price = klines[i].close;
                        let atr_ratio = if price > 0.0 { atr_50[i] / price } else { 0.01 };
                        let dynamic_thr = (0.40 + 0.15 * (atr_ratio * 100.0).clamp(0.0, 1.0)).clamp(0.40, 0.55) as f32;

                        let sig = if p_long > p_short && p_long > p_flat && p_long > dynamic_thr {
                            crate::live_trading::signal_generator::SignalType::Long
                        } else if p_short > p_long && p_short > p_flat && p_short > dynamic_thr {
                            crate::live_trading::signal_generator::SignalType::Short
                        } else {
                            crate::live_trading::signal_generator::SignalType::Flat
                        };

                        let new_pos = match sig {
                            crate::live_trading::signal_generator::SignalType::Long => 1.0,
                            crate::live_trading::signal_generator::SignalType::Short => -1.0,
                            crate::live_trading::signal_generator::SignalType::Flat => 0.0,
                        };

                        let is_change = new_pos != current_sim_pos;
                        if is_change {
                            flips_count += 1;
                        }
                        current_sim_pos = new_pos;

                        decision_history.push(DecisionRecord {
                            timestamp: klines[i].timestamp,
                            close: price,
                            p_long,
                            p_short,
                            p_flat,
                            dynamic_thr,
                            signal: sig,
                            is_change,
                        });
                    }
                }

                println!("✅ [Live Model Catch-up] Clon en memoria para {} sincronizado con {} pasos de aprendizaje online y Replay Buffer activado.", tf, trained_steps);

                // Mostrar el historial y evolución de decisiones hasta la última vela cerrada
                if !decision_history.is_empty() {
                    println!("\n📜 [{} | EVOLUCIÓN HISTÓRICA DE DECISIONES DE LA RED HASTA LA ÚLTIMA VELA CERRADA]", tf);
                    println!("───────────────────────────────────────────────────────────────────────────────────────────────────");
                    println!(" Fecha UTC            | Precio BTC   | Prob Long | Prob Short | Prob Flat | DynThr | Señal / Acción");
                    println!("───────────────────────────────────────────────────────────────────────────────────────────────────");

                    let total_decisions = decision_history.len();
                    let recent_window = 10;
                    let mut printed_indices = std::collections::HashSet::new();

                    for (d_idx, rec) in decision_history.iter().enumerate() {
                        if rec.is_change {
                            printed_indices.insert(d_idx);
                        }
                    }

                    if total_decisions > recent_window {
                        for idx in (total_decisions - recent_window)..total_decisions {
                            printed_indices.insert(idx);
                        }
                    } else {
                        for idx in 0..total_decisions {
                            printed_indices.insert(idx);
                        }
                    }

                    let mut sorted_indices: Vec<usize> = printed_indices.into_iter().collect();
                    sorted_indices.sort_unstable();

                    let mut prev_idx = 0;
                    for (row_num, &idx) in sorted_indices.iter().enumerate() {
                        if row_num > 0 && idx > prev_idx + 1 {
                            println!(" ... ({} velas intermedias con postura mantenida) ...", idx - prev_idx - 1);
                        }
                        prev_idx = idx;

                        let rec = &decision_history[idx];
                        let date_str = chrono::Utc.timestamp_millis_opt(rec.timestamp)
                            .single()
                            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "N/A".to_string());

                        let (sig_label, change_tag) = match rec.signal {
                            crate::live_trading::signal_generator::SignalType::Long => {
                                if rec.is_change { ("LONG 🚀", "🔄 CAMBIO A LONG") } else { ("LONG 🚀", "Mantiene LONG") }
                            }
                            crate::live_trading::signal_generator::SignalType::Short => {
                                if rec.is_change { ("SHORT 🔻", "🔄 CAMBIO A SHORT") } else { ("SHORT 🔻", "Mantiene SHORT") }
                            }
                            crate::live_trading::signal_generator::SignalType::Flat => {
                                if rec.is_change { ("FLAT ⚪", "🔄 CAMBIO A FLAT") } else { ("FLAT ⚪", "Mantiene FLAT") }
                            }
                        };

                        println!(
                            " {} | ${:<11.3} |   {:<7.3} |   {:<8.3} |   {:<7.3} |  {:<5.3} | {} ({})",
                            date_str, rec.close, rec.p_long, rec.p_short, rec.p_flat, rec.dynamic_thr, sig_label, change_tag
                        );
                    }
                    println!("───────────────────────────────────────────────────────────────────────────────────────────────────");
                    println!(
                        "📊 Resumen: {} velas evaluadas | {} cambios de postura | Señal de la última vela cerrada: {:?}",
                        total_decisions,
                        flips_count,
                        decision_history.last().map(|d| d.signal).unwrap_or(crate::live_trading::signal_generator::SignalType::Flat)
                    );
                }
            }

            self.instances.insert(tf.to_string(), LiveModelInstance {
                tf: tf.to_string(),
                live_model,
                optimizer_state,
                thr_long,
                thr_short,
                window_size,
                horizon,
                weight_decay,
                l1_lambda,
                learning_rate: base_lr,
                replay_buffer,
                peak_wallet_balance,
                last_trained_idx,
            });
        }

        Ok(self.instances.get_mut(tf))
    }

    /// Actualiza el clon en memoria si se detectan nuevas velas cerradas cuyo horizonte ya venció
    pub fn update_live_learning_on_tick(
        &mut self,
        conn_candles: &Connection,
        tf: &str,
        current_wallet_balance: f64,
    ) {
        if let Some(instance) = self.instances.get_mut(tf) {
            if current_wallet_balance > instance.peak_wallet_balance {
                instance.peak_wallet_balance = current_wallet_balance;
            }

            if let Ok(klines) = get_all_klines(conn_candles, tf) {
                if klines.len() > instance.horizon {
                    let max_trainable_idx = klines.len() - 1 - instance.horizon;
                    if max_trainable_idx > instance.last_trained_idx && max_trainable_idx >= instance.window_size + 200 {
                        let volume_sma = precalculate_volume_sma(&klines);
                        let (volume_sma_5, volume_sma_20) = precalculate_volume_smas(&klines);
                        let macd_hist = precalculate_macd_hist(&klines);
                        let ema_20 = precalculate_ema_20(&klines);
                        let ema_200_hl2 = precalculate_ema_200_hl2(&klines);
                        let atr_5 = precalculate_atr(&klines, 5);
                        let atr_50 = precalculate_atr(&klines, 50);
                        let labels = generate_astro_labels(&klines, instance.horizon);

                        // Learning Rate Adaptativo por Drawdown en Vivo
                        let dd_pct_float = if instance.peak_wallet_balance > 0.0 {
                            ((instance.peak_wallet_balance - current_wallet_balance) / instance.peak_wallet_balance).clamp(0.0, 0.5)
                        } else {
                            0.0
                        };
                        let adaptive_perf_factor = 1.0 + (dd_pct_float * 4.0);
                        let current_lr = instance.learning_rate * adaptive_perf_factor as f32;

                        for i in (instance.last_trained_idx + 1)..=max_trainable_idx {
                            if let Some(feat) = extract_cnn1d_features_for_tf(
                                &klines, i, instance.live_model.window_size, tf,
                                &volume_sma, &volume_sma_5, &volume_sma_20,
                                &macd_hist, &ema_20, &ema_200_hl2,
                                &atr_5, &atr_50, 0.0
                            ) {
                                let target_label = labels[i];
                                instance.live_model.train_step_smoothed(
                                    &feat, target_label, current_lr, instance.weight_decay, instance.l1_lambda, &mut instance.optimizer_state, 0.15
                                );

                                if instance.replay_buffer.len() >= 256 {
                                    instance.replay_buffer.remove(0);
                                }
                                instance.replay_buffer.push((feat, target_label));

                                if instance.replay_buffer.len() >= 10 {
                                    let b_len = instance.replay_buffer.len();
                                    let sample_indices = [(i * 7) % b_len, (i * 13) % b_len, (i * 29) % b_len];
                                    for &r_idx in &sample_indices {
                                        let (r_feat, r_target) = &instance.replay_buffer[r_idx];
                                        instance.live_model.train_step_smoothed(
                                            r_feat, *r_target, current_lr, instance.weight_decay, instance.l1_lambda, &mut instance.optimizer_state, 0.15
                                        );
                                    }
                                }

                                instance.last_trained_idx = i;
                                println!("🧠 [Live Learning] Clon en memoria para {} actualizado con vela #{} (LR: {:.6})", tf, i, current_lr);
                            }
                        }
                    }
                }
            }
        }
    }
}
