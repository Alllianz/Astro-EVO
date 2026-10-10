use super::dashboard::generate_intelligence_comparison_dashboard;
use super::models::{IntelligenceLayerInfo, IntelligenceLevelResult};
use crate::cnn1d::Cnn1DModel;
use crate::db;
use crate::evo_optimizer::EvoOptimizer;
use crate::graphics;
use crate::metrics::{compute_multicriteria_rankings, CandidateRankScore};
use crate::online_learning::{self, OnlineLearningConfig};
use rayon::prelude::*;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockEvolutionConfig {
    pub tf: String,
    pub start_intelligence: usize,
    pub iterations: usize,
    pub total_trials_per_level: usize,
    pub num_threads: usize,
    pub max_epochs: usize,
    pub capital_percent: f64,
    pub leverage: f64,
}

impl BlockEvolutionConfig {
    pub fn get_intelligence_levels(&self) -> Vec<usize> {
        let start = self.start_intelligence.max(1);
        let count = self.iterations.max(1);
        (start..(start + count)).collect()
    }
}

pub fn run_interactive_block_evolution(
    conn_candles: &Connection,
    conn_models: &Connection,
) -> Result<(), Box<dyn Error>> {
    println!("\n=========================================================================");
    println!("   OPTIMIZACIÓN ALGORÍTMICA POR BLOQUES CON MUTACIÓN DE INTELIGENCIA     ");
    println!("=========================================================================");

    let (default_cp, default_lev, _, _, _, _) =
        db::get_config(conn_models).unwrap_or((10.0, 10.0, String::new(), String::new(), "Astro-EVO-Server".to_string(), 60));

    print!("Temporalidad a evaluar (defecto 1D): ");
    io::stdout().flush()?;
    let mut tf_in = String::new();
    io::stdin().read_line(&mut tf_in)?;
    let tf = match tf_in.trim() {
        "" => "1D".to_string(),
        s => s.to_string().to_uppercase(),
    };

    print!("Punto inicial de inteligencia L (defecto 5): ");
    io::stdout().flush()?;
    let mut start_in = String::new();
    io::stdin().read_line(&mut start_in)?;
    let start_intelligence: usize = start_in.trim().parse().unwrap_or(5).max(1);

    print!("Cantidad de iteraciones / niveles a probar (Incremento +1) (defecto 3): ");
    io::stdout().flush()?;
    let mut iter_in = String::new();
    io::stdin().read_line(&mut iter_in)?;
    let iterations: usize = iter_in.trim().parse().unwrap_or(3).max(1);

    print!("Cantidad de ensayos algorítmicos por nivel de inteligencia (defecto 30): ");
    io::stdout().flush()?;
    let mut trials_in = String::new();
    io::stdin().read_line(&mut trials_in)?;
    let total_trials: usize = trials_in.trim().parse().unwrap_or(30).max(1);

    print!("Cantidad de redes en simultáneo a correr (defecto 4): ");
    io::stdout().flush()?;
    let mut threads_in = String::new();
    io::stdin().read_line(&mut threads_in)?;
    let num_threads: usize = threads_in.trim().parse().unwrap_or(4).max(1);

    print!("Máxima época de Pre-Entrenamiento IS a evaluar (defecto 10): ");
    io::stdout().flush()?;
    let mut epochs_in = String::new();
    io::stdin().read_line(&mut epochs_in)?;
    let max_epochs: usize = match epochs_in.trim() {
        "" => 10,
        s => s.parse().unwrap_or_else(|_| {
            if s.contains(':') {
                s.split(':').filter_map(|p| p.trim().parse::<usize>().ok()).max().unwrap_or(10)
            } else {
                10
            }
        }),
    };

    let evo_config = BlockEvolutionConfig {
        tf,
        start_intelligence,
        iterations,
        total_trials_per_level: total_trials,
        num_threads,
        max_epochs,
        capital_percent: default_cp,
        leverage: default_lev,
    };

    run_block_evolution_sweep(conn_candles, conn_models, &evo_config)
}

pub fn run_block_evolution_sweep(
    conn_candles: &Connection,
    conn_models: &Connection,
    config: &BlockEvolutionConfig,
) -> Result<(), Box<dyn Error>> {
    let tf = &config.tf;
    let dummy_leverage = config.leverage;
    let cap_percent = config.capital_percent;
    let levels = config.get_intelligence_levels();

    println!("\nCargando dataset en RAM para {}...", tf);
    let dataset = Arc::new(online_learning::prepare_dataset_with_horizon(conn_candles, tf, 10)?);

    let epochs_eval: Vec<usize> = if config.max_epochs == 0 {
        vec![0]
    } else {
        (1..=config.max_epochs).collect()
    };

    let pool = rayon::ThreadPoolBuilder::new().num_threads(config.num_threads).build()?;
    let mut all_winning_level_results: Vec<IntelligenceLevelResult> = Vec::new();

    println!(
        "\n🚀 [MUTACIÓN EVOLUTIVA] Iniciando optimización por bloques autónoma sobre {} niveles de inteligencia ({:?})...",
        levels.len(),
        levels
    );

    for &level in &levels {
        println!("\n=========================================================================================");
        println!(
            "  🧬 OPTIMIZANDO NIVEL DE INTELIGENCIA L={} (Arquitectura: {} ➔ {} ➔ {})",
            level, level, level * 2, level * 2
        );
        println!("=========================================================================================");

        let mut optimizer = EvoOptimizer::new(42 + (level as u64 * 100), 6);
        let mut trials_remaining = config.total_trials_per_level;
        let mut batch_idx = 1;
        let mut best_level_fitness = -1.0f64;
        let mut best_trial_winner: Option<(OnlineLearningConfig, usize, Cnn1DModel)> = None;

        while trials_remaining > 0 {
            let batch_size = trials_remaining.min(config.num_threads);
            println!("  🤖 Lote Evolutivo #{} (Nivel L={}) - Probando {} mutaciones...", batch_idx, level, batch_size);

            let mut batch_configs = Vec::new();
            for _ in 0..batch_size {
                let (w, k, h, wd, l1) = optimizer.sample_candidate();

                batch_configs.push(OnlineLearningConfig {
                    tf: tf.clone(),
                    capital_percent: cap_percent,
                    leverage: dummy_leverage,
                    window_size: w,
                    kernel_size: k,
                    horizon: h,
                    learning_rate: 0.001,
                    weight_decay: wd,
                    l1_lambda: l1,
                    seed_val: 42,
                    pre_train_epochs: 0,
                    intelligence: level,
                });
            }

            let (tx, rx) = std::sync::mpsc::channel();
            let pool_tx = tx.clone();
            let ds_for_pool = dataset.clone();
            let epochs_for_pool = Arc::new(epochs_eval.clone());

            pool.spawn(move || {
                batch_configs.into_par_iter().for_each(|cfg| {
                    let ds_with_h = ds_for_pool.with_horizon(cfg.horizon);
                    match online_learning::run_online_learning_incremental_epochs(&ds_with_h, &cfg, &epochs_for_pool) {
                        Ok(results) => {
                            for res in results {
                                let _ = pool_tx.send((cfg.clone(), res));
                            }
                        }
                        Err(e) => eprintln!("Error en ensayo algorítmico (L={}): {}", cfg.intelligence, e),
                    }
                });
            });

            let mut batch_results = Vec::new();
            let total_in_batch = batch_size * epochs_eval.len();

            while let Ok((cfg, (epoch, m_nom, _m_pct, trained_model, _eq_nom, _eq_pct))) = rx.recv() {
                batch_results.push((cfg, epoch, m_nom, trained_model));

                if batch_results.len() >= total_in_batch {
                    break;
                }
            }

            // Ranking Multicriterio Integrado para este lote
            let rank_map = compute_multicriteria_rankings(
                &batch_results,
                |item| format!("{}_L{}_W{}_K{}_H{}_WD{:.6}_L1{:.6}_E{}", item.0.tf, item.0.intelligence, item.0.window_size, item.0.kernel_size, item.0.horizon, item.0.weight_decay, item.0.l1_lambda, item.1),
                |item| item.2.slope_ratio,
                |item| item.2.capital_dd_ratio,
                |item| item.2.curve_r2,
                |item| item.2.smoothness_score,
            );

            let mut ranked_batch = Vec::new();
            for (cfg, epoch, m_nom, trained_model) in batch_results {
                let key = format!("{}_L{}_W{}_K{}_H{}_WD{:.6}_L1{:.6}_E{}", cfg.tf, cfg.intelligence, cfg.window_size, cfg.kernel_size, cfg.horizon, cfg.weight_decay, cfg.l1_lambda, epoch);
                let rank_score = rank_map.get(&key).cloned().unwrap_or(CandidateRankScore {
                    candidate_id: key,
                    rank_slope: 999,
                    rank_cap_dd: 999,
                    rank_r2: 999,
                    rank_smoothness: 999,
                    weighted_avg_rank: 999.0,
                });

                optimizer.record_trial(cfg.window_size, cfg.kernel_size, cfg.horizon, cfg.weight_decay, cfg.l1_lambda, epoch, rank_score.weighted_avg_rank);

                ranked_batch.push((rank_score, cfg, epoch, m_nom, trained_model));
            }

            ranked_batch.sort_by(|a, b| a.0.weighted_avg_rank.partial_cmp(&b.0.weighted_avg_rank).unwrap_or(std::cmp::Ordering::Equal));

            if let Some(best) = ranked_batch.first() {
                if best_level_fitness < 0.0 || best.0.weighted_avg_rank < best_level_fitness {
                    best_level_fitness = best.0.weighted_avg_rank;
                    println!(
                        "  🔥 [MEJOR DE L={}] Rank: {:.2} | Net Profit: ${:.2} | Max DD: {:.2}% | R²: {:.4} | W={}, K={}, H={}, WD={:.5}, L1={:.5} (Época {})",
                        level,
                        best.0.weighted_avg_rank,
                        best.3.net_profit,
                        best.3.max_dd,
                        best.3.curve_r2,
                        best.1.window_size,
                        best.1.kernel_size,
                        best.1.horizon,
                        best.1.weight_decay,
                        best.1.l1_lambda,
                        best.2
                    );
                    let mut winner_cfg = best.1.clone();
                    winner_cfg.pre_train_epochs = best.2;
                    best_trial_winner = Some((winner_cfg, best.2, best.4.clone()));
                }
            }

            trials_remaining -= batch_size;
            batch_idx += 1;
        }

        // Si encontramos un ganador para este nivel de inteligencia, procesar y guardar su simulación final
        if let Some((mut winner_cfg, epoch_winner, winner_model)) = best_trial_winner {
            winner_cfg.pre_train_epochs = epoch_winner;
            let ds_winner = dataset.with_horizon(winner_cfg.horizon);

            let (m_nom, m_pct, final_trained_model, eq_nom, eq_pct) = online_learning::run_online_learning_simulation_with_model(
                &ds_winner,
                &winner_cfg,
                winner_model.clone(),
            )?;

            let layer_info = IntelligenceLayerInfo::from_model(&final_trained_model, level);

            all_winning_level_results.push(IntelligenceLevelResult {
                level,
                layer_info,
                config: winner_cfg,
                metrics_nom: m_nom,
                metrics_pct: m_pct,
                model: final_trained_model,
                eq_curve_nom: eq_nom,
                eq_curve_pct: eq_pct,
                rank_score: CandidateRankScore {

                    candidate_id: format!("Level_{}", level),
                    rank_slope: 999,
                    rank_cap_dd: 999,
                    rank_r2: 999,
                    rank_smoothness: 999,
                    weighted_avg_rank: best_level_fitness,
                },
                epoch: epoch_winner,
            });
        }
    }

        if all_winning_level_results.is_empty() {
        eprintln!("⚠️ No se obtuvieron resultados ganadores.");
        return Ok(());
    }

    // Persistir y fusionar los resultados de esta corrida con todo el historial previo en SQLite
    let combined_results = super::history::merge_and_rank_with_history(conn_models, &all_winning_level_results, tf)?;

    println!("\n=========================================================================================================");
    println!("     🏆 RANKING COMPARATIVO MULTI-SESIÓN DE ARQUITECTURAS OPTIMIZADAS POR BLOQUES (TOTAL: {} NIVELES)   ", combined_results.len());
    println!("=========================================================================================================");
    println!("{:<4} | {:<8} | {:<16} | {:<10} | {:<12} | {:<10} | {:<8} | {:<8} | {:<8} | {:<16}",
        "Pos", "Nivel", "Neuronas", "Parámetros", "Net Profit", "Max DD", "Win Rate", "Sortino", "R² Glob", "Config Óptima"
    );
    println!("---------------------------------------------------------------------------------------------------------");

    for (pos, res) in combined_results.iter().enumerate() {
        let neurons = format!("{}➔{}➔{}", res.layer_info.conv1_out, res.layer_info.conv2_out, res.layer_info.dense_in);
        let cfg_str = format!("W{} K{} H{} E{}", res.config.window_size, res.config.kernel_size, res.config.horizon, res.epoch);
        println!("{:<4} | L={:<6} | {:<16} | {:<10} | ${:<11.2} | {:<9.2}% | {:<7.2}% | {:<8.2} | {:<8.4} | {}",
            pos + 1,
            res.level,
            neurons,
            res.layer_info.total_weights_count,
            res.metrics_nom.net_profit,
            res.metrics_nom.max_dd,
            res.metrics_nom.winrate,
            res.metrics_nom.sortino_ratio,
            res.metrics_nom.curve_r2,
            cfg_str
        );
    }
    println!("=========================================================================================================");

    let is_start_ts = crate::indicators::parse_date_to_timestamp("20-04-2020").unwrap_or(0);
    let oos_start_ts = crate::indicators::parse_date_to_timestamp("01-01-2023").unwrap_or(0);
    let mut is_start_idx = dataset.klines.binary_search_by_key(&is_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    let min_idx = 30 + 200;
    if is_start_idx < min_idx { is_start_idx = min_idx; }
    let mut oos_start_idx = dataset.klines.binary_search_by_key(&oos_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    if oos_start_idx <= is_start_idx || oos_start_idx >= dataset.klines.len() {
        oos_start_idx = is_start_idx + 100;
    }

    let min_combined_level = combined_results.iter().map(|r| r.level).min().unwrap_or(config.start_intelligence);
    let max_combined_level = combined_results.iter().map(|r| r.level).max().unwrap_or(config.start_intelligence);

    let dummy_sweep_cfg = super::config::IntelligenceSweepConfig {
        tf: tf.clone(),
        start_intelligence: min_combined_level,
        iterations: (max_combined_level - min_combined_level) + 1,
        capital_percent: config.capital_percent,
        leverage: config.leverage,
        window_size: combined_results.first().map(|r| r.config.window_size).unwrap_or(30),
        kernel_size: combined_results.first().map(|r| r.config.kernel_size).unwrap_or(15),
        horizon: combined_results.first().map(|r| r.config.horizon).unwrap_or(10),
        learning_rate: 0.001,
        weight_decay: 0.001,
        l1_lambda: 0.0002,
        seed_val: 42,
        pre_train_epochs: 5,
        num_threads: config.num_threads,
    };

    // Generar Dashboard HTML Comparativo Multi-Inteligencia con todos los niveles históricos combinados
    let dashboard_path = generate_intelligence_comparison_dashboard(
        "",
        &dataset.klines,
        &dummy_sweep_cfg,
        &combined_results,
        is_start_idx,
        oos_start_idx,
    )?;

    println!("\n🎨 Generando Dashboards Duales individuales para cada nivel de inteligencia...");
    for res in &combined_results {
        let individual_html_name = format!(
            "dashboard_dual_L{}_{}_W{}_K{}_H{}_E{}_WD{:.4}_L1{:.6}__{:.0}X_{:.0}%.html",
            res.level,
            res.config.tf,
            res.config.window_size,
            res.config.kernel_size,
            res.config.horizon,
            res.epoch,
            res.config.weight_decay,
            res.config.l1_lambda,
            res.config.leverage,
            res.config.capital_percent
        );

        let individual_img_name = format!(
            "equity_chart_L{}_{}_W{}_K{}_H{}_E{}__{:.0}X.png",
            res.level,
            res.config.tf,
            res.config.window_size,
            res.config.kernel_size,
            res.config.horizon,
            res.epoch,
            res.config.leverage
        );

        let _ = crate::dashboard::generate_dual_dashboard(
            "",
            &dataset.klines,
            &res.config,
            res.epoch,
            &res.metrics_nom,
            &res.metrics_pct,
            &res.eq_curve_nom,
            &res.eq_curve_pct,
            is_start_idx,
            oos_start_idx,
            Some(&individual_html_name),
        );

        let _ = graphics::draw_equity_chart(
            &dataset.klines,
            &res.eq_curve_nom,
            &res.config.tf,
            res.epoch,
            res.config.window_size,
            res.config.leverage,
            res.config.capital_percent,
            is_start_idx,
            oos_start_idx,
            res.metrics_nom.capital_final,
            res.metrics_nom.max_dd,
            res.metrics_nom.profit_factor,
            res.metrics_nom.winrate,
            &individual_img_name,
        );

        println!("   ↳ [Nivel L={}] Dashboard individual generado: {}", res.level, individual_html_name);
    }

    // Guardar el ganador absoluto en SQLite y generar gráfico PNG
    if let Some(best) = combined_results.first() {
        let assigned_id = db::save_cnn1d_model_full(
            conn_models,
            &best.config.tf,
            best.config.window_size,
            best.config.kernel_size,
            best.epoch,
            &best.model,
            0.40,
            0.40,
            true,
            best.config.leverage,
            best.config.capital_percent,
            best.config.weight_decay as f64,
            best.config.l1_lambda as f64,
            0.0,
            best.config.horizon,
        )?;

        println!("\n💾 [GANADOR GLOBAL HISTÓRICO L={}] Guardado automáticamente en models.db con ID #{}", best.level, assigned_id);

        let img_name = format!(
            "opt_bloques_winner_L{}_{}_W{}_K{}_H{}__{:.0}X.png",
            best.level, best.config.tf, best.config.window_size, best.config.kernel_size, best.config.horizon, best.config.leverage
        );

        let _ = graphics::draw_equity_chart(
            &dataset.klines,
            &best.eq_curve_nom,
            &best.config.tf,
            best.epoch,
            best.config.window_size,
            best.config.leverage,
            best.config.capital_percent,
            is_start_idx,
            oos_start_idx,
            best.metrics_nom.capital_final,
            best.metrics_nom.max_dd,
            best.metrics_nom.profit_factor,
            best.metrics_nom.winrate,
            &img_name,
        );
    }

    println!("\n✨ Proceso completado exitosamente. Todos los Dashboards individuales y el Dashboard Comparativo han sido generados.");
    println!("   📂 Dashboard Comparativo: {}", dashboard_path);

    Ok(())
}
