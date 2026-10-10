pub mod models;
pub mod data_exporter;
pub mod html_template;

use std::error::Error;
use std::fs::{self, File};
use std::io::Write;

pub use models::{DashboardModelParams, DashboardSummaryMetrics};
pub use data_exporter::{export_equity_series, ExportedChartData};
pub use html_template::render_html_dashboard;
use crate::db::Kline;
use crate::online_learning::{AstroSubMetrics, OnlineLearningConfig};

pub fn generate_dual_dashboard(
    base_folder: &str,
    klines: &[Kline],
    config: &OnlineLearningConfig,
    epoch: usize,
    m_nom: &AstroSubMetrics,
    m_pct: &AstroSubMetrics,
    eq_curve_nom: &[(f32, f32)],
    eq_curve_pct: &[(f32, f32)],
    is_start_idx: usize,
    oos_start_idx: usize,
    custom_filename: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    let dashboard_dir = if base_folder.is_empty() || base_folder == "Astro EVO" {
        format!("{}/Dashboard/Individual", config.tf)
    } else {
        format!("{}/{}/Dashboard/Individual", base_folder, config.tf)
    };
    fs::create_dir_all(&dashboard_dir)?;

    let is_start_date = "2020-04-20".to_string();
    let oos_start_date = "2023-01-01".to_string();

    let chart_data = export_equity_series(klines, is_start_idx, eq_curve_nom, eq_curve_pct);

    let is_ret_nom_pct = if is_start_idx < eq_curve_nom.len() && oos_start_idx < eq_curve_nom.len() {
        let eq_is = eq_curve_nom[0].1 as f64;
        let eq_oos = eq_curve_nom.iter().find(|&&(x, _)| (x as usize) >= oos_start_idx).map(|&(_, y)| y as f64).unwrap_or(eq_is);
        if eq_is > 0.0 { ((eq_oos - eq_is) / eq_is) * 100.0 } else { 0.0 }
    } else { 0.0 };

    let oos_ret_nom_pct = if oos_start_idx < eq_curve_nom.len() {
        let eq_oos = eq_curve_nom.iter().find(|&&(x, _)| (x as usize) >= oos_start_idx).map(|&(_, y)| y as f64).unwrap_or(10000.0);
        let eq_final = m_nom.capital_final as f64;
        if eq_oos > 0.0 { ((eq_final - eq_oos) / eq_oos) * 100.0 } else { 0.0 }
    } else { 0.0 };

    let is_ret_pct_pct = if is_start_idx < eq_curve_pct.len() && oos_start_idx < eq_curve_pct.len() {
        let eq_is = eq_curve_pct[0].1 as f64;
        let eq_oos = eq_curve_pct.iter().find(|&&(x, _)| (x as usize) >= oos_start_idx).map(|&(_, y)| y as f64).unwrap_or(eq_is);
        if eq_is > 0.0 { ((eq_oos - eq_is) / eq_is) * 100.0 } else { 0.0 }
    } else { 0.0 };

    let oos_ret_pct_pct = if oos_start_idx < eq_curve_pct.len() {
        let eq_oos = eq_curve_pct.iter().find(|&&(x, _)| (x as usize) >= oos_start_idx).map(|&(_, y)| y as f64).unwrap_or(10000.0);
        let eq_final = m_pct.capital_final as f64;
        if eq_oos > 0.0 { ((eq_final - eq_oos) / eq_oos) * 100.0 } else { 0.0 }
    } else { 0.0 };

    let params = DashboardModelParams {
        tf: config.tf.clone(),
        window_size: config.window_size,
        kernel_size: config.kernel_size,
        horizon: config.horizon,
        weight_decay: config.weight_decay,
        l1_lambda: config.l1_lambda,
        epoch,
        leverage: config.leverage,
        capital_percent: config.capital_percent,
        is_start_date,
        oos_start_date,
        is_start_idx,
        oos_start_idx,
    };

    let summary = DashboardSummaryMetrics {
        capital_final_nom: m_nom.capital_final as f64,
        net_profit_nom: m_nom.net_profit,
        total_return_nom_pct: ((m_nom.capital_final as f64 - 10000.0) / 10000.0) * 100.0,
        max_dd_nom_pct: m_nom.max_dd as f64,
        max_dd_nom_dollar: m_nom.max_dd_nominal,
        winrate_nom: m_nom.winrate,
        profit_factor_nom: m_nom.profit_factor,
        total_trades_nom: m_nom.total_trades,
        longs_count_nom: m_nom.longs_count,
        shorts_count_nom: m_nom.shorts_count,
        liquidations_nom: m_nom.liquidations,
        slope_ratio_nom: m_nom.slope_ratio,
        is_return_nom_pct: is_ret_nom_pct,
        oos_return_nom_pct: oos_ret_nom_pct,

        capital_final_pct: m_pct.capital_final as f64,
        net_profit_pct: m_pct.net_profit,
        total_return_pct_pct: ((m_pct.capital_final as f64 - 10000.0) / 10000.0) * 100.0,
        max_dd_pct_pct: m_pct.max_dd as f64,
        max_dd_pct_dollar: m_pct.max_dd_nominal,
        winrate_pct: m_pct.winrate,
        profit_factor_pct: m_pct.profit_factor,
        total_trades_pct: m_pct.total_trades,
        longs_count_pct: m_pct.longs_count,
        shorts_count_pct: m_pct.shorts_count,
        liquidations_pct: m_pct.liquidations,
        slope_ratio_pct: m_pct.slope_ratio,
        is_return_pct_pct: is_ret_pct_pct,
        oos_return_pct_pct: oos_ret_pct_pct,
    };

    let html_content = render_html_dashboard(&params, &summary, &chart_data);

    let filename = match custom_filename {
        Some(name) => name.to_string(),
        None => format!(
            "dashboard_dual_{}_W{}_K{}_H{}_E{}_WD{:.4}_L1{:.6}__{:.0}X_{:.0}%.html",
            config.tf, config.window_size, config.kernel_size, config.horizon, epoch, config.weight_decay, config.l1_lambda, config.leverage, config.capital_percent
        ),
    };

    let full_path = format!("{}/{}", dashboard_dir, filename);
    let mut file = File::create(&full_path)?;
    file.write_all(html_content.as_bytes())?;

    println!("🌐 Dashboard HTML interactivo generado exitosamente en: {}", full_path);

    // Exportación automática del historial de trades en formato Myfxbook CSV
    if !m_nom.trades.is_empty() {
        let csv_filename = filename
            .replace("dashboard_dual_", "statement_")
            .replace(".html", ".csv");

        let trades_dirs = if base_folder.is_empty() || base_folder == "Astro EVO" {
            vec![
                format!("{}/data", config.tf),
                format!("{}/Trades", config.tf),
            ]
        } else {
            vec![
                format!("{}/{}/data", base_folder, config.tf),
                format!("{}/{}/Trades", base_folder, config.tf),
            ]
        };

        for t_dir in trades_dirs {
            let csv_path_buf = std::path::PathBuf::from(&t_dir).join(&csv_filename);
            if let Err(e) = crate::statement::export_statement_csv(&m_nom.trades, klines, &config.tf, &csv_path_buf) {
                eprintln!("⚠️ Error al exportar statement CSV en {:?}: {}", csv_path_buf, e);
            } else {
                println!("📄 Historial de Trades CSV exportado exitosamente en: {:?}", csv_path_buf);
            }
        }
    }

    Ok(full_path)
}
