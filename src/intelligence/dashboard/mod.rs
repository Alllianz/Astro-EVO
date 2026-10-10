pub mod exporter;
pub mod template;

use crate::db::Kline;
use crate::intelligence::config::IntelligenceSweepConfig;
use crate::intelligence::models::IntelligenceLevelResult;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;

pub fn generate_intelligence_comparison_dashboard(
    base_folder: &str,
    klines: &[Kline],
    sweep_config: &IntelligenceSweepConfig,
    results: &[IntelligenceLevelResult],
    is_start_idx: usize,
    oos_start_idx: usize,
) -> Result<String, Box<dyn Error>> {
    let dashboard_dir = if base_folder.is_empty() || base_folder == "Astro EVO" {
        format!("{}/Dashboard/General", sweep_config.tf)
    } else {
        format!("{}/{}/Dashboard/General", base_folder, sweep_config.tf)
    };
    fs::create_dir_all(&dashboard_dir)?;

    let export_data = exporter::export_intelligence_comparison_data(
        klines,
        is_start_idx,
        oos_start_idx,
        results,
        &sweep_config.tf,
        sweep_config.start_intelligence,
        sweep_config.iterations,
        sweep_config.window_size,
        sweep_config.kernel_size,
        sweep_config.horizon,
        sweep_config.leverage,
        sweep_config.capital_percent,
    );

    let html_content = template::render_intelligence_dashboard_html(&export_data);

    let end_level = sweep_config.start_intelligence + sweep_config.iterations - 1;
    let filename = format!(
        "dashboard_intelligence_comparison_{}_L{}_to_L{}_W{}_K{}_H{}__{:.0}X_{:.0}%.html",
        sweep_config.tf,
        sweep_config.start_intelligence,
        end_level,
        sweep_config.window_size,
        sweep_config.kernel_size,
        sweep_config.horizon,
        sweep_config.leverage,
        sweep_config.capital_percent
    );

    let full_path = format!("{}/{}", dashboard_dir, filename);
    let mut file = File::create(&full_path)?;
    file.write_all(html_content.as_bytes())?;

    println!("\n🌐 [DASHBOARD COMPARATIVO DE INTELIGENCIA] Generado exitosamente en: {}", full_path);

    Ok(full_path)
}
