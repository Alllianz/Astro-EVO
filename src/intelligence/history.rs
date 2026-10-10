use super::config::IntelligenceSweepConfig;
use super::dashboard::generate_intelligence_comparison_dashboard;
use super::models::IntelligenceLevelResult;
use super::ranking::rank_intelligence_results;
use crate::db;
use crate::online_learning;
use rusqlite::Connection;
use std::error::Error;
use std::io::{self, Write};

/// Guarda un resultado de nivel de inteligencia en la tabla `intelligence_history`
pub fn save_intelligence_result(
    conn_models: &Connection,
    result: &IntelligenceLevelResult,
) -> Result<(), Box<dyn Error>> {
    let rec = result.to_stored_record();
    db::save_intelligence_record(conn_models, &rec)?;
    Ok(())
}

/// Carga todos los resultados históricos de inteligencia para una temporalidad dada
pub fn load_all_intelligence_results_for_tf(
    conn_models: &Connection,
    tf: &str,
) -> Result<Vec<IntelligenceLevelResult>, Box<dyn Error>> {
    let stored_records = db::get_all_intelligence_records_for_tf(conn_models, tf)?;
    let mut list = Vec::new();
    for rec in stored_records {
        if let Some(item) = IntelligenceLevelResult::from_stored_record(&rec) {
            list.push(item);
        }
    }
    Ok(list)
}

/// Guarda los resultados actuales y los fusiona con todo el historial previo del timeframe
pub fn merge_and_rank_with_history(
    conn_models: &Connection,
    current_results: &[IntelligenceLevelResult],
    tf: &str,
) -> Result<Vec<IntelligenceLevelResult>, Box<dyn Error>> {
    // 1. Guardar o actualizar cada resultado de la corrida actual en SQLite
    for res in current_results {
        let _ = save_intelligence_result(conn_models, res);
    }

    // 2. Cargar todos los niveles persistidos históricamente para esta temporalidad
    let mut all_levels = load_all_intelligence_results_for_tf(conn_models, tf)?;

    // Si por alguna razón la BD no devolvió nada, usamos los actuales
    if all_levels.is_empty() {
        all_levels = current_results.to_vec();
    }

    // 3. Re-clasificar multicriterio sobre la totalidad de los niveles históricos combinados
    rank_intelligence_results(&mut all_levels);

    Ok(all_levels)
}

/// Genera el Dashboard HTML Comparativo que une todas las sesiones y niveles históricos para un timeframe
pub fn generate_accumulated_dashboard_for_tf(
    conn_candles: &Connection,
    conn_models: &Connection,
    tf: &str,
) -> Result<String, Box<dyn Error>> {
    let mut all_results = load_all_intelligence_results_for_tf(conn_models, tf)?;
    if all_results.is_empty() {
        return Err(format!("No hay registros de inteligencia guardados para la temporalidad {}", tf).into());
    }

    // Clasificar multicriterio
    rank_intelligence_results(&mut all_results);

    let first = all_results.first().unwrap();
    let min_level = all_results.iter().map(|r| r.level).min().unwrap_or(1);
    let max_level = all_results.iter().map(|r| r.level).max().unwrap_or(1);
    let iterations = (max_level - min_level) + 1;

    let horizon = first.config.horizon;
    let dataset = online_learning::prepare_dataset_with_horizon(conn_candles, tf, horizon)?;

    let is_start_ts = crate::indicators::parse_date_to_timestamp("20-04-2020").unwrap_or(0);
    let oos_start_ts = crate::indicators::parse_date_to_timestamp("01-01-2023").unwrap_or(0);
    let mut is_start_idx = dataset.klines.binary_search_by_key(&is_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    let min_idx = first.config.window_size + 200;
    if is_start_idx < min_idx { is_start_idx = min_idx; }
    let mut oos_start_idx = dataset.klines.binary_search_by_key(&oos_start_ts, |k| k.timestamp)
        .unwrap_or_else(|idx| idx);
    if oos_start_idx <= is_start_idx || oos_start_idx >= dataset.klines.len() {
        oos_start_idx = is_start_idx + 100;
    }

    let sweep_config = IntelligenceSweepConfig {
        tf: tf.to_string(),
        start_intelligence: min_level,
        iterations,
        capital_percent: first.config.capital_percent,
        leverage: first.config.leverage,
        window_size: first.config.window_size,
        kernel_size: first.config.kernel_size,
        horizon: first.config.horizon,
        learning_rate: first.config.learning_rate,
        weight_decay: first.config.weight_decay,
        l1_lambda: first.config.l1_lambda,
        seed_val: 42,
        pre_train_epochs: first.epoch,
        num_threads: 4,
    };

    println!("\n=========================================================================================================");
    println!("     🏆 RESUMEN MULTI-SESIÓN DE INTELIGENCIA ACUMULADA ({}) — TOTAL NIVELES: {}                         ", tf, all_results.len());
    println!("=========================================================================================================");
    println!("{:<4} | {:<8} | {:<16} | {:<10} | {:<12} | {:<10} | {:<8} | {:<8} | {:<8} | {}",
        "Pos", "Nivel", "Neuronas", "Parámetros", "Net Profit", "Max DD", "Win Rate", "Sortino", "R² Glob", "Rank Ponderado"
    );
    println!("---------------------------------------------------------------------------------------------------------");

    for (pos, res) in all_results.iter().enumerate() {
        let neurons = format!("{}➔{}➔{}", res.layer_info.conv1_out, res.layer_info.conv2_out, res.layer_info.dense_in);
        println!("{:<4} | L={:<6} | {:<16} | {:<10} | ${:<11.2} | {:<9.2}% | {:<7.2}% | {:<8.2} | {:<8.4} | #{:.2}",
            pos + 1,
            res.level,
            neurons,
            res.layer_info.total_weights_count,
            res.metrics_nom.net_profit,
            res.metrics_nom.max_dd,
            res.metrics_nom.winrate,
            res.metrics_nom.sortino_ratio,
            res.metrics_nom.curve_r2,
            res.rank_score.weighted_avg_rank
        );
    }
    println!("=========================================================================================================");

    let dashboard_path = generate_intelligence_comparison_dashboard(
        "",
        &dataset.klines,
        &sweep_config,
        &all_results,
        is_start_idx,
        oos_start_idx,
    )?;

    println!("\n🎨 Generando Dashboards Duales interactivos y gráficos individuales para cada nivel de inteligencia...");
    for res in &all_results {
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

        let _ = crate::graphics::draw_equity_chart(
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

    Ok(dashboard_path)
}

/// Menú interactivo para consultar y regenerar dashboards de historial de inteligencia acumulado
pub fn show_interactive_intelligence_history_menu(
    conn_candles: &Connection,
    conn_models: &Connection,
) -> Result<(), Box<dyn Error>> {
    loop {
        let summaries = db::get_all_stored_intelligence_summaries(conn_models)?;
        println!("\n=========================================================================");
        println!("        HISTORIAL ACUMULADO Y DASHBOARD DE INTELIGENCIA (ASTRO EVO)      ");
        println!("=========================================================================");
        println!("1) Ver y Regenerar Dashboard Combinado para un Timeframe");
        println!("2) Listar todos los niveles de inteligencia guardados en SQLite");
        println!("3) Borrar un nivel de inteligencia específico");
        println!("4) Borrar todo el historial de inteligencia de un Timeframe");
        println!("5) Volver al menú principal");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                print!("\nIngrese la Temporalidad a visualizar (defecto 1D): ");
                io::stdout().flush()?;
                let mut tf_in = String::new();
                io::stdin().read_line(&mut tf_in)?;
                let tf = match tf_in.trim() {
                    "" => "1D".to_string(),
                    s => s.to_string().to_uppercase(),
                };

                println!("Generando dashboard combinado multi-sesión para {}...", tf);
                match generate_accumulated_dashboard_for_tf(conn_candles, conn_models, &tf) {
                    Ok(path) => {
                        println!("\n✨ ¡Dashboard Combinado Multi-Sesión generado con éxito!");
                        println!("   📂 Ruta: {}", path);
                    }
                    Err(e) => eprintln!("⚠️ No se pudo generar el dashboard: {}", e),
                }
            }
            "2" => {
                if summaries.is_empty() {
                    println!("\nℹ️ No hay registros guardados en intelligence_history.");
                } else {
                    println!("\n📜 [NIVELES DE INTELIGENCIA GUARDADOS EN MODELS.DB — TOTAL: {}]", summaries.len());
                    println!("──────────────────────────────────────────────────────────────────────────────────────────");
                    println!(" TF   | Nivel | Arquitectura (L➔2L➔2L) | Ventana W | Kernel K | H   | Épocas | Lev. | Cap%");
                    println!("──────────────────────────────────────────────────────────────────────────────────────────");
                    for s in &summaries {
                        let arch = format!("{} ➔ {} ➔ {}", s.level, s.level * 2, s.level * 2);
                        println!(
                            " {:<4} | L={:<4} | {:<22} | {:<9} | {:<8} | {:<3} | {:<6} | {:<4.0}X | {:<5.1}%",
                            s.timeframe, s.level, arch, s.window_size, s.kernel_size, s.horizon, s.epoch, s.leverage, s.capital_percent
                        );
                    }
                    println!("──────────────────────────────────────────────────────────────────────────────────────────");
                }
            }
            "3" => {
                print!("\nIngrese la Temporalidad (ej: 1D): ");
                io::stdout().flush()?;
                let mut tf_in = String::new();
                io::stdin().read_line(&mut tf_in)?;
                let tf = tf_in.trim().to_uppercase();

                print!("Ingrese el Nivel L a eliminar: ");
                io::stdout().flush()?;
                let mut lvl_in = String::new();
                io::stdin().read_line(&mut lvl_in)?;
                if let Ok(lvl) = lvl_in.trim().parse::<usize>() {
                    match db::delete_intelligence_record_by_level(conn_models, &tf, lvl) {
                        Ok(count) => {
                            if count > 0 {
                                println!("✅ Nivel L={} de {} eliminado correctamente.", lvl, tf);
                            } else {
                                println!("⚠️ No se encontró el registro para {} L={}.", tf, lvl);
                            }
                        }
                        Err(e) => eprintln!("❌ Error al borrar: {}", e),
                    }
                }
            }
            "4" => {
                print!("\nIngrese la Temporalidad a limpiar (ej: 1D): ");
                io::stdout().flush()?;
                let mut tf_in = String::new();
                io::stdin().read_line(&mut tf_in)?;
                let tf = tf_in.trim().to_uppercase();
                if !tf.is_empty() {
                    print!("¿Seguro que desea borrar TODOS los niveles de inteligencia de {}? (s/N): ", tf);
                    io::stdout().flush()?;
                    let mut conf = String::new();
                    io::stdin().read_line(&mut conf)?;
                    if conf.trim().eq_ignore_ascii_case("s") || conf.trim().eq_ignore_ascii_case("si") {
                        match db::delete_intelligence_records_for_tf(conn_models, &tf) {
                            Ok(count) => println!("✅ Se eliminaron {} registros de inteligencia de {}.", count, tf),
                            Err(e) => eprintln!("❌ Error al borrar: {}", e),
                        }
                    }
                }
            }
            _ => break,
        }
    }
    Ok(())
}
