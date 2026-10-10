use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;
use reqwest::Client;
use rayon::prelude::*;

use astro_evo::binance;
use astro_evo::db;
use astro_evo::graphics;
use astro_evo::online_learning::{self, OnlineLearningConfig};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn SetProcessAffinityMask(h_process: *mut std::ffi::c_void, dw_process_affinity_mask: usize) -> i32;
}

#[tokio::main(flavor = "multi_thread", worker_threads = 28)]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Ok(cores) = std::thread::available_parallelism() {
        let cores = cores.get();
        if cores > 4 {
            let limit = (cores - 4).min(usize::BITS as usize - 1);
            let mask: usize = (1 << limit) - 1;
            #[cfg(target_os = "windows")]
            unsafe {
                let _ = SetProcessAffinityMask(GetCurrentProcess(), mask);
            }
        }
    }

    unsafe {
        std::env::set_var("RAYON_NUM_THREADS", "28");
    }
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(28)
        .build_global();

    let mut conn_candles = db::init_candles_db()?;
    let conn_models = db::init_models_db()?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| Client::new());

    loop {
        let (capital_percent, leverage, _trade_webhook, _status_webhook, _server_name, _status_interval) = 
            db::get_config(&conn_models).unwrap_or((10.0, 10.0, String::new(), String::new(), "Astro-EVO-Server".to_string(), 60));

        println!("\n================================================");
        println!("               ASTRO EVO SYSTEM                 ");
        println!("================================================");
        println!("1. Base de Datos (DB)");
        println!("2. Batch Multiparámétrico de Online Learning");
        println!("3. Optimización Algorítmica por Bloques (Entrenamiento Original)");
        println!("4. Optimización Algorítmica por Bloques (con Mutación de Inteligencia L ➔ 2L ➔ 2L)");
        println!("5. Salir");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "5" {
            println!("Saliendo de Astro EVO...");
            break;
        }



        match input {
            "1" => {
                loop {
                    println!("\n--- Módulo Base de Datos (DB) ---");
                    println!("1) Contar Registros");
                    println!("2) Descargar/Actualizar Velas desde Binance API");
                    println!("3) Gestionar / Borrar Modelos Guardados (models.db)");
                    println!("4) Ver / Regenerar Dashboard Histórico Acumulado de Inteligencia");
                    println!("5) Volver al menú principal");
                    print!("> ");
                    io::stdout().flush()?;

                    let mut db_choice = String::new();
                    io::stdin().read_line(&mut db_choice)?;
                    let db_choice = db_choice.trim();

                    match db_choice {
                        "1" => {
                            println!("\n[Contar Registros en Candles DB]");
                            if let Err(e) = db::count_db(&conn_candles) {
                                eprintln!("Error al contar registros: {}", e);
                            }
                        }
                        "2" => {
                            println!("\n[Actualizar Velas desde Binance API]");
                            if let Err(e) = update_db(&mut conn_candles, &client).await {
                                eprintln!("Error actualizando DB: {}", e);
                            }
                        }
                        "3" => {
                            if let Err(e) = astro_evo::menu::models_db::show_models_management_menu(&conn_models) {
                                eprintln!("Error en gestión de modelos: {}", e);
                            }
                        }
                        "4" => {
                            if let Err(e) = astro_evo::intelligence::show_interactive_intelligence_history_menu(&conn_candles, &conn_models) {
                                eprintln!("Error en visor de historial de inteligencia: {}", e);
                            }
                        }
                        _ => break,
                    }
                }
            }
            "2" => {
                println!("\n================================================");
                println!("    BATCH MULTIPARAMÉTRICO DE ONLINE LEARNING   ");
                println!("================================================");
                
                print!("Timeframes a probar (defecto 1D): ");
                io::stdout().flush()?;
                let mut tfs_in = String::new();
                io::stdin().read_line(&mut tfs_in)?;
                let mut tfs: Vec<String> = tfs_in.split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if tfs.is_empty() { tfs.push("1D".to_string()); }

                print!("Porcentajes de capital (defecto 10): ");
                io::stdout().flush()?;
                let mut cp_grid = String::new();
                io::stdin().read_line(&mut cp_grid)?;
                let capital_percents = parse_f64_grid(&cp_grid, capital_percent);

                print!("Apalancamientos (defecto 10): ");
                io::stdout().flush()?;
                let mut lev_grid = String::new();
                io::stdin().read_line(&mut lev_grid)?;
                let leverages = parse_f64_grid(&lev_grid, leverage);

                print!("Ventanas W (defecto 30): ");
                io::stdout().flush()?;
                let mut w_grid = String::new();
                io::stdin().read_line(&mut w_grid)?;
                let windows = parse_usize_grid(&w_grid, 30);

                print!("Kernels K (defecto 15): ");
                io::stdout().flush()?;
                let mut k_grid = String::new();
                io::stdin().read_line(&mut k_grid)?;
                let kernels = parse_usize_grid(&k_grid, 15);

                print!("Horizontes de predicción H (defecto 10): ");
                io::stdout().flush()?;
                let mut h_grid = String::new();
                io::stdin().read_line(&mut h_grid)?;
                let horizons = parse_usize_grid(&h_grid, 10);

                // Learning Rate Base (Adaptativo por Performance / Drawdown automáticamente)
                let lrs = vec![0.001f32];

                print!("Weight Decays (defecto 0.001): ");
                io::stdout().flush()?;
                let mut wd_grid = String::new();
                io::stdin().read_line(&mut wd_grid)?;
                let wds = parse_f32_grid(&wd_grid, 0.001);

                print!("L1 Lambdas (defecto 0.0002): ");
                io::stdout().flush()?;
                let mut l1_grid = String::new();
                io::stdin().read_line(&mut l1_grid)?;
                let l1s = parse_f32_grid(&l1_grid, 0.0002);

                print!("Épocas de Pre-Entrenamiento IS (defecto 5): ");
                io::stdout().flush()?;
                let mut pre_grid = String::new();
                io::stdin().read_line(&mut pre_grid)?;
                let pre_epochs_list = parse_usize_grid(&pre_grid, 5);

                print!("Semilla Fija Determinista (defecto 42): ");
                io::stdout().flush()?;
                let mut seed_in = String::new();
                io::stdin().read_line(&mut seed_in)?;
                let seed_val: u32 = seed_in.trim().parse().unwrap_or(42);

                print!("Cantidad de redes en simultáneo a correr (defecto 4): ");
                io::stdout().flush()?;
                let mut threads_in = String::new();
                io::stdin().read_line(&mut threads_in)?;
                let num_threads: usize = threads_in.trim().parse().unwrap_or(4);

                let mut ol_base_configs = Vec::new();
                for &cp in &capital_percents {
                    for &lev in &leverages {
                        for tf_item in &tfs {
                            for &w in &windows {
                                for &k in &kernels {
                                    if k >= w { continue; }
                                    for &h in &horizons {
                                        for &lr in &lrs {
                                            for &wd in &wds {
                                                for &l1 in &l1s {
                                                    ol_base_configs.push(OnlineLearningConfig {
                                                        tf: tf_item.clone(),
                                                        capital_percent: cp,
                                                        leverage: lev,
                                                        window_size: w,
                                                        kernel_size: k,
                                                        horizon: h,
                                                        learning_rate: lr,
                                                        weight_decay: wd,
                                                        l1_lambda: l1,
                                                        seed_val,
                                                        pre_train_epochs: 0,
                                                        intelligence: 0,
                                                    });

                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let configs_len = ol_base_configs.len();
                let total_runs = configs_len * pre_epochs_list.len();
                println!("\n🚀 Lanzando {} configuraciones base (total {} evaluaciones por épocas) en paralelo con {} hilos...", configs_len, total_runs, num_threads);

                let mut datasets_map = HashMap::new();
                for tf_item in &tfs {
                    for &h in &horizons {
                        let key = format!("{}_H{}", tf_item, h);
                        println!("Cargando dataset en RAM para {} (Horizonte H={})...", tf_item, h);
                        match online_learning::prepare_dataset_with_horizon(&conn_candles, tf_item, h) {
                            Ok(ds) => { datasets_map.insert(key, Arc::new(ds)); }
                            Err(e) => eprintln!("Error al preparar dataset para {} (H={}): {}", tf_item, h, e),
                        }
                    }
                }

                let datasets_arc = Arc::new(datasets_map);
                let (tx, rx) = std::sync::mpsc::channel();

                let pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap();
                let ol_configs_arc = Arc::new(ol_base_configs);
                let pre_epochs_arc = Arc::new(pre_epochs_list);

                let pool_configs = Arc::clone(&ol_configs_arc);
                let pool_epochs = Arc::clone(&pre_epochs_arc);
                let pool_datasets = Arc::clone(&datasets_arc);
                pool.spawn(move || {
                    pool_configs.par_iter().for_each(|cfg| {
                        let ds_key = format!("{}_H{}", cfg.tf, cfg.horizon);
                        if let Some(ds) = pool_datasets.get(&ds_key) {
                            match online_learning::run_online_learning_incremental_epochs(ds, cfg, &pool_epochs) {
                                Ok(epoch_results) => {
                                    for res in epoch_results {
                                        let _ = tx.send((cfg.clone(), res));
                                    }
                                }
                                Err(e) => eprintln!("Error en tarea de Online Learning ({}, W{}, K{}, H{}): {}", cfg.tf, cfg.window_size, cfg.kernel_size, cfg.horizon, e),
                            }
                        }
                    });
                });

                let mut finished_count = 0;

                while let Ok((cfg, (target_epoch, m_nom, m_pct, model, eq_nom, eq_pct))) = rx.recv() {
                    let _assigned_db_id = match db::save_cnn1d_model_full(
                        &conn_models, &cfg.tf, cfg.window_size, cfg.kernel_size, target_epoch, &model, 
                        0.40, 0.40, true, cfg.leverage, cfg.capital_percent, cfg.weight_decay as f64, 
                        cfg.l1_lambda as f64, 0.0, cfg.horizon
                    ) {
                        Ok(id) => id as usize,
                        Err(e) => {
                            eprintln!("⚠️ Error al guardar época en DB: {}", e);
                            finished_count + 1
                        }
                    };

                    let img_name = format!("online_learning_backtest_{}_W{}_K{}_H{}_WD{:.4}_L1{:.6}__{:.0}X_{:.0}%.png", cfg.tf, cfg.window_size, cfg.kernel_size, cfg.horizon, cfg.weight_decay, cfg.l1_lambda, cfg.leverage, cfg.capital_percent);
                    let ds_key = format!("{}_H{}", cfg.tf, cfg.horizon);
                    if let Some(ds) = datasets_arc.get(&ds_key) {
                        let is_start_ts = astro_evo::indicators::parse_date_to_timestamp("20-04-2020").unwrap_or(0);
                        let oos_start_ts = astro_evo::indicators::parse_date_to_timestamp("01-01-2023").unwrap_or(0);
                        let mut is_start_idx = ds.klines.binary_search_by_key(&is_start_ts, |k| k.timestamp)
                            .unwrap_or_else(|idx| idx);
                        let min_idx = cfg.window_size + 200;
                        if is_start_idx < min_idx {
                            is_start_idx = min_idx;
                        }
                        let mut oos_start_idx = ds.klines.binary_search_by_key(&oos_start_ts, |k| k.timestamp)
                            .unwrap_or_else(|idx| idx);
                        if oos_start_idx <= is_start_idx || oos_start_idx >= ds.klines.len() {
                            oos_start_idx = is_start_idx + 100;
                        }

                        let _ = graphics::draw_equity_chart(
                            &ds.klines, &eq_nom, &cfg.tf, target_epoch, cfg.window_size, 
                            cfg.leverage, cfg.capital_percent, is_start_idx, oos_start_idx, 
                            m_nom.capital_final, m_nom.max_dd, m_nom.profit_factor, m_nom.winrate, &img_name
                        );

                        let html_name = format!("dashboard_dual_{}_W{}_K{}_H{}_E{}_WD{:.4}_L1{:.6}__{:.0}X_{:.0}%.html", cfg.tf, cfg.window_size, cfg.kernel_size, cfg.horizon, target_epoch, cfg.weight_decay, cfg.l1_lambda, cfg.leverage, cfg.capital_percent);
                        let _ = astro_evo::dashboard::generate_dual_dashboard(
                            "",
                            &ds.klines,
                            &cfg,
                            target_epoch,
                            &m_nom,
                            &m_pct,
                            &eq_nom,
                            &eq_pct,
                            is_start_idx,
                            oos_start_idx,
                            Some(&html_name),
                        );
                    }

                    finished_count += 1;
                    println!("Progress: {}/{} corridas completadas", finished_count, total_runs);
                }

                println!("\n¡Todo el lote de Online Learning Multiparámétrico ha finalizado con éxito!");
            }

            "3" => {
                if let Err(e) = astro_evo::intelligence::run_interactive_original_block_evolution(&conn_candles, &conn_models) {
                    eprintln!("⚠️ Error en optimización algorítmica por bloques (original): {}", e);
                }
            }
            "4" => {
                if let Err(e) = astro_evo::intelligence::run_interactive_block_evolution(&conn_candles, &conn_models) {
                    eprintln!("⚠️ Error en optimización por bloques con mutación de inteligencia: {}", e);
                }
            }
            _ => {
                println!("Opción no válida.");
            }
        }
    }



    Ok(())
}

async fn update_db(conn: &mut rusqlite::Connection, client: &Client) -> Result<(), Box<dyn Error>> {
    let symbol = "BTCUSDT";
    let intervals = [("1D", "1d"), ("12H", "12h"), ("8H", "8h"), ("4H", "4h"), ("1H", "1h"), ("15m", "15m"), ("5m", "5m"), ("1m", "1m")];

    for (db_tf, api_tf) in intervals {
        println!("Actualizando velas en {}...", db_tf);
        loop {
            let last_ts = db::get_latest_timestamp(conn, db_tf)?;
            
            let start_time = match last_ts {
                Some(ts) => Some(ts + 1),
                None => {
                    use chrono::{TimeZone, Utc};
                    Some(Utc.with_ymd_and_hms(2017, 8, 17, 0, 0, 0).unwrap().timestamp_millis())
                }
            };
            
            let klines = binance::download_klines(client, symbol, api_tf, start_time, 1000).await?;
            if klines.is_empty() {
                println!("- {} está completamente actualizado.", db_tf);
                break;
            }

            let len = klines.len();
            db::insert_klines(conn, db_tf, &klines)?;
            
            if let Some(last) = klines.last() {
                use chrono::{TimeZone, Utc};
                let date = Utc.timestamp_millis_opt(last.timestamp).unwrap();
                println!("  Guardadas {} velas de {}. Última: {}", len, db_tf, date.format("%Y-%m-%d %H:%M:%S"));
            }

            if len < 1000 {
                println!("- {} está completamente actualizado.", db_tf);
                break;
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }
    }

    Ok(())
}

fn parse_usize_grid(input: &str, default_val: usize) -> Vec<usize> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return vec![default_val];
    }
    if trimmed.contains(':') {
        let parts: Vec<usize> = trimmed.split(':')
            .map(|s| s.trim().parse().unwrap_or(0))
            .filter(|&v| v > 0)
            .collect();
        if parts.len() == 2 && parts[1] >= parts[0] {
            return (parts[0]..=parts[1]).collect();
        }
        if parts.len() == 3 && parts[1] >= parts[0] && parts[2] > 0 {
            let mut res = Vec::new();
            let mut v = parts[0];
            while v <= parts[1] {
                res.push(v);
                v += parts[2];
            }
            if !res.is_empty() { return res; }
        }
    }
    let parts: Vec<usize> = trimmed.split(|c| c == ',' || c == ';')
        .map(|s| s.trim().parse().unwrap_or(0))
        .filter(|&v| v > 0)
        .collect();
    if !parts.is_empty() {
        return parts;
    }
    vec![default_val]
}

fn parse_f32_grid(input: &str, default_val: f32) -> Vec<f32> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return vec![default_val];
    }
    if trimmed.contains(':') {
        let parts: Vec<f32> = trimmed.split(':')
            .map(|s| s.trim().replace(',', ".").parse().unwrap_or(-1.0))
            .filter(|&v| v >= 0.0)
            .collect();
        if parts.len() == 3 && parts[1] >= parts[0] && parts[2] > 0.0 {
            let mut res = Vec::new();
            let mut v = parts[0];
            while v <= parts[1] + 1e-6 {
                res.push(v);
                v += parts[2];
            }
            if !res.is_empty() { return res; }
        }
    }
    let parts: Vec<f32> = trimmed.split(|c| c == ',' || c == ';')
        .map(|s| s.trim().replace(',', ".").parse().unwrap_or(-1.0))
        .filter(|&v| v >= 0.0)
        .collect();
    if !parts.is_empty() {
        return parts;
    }
    vec![default_val]
}

fn parse_f64_grid(input: &str, default_val: f64) -> Vec<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return vec![default_val];
    }
    if trimmed.contains(':') {
        let parts: Vec<f64> = trimmed.split(':')
            .map(|s| s.trim().replace(',', ".").parse().unwrap_or(-1.0))
            .filter(|&v| v >= 0.0)
            .collect();
        if parts.len() == 3 && parts[1] >= parts[0] && parts[2] > 0.0 {
            let mut res = Vec::new();
            let mut v = parts[0];
            while v <= parts[1] + 1e-6 {
                res.push(v);
                v += parts[2];
            }
            if !res.is_empty() { return res; }
        }
    }
    let parts: Vec<f64> = trimmed.split(|c| c == ',' || c == ';')
        .map(|s| s.trim().replace(',', ".").parse().unwrap_or(-1.0))
        .filter(|&v| v >= 0.0)
        .collect();
    if !parts.is_empty() {
        return parts;
    }
    vec![default_val]
}
