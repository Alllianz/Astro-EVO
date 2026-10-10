use super::config::IntelligenceSweepConfig;
use super::models::{IntelligenceLayerInfo, IntelligenceLevelResult};
use super::ranking::rank_intelligence_results;
use crate::cnn1d::Cnn1DModel;
use crate::metrics::CandidateRankScore;
use crate::online_learning::{self, AstroDataset, OnlineLearningConfig};
use rayon::prelude::*;
use rusqlite::Connection;
use std::error::Error;
use std::sync::Arc;

pub fn run_intelligence_sweep(
    conn_candles: &Connection,
    config: &IntelligenceSweepConfig,
) -> Result<(AstroDataset, Vec<IntelligenceLevelResult>), Box<dyn Error>> {
    let dataset = online_learning::prepare_dataset_with_horizon(
        conn_candles,
        &config.tf,
        config.horizon,
    )?;

    let levels = config.get_intelligence_levels();
    let total_levels = levels.len();

    println!(
        "\n🚀 [SWEEP DE INTELIGENCIA] Evaluando {} niveles de inteligencia (L={} a L={}) en paralelo con {} hilos...",
        total_levels,
        levels.first().unwrap_or(&1),
        levels.last().unwrap_or(&1),
        config.num_threads
    );

    let (tx, rx) = std::sync::mpsc::channel();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.num_threads)
        .build()?;

    let dataset_arc = Arc::new(dataset.clone());
    let config_arc = Arc::new(config.clone());
    let epochs_list = vec![config.pre_train_epochs];
    let epochs_arc = Arc::new(epochs_list);

    let pool_tx = tx.clone();
    let pool_dataset = dataset_arc.clone();
    let pool_config = config_arc.clone();
    let pool_epochs = epochs_arc.clone();

    pool.spawn(move || {
        levels.into_par_iter().for_each(|level| {
            let mut ol_cfg = OnlineLearningConfig {
                tf: pool_config.tf.clone(),
                capital_percent: pool_config.capital_percent,
                leverage: pool_config.leverage,
                window_size: pool_config.window_size,
                kernel_size: pool_config.kernel_size,
                horizon: pool_config.horizon,
                learning_rate: pool_config.learning_rate,
                weight_decay: pool_config.weight_decay,
                l1_lambda: pool_config.l1_lambda,
                seed_val: pool_config.seed_val,
                pre_train_epochs: pool_config.pre_train_epochs,
                intelligence: level,
            };

            let base_model = Cnn1DModel::new_with_intelligence(
                pool_config.tf.clone(),
                pool_config.window_size,
                pool_config.kernel_size,
                pool_config.seed_val,
                level,
            );
            let layer_info = IntelligenceLayerInfo::from_model(&base_model, level);

            println!(
                "  ⚙️ [Nivel L={}] Inicializando red: Conv1: {} filtros, Conv2: {} filtros, Densa: {} ➔ 3 (Total params: {})",
                level, layer_info.conv1_out, layer_info.conv2_out, layer_info.dense_in, layer_info.total_weights_count
            );

            match online_learning::run_online_learning_incremental_epochs(
                &pool_dataset,
                &ol_cfg,
                &pool_epochs,
            ) {
                Ok(epoch_results) => {
                    if let Some((target_epoch, m_nom, m_pct, trained_model, eq_curve_nom, eq_curve_pct)) = epoch_results.into_iter().last() {
                        ol_cfg.pre_train_epochs = target_epoch;
                        let item = IntelligenceLevelResult {
                            level,
                            layer_info,
                            config: ol_cfg,
                            metrics_nom: m_nom,
                            metrics_pct: m_pct,
                            model: trained_model,
                            eq_curve_nom,
                            eq_curve_pct,
                            rank_score: CandidateRankScore {
                                candidate_id: format!("Level_{}", level),
                                rank_slope: 999,
                                rank_cap_dd: 999,
                                rank_r2: 999,
                                rank_smoothness: 999,
                                weighted_avg_rank: 999.0,
                            },
                            epoch: target_epoch,
                        };
                        let _ = pool_tx.send(item);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error evaluando nivel de inteligencia L={}: {}", level, e);
                }
            }
        });
    });

    let mut results = Vec::new();
    while let Ok(item) = rx.recv() {
        println!(
            "  ✅ [Nivel L={}] Finalizado ➔ Net Profit: ${:.2} | Retorno: {:.2}% | Max DD: {:.2}% | WinRate: {:.2}% | R²: {:.4} | Sortino: {:.2}",
            item.level,
            item.metrics_nom.net_profit,
            ((item.metrics_nom.capital_final as f64 - 10000.0) / 10000.0) * 100.0,
            item.metrics_nom.max_dd,
            item.metrics_nom.winrate,
            item.metrics_nom.curve_r2,
            item.metrics_nom.sortino_ratio
        );
        results.push(item);

        if results.len() >= total_levels {
            break;
        }
    }

    // Calcular rankings multicriterio y ordenar
    rank_intelligence_results(&mut results);

    Ok((dataset, results))
}
