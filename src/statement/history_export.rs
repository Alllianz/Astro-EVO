use crate::db::{get_all_klines, get_all_intelligence_records_for_tf, init_candles_db, init_models_db};
use crate::intelligence::models::IntelligenceLevelResult;
use crate::online_learning::{prepare_dataset_with_horizon, run_online_learning_simulation_with_model};
use super::exporter::export_statement_csv;
use std::error::Error;
use std::path::PathBuf;

/// Genera el archivo statement CSV para un nivel específico de inteligencia histórica (ej. el ganador L14)
pub fn export_statement_for_historical_level(
    tf: &str,
    target_level: usize,
) -> Result<PathBuf, Box<dyn Error>> {
    let conn_candles = init_candles_db()?;
    let conn_models = init_models_db()?;

    let klines = get_all_klines(&conn_candles, tf)?;
    if klines.is_empty() {
        return Err(format!("No se encontraron velas para timeframe {}", tf).into());
    }

    let records = get_all_intelligence_records_for_tf(&conn_models, tf)?;
    let target_rec = records
        .into_iter()
        .find(|r| r.level == target_level)
        .ok_or_else(|| format!("No se encontró el nivel {} en el historial de inteligencia para {}", target_level, tf))?;

    let level_res = IntelligenceLevelResult::from_stored_record(&target_rec)
        .ok_or_else(|| "Error al deserializar el registro del nivel de inteligencia")?;

    let dataset = prepare_dataset_with_horizon(&conn_candles, tf, level_res.config.horizon)?;

    // Ejecutamos la simulación determinista para obtener la lista exacta de trades
    let (m_nom, _, _, _, _) = run_online_learning_simulation_with_model(
        &dataset,
        &level_res.config,
        level_res.model.clone(),
    )?;

    let csv_filename = format!(
        "statement_L{}_{}_W{}_K{}_H{}_E{}_WD{:.4}_L1{:.6}__{:.0}X_{:.0}%.csv",
        level_res.level,
        level_res.config.tf,
        level_res.config.window_size,
        level_res.config.kernel_size,
        level_res.config.horizon,
        level_res.epoch,
        level_res.config.weight_decay,
        level_res.config.l1_lambda,
        level_res.config.leverage,
        level_res.config.capital_percent
    );

    let output_dirs = vec![
        format!("{}/data", tf),
        format!("{}/Trades", tf),
    ];

    let mut final_path = PathBuf::new();
    for dir in output_dirs {
        let path = PathBuf::from(&dir).join(&csv_filename);
        export_statement_csv(&m_nom.trades, &klines, tf, &path)?;
        println!("📄 Statement CSV guardado exitosamente en: {:?}", path);
        final_path = path;
    }

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_winner_l14_statement() {
        let res = export_statement_for_historical_level("1D", 14);
        assert!(res.is_ok(), "Error al exportar statement para L14: {:?}", res.err());
        let path = res.unwrap();
        assert!(path.exists(), "El archivo CSV exportado debe existir");
    }
}
