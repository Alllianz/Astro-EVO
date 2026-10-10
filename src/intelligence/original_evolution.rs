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
pub struct OriginalBlockEvolutionConfig {
    pub tf: String,
    pub total_trials: usize,
    pub num_threads: usize,
    pub max_epochs: usize,
    pub capital_percent: f64,
    pub leverage: f64,
}

pub fn run_interactive_original_block_evolution(
    conn_candles: &Connection,
    conn_models: &Connection,
) -> Result<(), Box<dyn Error>> {
    println!("\n=========================================================================");
    println!("   OPTIMIZACIÓN ALGORÍTMICA POR BLOQUES (ENTRENAMIENTO ORIGINAL)         ");
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

    print!("Cantidad total de ensayos algorítmicos a evaluar (defecto 30): ");
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

    let evo_config = OriginalBlockEvolutionConfig {
        tf,
        total_trials,
        num_threads,
        max_epochs,
        capital_percent: default_cp,
        leverage: default_lev,
    };

    run_original_block_evolution(conn_candles, conn_models, &evo_config)
}

pub fn run_original_block_evolution(
    conn_candles: &Connection,
    conn_models: &Connection,
    config: &OriginalBlockEvolutionConfig,
) -> Result<(), Box<dyn Error>> {
    let tf = &config.tf;
    let dummy_leverage = config.leverage;
    let cap_percent = config.capital_percent;

    println!("\nCargando dataset en RAM para {}...", tf);
    let dataset = Arc::new(online_learning::prepare_dataset_with_horizon(conn_candles, tf, 10)?);

    let epochs_eval: Vec<usize> = if config.max_epochs == 0 {
        vec![0]
    } else {
        (1..=config.max_epochs).collect()
    };

    let pool = rayon::ThreadPoolBuilder::new().num_threads(config.num_threads).build()?;

    println!(
        "\n🚀 [ENTRENAMIENTO ORIGINAL] Iniciando optimización por bloques autónoma sobre {} ensayos...",
        config.total_trials
    );

    let mut optimizer = EvoOptimizer::new(42, 6);
    let mut trials_remaining = config.total_trials;
    let mut batch_idx = 1;
    let mut best_fitness = -1.0f64;
    let mut best_winner: Option<(OnlineLearningConfig, usize, Cnn1DModel)> = None;

    while trials_remaining > 0 {
        let batch_size = trials_remaining.min(config.num_threads);
        println!("  🤖 Lote Evolutivo #{} - Probando {} mutaciones...", batch_idx, batch_size);

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
                intelligence: 0, // Arquitectura fija estándar del timeframe
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
                    Err(e) => eprintln!("Error en ensayo algorítmico original: {}", e),
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

        let rank_map = compute_multicriteria_rankings(
            &batch_results,
            |item| format!("{}_W{}_K{}_H{}_WD{:.6}_L1{:.6}_E{}", item.0.tf, item.0.window_size, item.0.kernel_size, item.0.horizon, item.0.weight_decay, item.0.l1_lambda, item.1),
            |item| item.2.slope_ratio,
            |item| item.2.capital_dd_ratio,
            |item| item.2.curve_r2,
            |item| item.2.smoothness_score,
        );

        let mut ranked_batch = Vec::new();
        for (cfg, epoch, m_nom, trained_model) in batch_results {
            let key = format!("{}_W{}_K{}_H{}_WD{:.6}_L1{:.6}_E{}", cfg.tf, cfg.window_size, cfg.kernel_size, cfg.horizon, cfg.weight_decay, cfg.l1_lambda, epoch);
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
            if best_fitness < 0.0 || best.0.weighted_avg_rank < best_fitness {
                best_fitness = best.0.weighted_avg_rank;
                println!(
                    "  🔥 [MEJOR MODELO] Rank: {:.2} | Net Profit: ${:.2} | Max DD: {:.2}% | R²: {:.4} | W={}, K={}, H={}, WD={:.5}, L1={:.5} (Época {})",
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
                best_winner = Some((winner_cfg, best.2, best.4.clone()));
            }
        }

        trials_remaining -= batch_size;
        batch_idx += 1;
    }

    let (mut winner_cfg, epoch_winner, winner_model) = match best_winner {
        Some(w) => w,
        None => {
            eprintln!("⚠️ No se obtuvieron resultados ganadores en la optimización por bloques.");
            return Ok(());
        }
    };

    winner_cfg.pre_train_epochs = epoch_winner;
    let ds_winner = dataset.with_horizon(winner_cfg.horizon);

    let (m_nom, m_pct, final_trained_model, eq_nom, eq_pct) = online_learning::run_online_learning_simulation_with_model(
        &ds_winner,
        &winner_cfg,
        winner_model,
    )?;

    let is_start_ts = crate::indicators::parse_date_to_timestamp("20-04-2020").unwrap_or(0);
    let oos_start_ts = crate::indicators::parse_date_to_timestamp("01-01-2023").unwrap_or(0);
    let mut is_start_idx = dataset.klines.binary_search_by_key(&is_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    let min_idx = winner_cfg.window_size + 200;
    if is_start_idx < min_idx { is_start_idx = min_idx; }
    let mut oos_start_idx = dataset.klines.binary_search_by_key(&oos_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    if oos_start_idx <= is_start_idx || oos_start_idx >= dataset.klines.len() {
        oos_start_idx = is_start_idx + 100;
    }

    // 1. Guardar modelo ganador en base de datos SQLite (cnn1d_models)
    let assigned_id = db::save_cnn1d_model_full(
        conn_models,
        &winner_cfg.tf,
        winner_cfg.window_size,
        winner_cfg.kernel_size,
        epoch_winner,
        &final_trained_model,
        0.40,
        0.40,
        true,
        winner_cfg.leverage,
        winner_cfg.capital_percent,
        winner_cfg.weight_decay as f64,
        winner_cfg.l1_lambda as f64,
        0.0,
        winner_cfg.horizon,
    )?;

    println!("\n💾 [MODELO GANADOR GUARDADO] ID #{} en models.db", assigned_id);

    // 2. Generar Dashboard Dual en <TF>/Dashboard/Individual/
    let html_name = format!(
        "dashboard_dual_{}_W{}_K{}_H{}_E{}_WD{:.4}_L1{:.6}__{:.0}X_{:.0}%.html",
        winner_cfg.tf,
        winner_cfg.window_size,
        winner_cfg.kernel_size,
        winner_cfg.horizon,
        epoch_winner,
        winner_cfg.weight_decay,
        winner_cfg.l1_lambda,
        winner_cfg.leverage,
        winner_cfg.capital_percent
    );

    let html_path = crate::dashboard::generate_dual_dashboard(
        "",
        &dataset.klines,
        &winner_cfg,
        epoch_winner,
        &m_nom,
        &m_pct,
        &eq_nom,
        &eq_pct,
        is_start_idx,
        oos_start_idx,
        Some(&html_name),
    )?;

    // 3. Generar Gráfico de Curva de Equidad PNG en <TF>/Epochs/
    let img_name = format!(
        "opt_bloques_{}_W{}_K{}_H{}_E{}__{:.0}X.png",
        winner_cfg.tf,
        winner_cfg.window_size,
        winner_cfg.kernel_size,
        winner_cfg.horizon,
        epoch_winner,
        winner_cfg.leverage
    );

    let _ = graphics::draw_equity_chart(
        &dataset.klines,
        &eq_nom,
        &winner_cfg.tf,
        epoch_winner,
        winner_cfg.window_size,
        winner_cfg.leverage,
        winner_cfg.capital_percent,
        is_start_idx,
        oos_start_idx,
        m_nom.capital_final,
        m_nom.max_dd,
        m_nom.profit_factor,
        m_nom.winrate,
        &img_name,
    );

    println!("\n=========================================================================================================");
    println!("     🏆 RESULTADOS FINALES DEL ENTRENAMIENTO ORIGINAL (ID #{})                                           ", assigned_id);
    println!("=========================================================================================================");
    println!("Temporalidad:         {}", winner_cfg.tf);
    println!("Configuración Óptima: W={} | K={} | H={} | Época={} | WD={:.5} | L1={:.5}",
        winner_cfg.window_size, winner_cfg.kernel_size, winner_cfg.horizon, epoch_winner, winner_cfg.weight_decay, winner_cfg.l1_lambda
    );
    println!("Capital Final:        ${:.2}", m_nom.capital_final);
    println!("Beneficio Neto:       ${:.2}", m_nom.net_profit);
    println!("Drawdown Máximo:      {:.2}%", m_nom.max_dd);
    println!("Tasa de Acierto:      {:.2}%", m_nom.winrate);
    println!("Profit Factor:        {:.2}", m_nom.profit_factor);
    println!("Ratio Sortino:        {:.2}", m_nom.sortino_ratio);
    println!("R² Curva Global:      {:.4}", m_nom.curve_r2);
    println!("---------------------------------------------------------------------------------------------------------");
    println!("📂 Dashboard Individual: {}", html_path);
    println!("🖼️ Gráfico de Equidad:   {}/Epochs/{}", winner_cfg.tf, img_name);
    println!("=========================================================================================================");

    Ok(())
}
