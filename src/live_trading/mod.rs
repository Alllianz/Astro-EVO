pub mod account_manager;
pub mod model_manager;
pub mod models;
pub mod order_executor;
pub mod signal_generator;

use crate::db::{get_all_api_configs, Connection};
use chrono::{TimeZone, Utc};
use model_manager::LiveModelManager;
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;

pub fn tf_to_binance_interval(tf: &str) -> &str {
    match tf {
        "1D" | "1d" => "1d",
        "12H" | "12h" => "12h",
        "8H" | "8h" => "8h",
        "4H" | "4h" => "4h",
        "1H" | "1h" => "1h",
        "15M" | "15m" => "15m",
        "5M" | "5m" => "5m",
        "1M" | "1m" => "1m",
        _ => "1d",
    }
}

pub fn tf_to_duration_millis(tf: &str) -> i64 {
    match tf {
        "1m" | "1M" => 60 * 1000,
        "5m" | "5M" => 5 * 60 * 1000,
        "15m" | "15M" => 15 * 60 * 1000,
        "1H" | "1h" => 60 * 60 * 1000,
        "4H" | "4h" => 4 * 60 * 60 * 1000,
        "8H" | "8h" => 8 * 60 * 60 * 1000,
        "12H" | "12h" => 12 * 60 * 60 * 1000,
        "1D" | "1d" => 24 * 60 * 60 * 1000,
        _ => 24 * 60 * 60 * 1000,
    }
}

pub async fn sync_all_missing_klines_from_binance(
    client: &Client,
    conn_candles: &Connection,
    tf: &str,
) {
    let tf_duration = tf_to_duration_millis(tf);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let current_candle_start = (now_ms / tf_duration) * tf_duration;
    let max_closed_ts = current_candle_start - tf_duration;

    if let Ok(conn_clean) = Connection::open("candles.db") {
        let _ = conn_clean.execute(
            "DELETE FROM klines WHERE timeframe = ?1 AND timestamp > ?2",
            rusqlite::params![tf, max_closed_ts],
        );
    }

    let api_interval = tf_to_binance_interval(tf);
    let symbol = "BTCUSDT";
    let last_ts = crate::db::get_latest_timestamp(conn_candles, tf).unwrap_or(None);

    println!("🔄 [{}] Sincronizando velas históricas y recientes desde Binance...", tf);
    let mut current_start = last_ts.map(|t| t + 1);
    let mut total_downloaded = 0;

    loop {
        match crate::binance::download_klines(client, symbol, api_interval, current_start, 1000).await {
            Ok(klines) => {
                if klines.is_empty() {
                    break;
                }
                let len = klines.len();
                total_downloaded += len;
                if let Ok(mut conn_clone) = Connection::open("candles.db") {
                    let _ = crate::db::insert_klines(&mut conn_clone, tf, &klines);
                }
                if let Some(last) = klines.last() {
                    current_start = Some(last.timestamp + 1);
                }
                if len < 1000 {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            Err(e) => {
                eprintln!("⚠️ [{}] Error al sincronizar velas: {}", tf, e);
                break;
            }
        }
    }

    if total_downloaded > 0 {
        println!("✅ [{}] Sincronizadas {} velas nuevas con éxito.", tf, total_downloaded);
    } else {
        println!("✅ [{}] Velas al día.", tf);
    }
}

pub async fn sync_recent_klines_from_binance(
    client: &Client,
    _conn_candles: &Connection,
    tf: &str,
) {
    let api_interval = tf_to_binance_interval(tf);
    let symbol = "BTCUSDT";

    if let Ok(recent_klines) = crate::binance::download_klines(client, symbol, api_interval, None, 20).await {
        if !recent_klines.is_empty() {
            let mut conn_clone = match Connection::open("candles.db") {
                Ok(c) => c,
                Err(_) => return,
            };
            let _ = crate::db::insert_klines(&mut conn_clone, tf, &recent_klines);
        }
    }
}

pub async fn run_live_trading_tick(
    client: &Client,
    conn_candles: &Connection,
    conn_models: &Connection,
    model_mgr: &mut LiveModelManager,
    tf: &str,
    capital_percent: f64,
    leverage: f64,
    global_trade_webhook: &str,
    _status_webhook: &str,
) -> Result<(), Box<dyn Error>> {
    // 0. Sincronizar las últimas velas en tiempo real desde la API
    sync_recent_klines_from_binance(client, conn_candles, tf).await;

    // 1. Obtener todas las cuentas con API Key configuradas
    let api_configs = get_all_api_configs(conn_models).unwrap_or_default();
    if api_configs.is_empty() {
        println!("⚠️ [{}] Saltado: Sin API Keys configuradas en la base de datos.", tf);
        return Ok(());
    }

    // 2. Unificar capital consultando balances en vivo
    let pool = account_manager::fetch_unified_accounts(client, &api_configs).await?;
    if pool.is_empty() {
        println!("⚠️ [{}] Saltado: No se pudo conectar a ninguna cuenta registrada.", tf);
        return Ok(());
    }

    // 3. Consultar posición real de las cuentas
    let current_pos_val = account_manager::get_pool_position_state(client, &pool).await;

    // 4. Actualizar online learning en el clon de memoria si cerró una vela
    model_mgr.update_live_learning_on_tick(conn_candles, tf, pool.total_wallet_balance);

    // 5. Obtener el clon en memoria puesto al día (sin tocar SQLite)
    let instance = match model_mgr.get_or_init_live_model(conn_candles, conn_models, tf, pool.total_wallet_balance)? {
        Some(inst) => inst,
        None => {
            println!("⚠️ [{}] Saltado: Sin modelo entrenado en la base de datos.", tf);
            return Ok(());
        }
    };

    // 6. Evaluar señal con umbral dinámico y p_flat
    let eval = match signal_generator::evaluate_timeframe_signal_with_model(
        conn_candles,
        tf,
        &instance.live_model,
        instance.window_size,
        instance.horizon,
        current_pos_val,
    )? {
        Some(ev) => ev,
        None => return Ok(()),
    };

    println!(
        "[{}] Live Tick (H={}) | Prob Long: {:.4} | Short: {:.4} | Flat: {:.4} | DynThr: {:.4} | Pos Actual: {} | Cap Unificado: ${:.3}",
        tf, eval.horizon, eval.p_long, eval.p_short, eval.p_flat, eval.dynamic_thr, current_pos_val, pool.total_wallet_balance
    );

    // 7. Resolver el webhook de operaciones específico de esta temporalidad/modelo (o fallback al global)
    let specific_wh = crate::db::get_timeframe_webhook(conn_models, tf).ok().flatten();
    let effective_trade_wh = specific_wh.as_deref().unwrap_or(global_trade_webhook);

    // 8. Ejecutar órdenes proporcionales en el pool de cuentas unificadas y notificar al webhook del TF
    order_executor::execute_unified_signal(
        client,
        &pool,
        tf,
        &eval,
        capital_percent,
        leverage,
        effective_trade_wh,
    ).await?;

    Ok(())
}

pub async fn run_live_trading_loop(
    client: &Client,
    conn_candles: &Connection,
    conn_models: &Connection,
    capital_percent: f64,
    leverage: f64,
    trade_webhook: &str,
    status_webhook: &str,
    selected_tfs: Vec<String>,
) {
    let mut model_mgr = LiveModelManager::new();

    println!("\n▶️ Iniciando ciclo continuo de Trading en Vivo con Capital Unificado y Clones en Memoria...");
    println!("⏱️ Temporalidades / Modelos Activos en Simultáneo: {:?}", selected_tfs);
    println!("⚡ Porcentaje de Operación Asignado: {:.2}%", capital_percent);
    println!("⚙️ Apalancamiento Base: {:.0}X\n", leverage);

    // Sincronización completa inicial de todas las velas hasta hoy para cada temporalidad seleccionada
    for tf in &selected_tfs {
        sync_all_missing_klines_from_binance(client, conn_candles, tf).await;
        // Poner al día el modelo con todas las velas disponibles hasta hoy
        if let Ok(Some(inst)) = model_mgr.get_or_init_live_model(conn_candles, conn_models, tf, 10000.0) {
            // Mostrar la evaluación de la red sobre la última vela disponible
            if let Ok(Some(eval)) = signal_generator::evaluate_timeframe_signal_with_model(
                conn_candles,
                tf,
                &inst.live_model,
                inst.window_size,
                inst.horizon,
                0.0,
            ) {
                let sig_str = match eval.signal {
                    signal_generator::SignalType::Long => "LONG 🚀",
                    signal_generator::SignalType::Short => "SHORT 🔻",
                    signal_generator::SignalType::Flat => "FLAT ⚪",
                };
                let candle_date_str = Utc.timestamp_millis_opt(eval.candle_timestamp)
                    .single()
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                println!(
                    "🧠 [{} | Vela: {}] Evaluación Red: Prob Long: {:.3} | Short: {:.3} | Flat: {:.3} | DynThr: {:.3} -> Señal: {}",
                    tf, candle_date_str, eval.p_long, eval.p_short, eval.p_flat, eval.dynamic_thr, sig_str
                );
            }
        }
    }

    // Iniciar tarea en segundo plano para el Heartbeat de Estado del Servidor
    let status_wh = status_webhook.to_string();
    let client_for_status = client.clone();
    let (_, _, _, _, server_name, interval_min) = crate::db::get_config(conn_models)
        .unwrap_or((10.0, 10.0, String::new(), String::new(), "Astro-EVO-Server".to_string(), 60));

    if !status_wh.is_empty() && interval_min > 0 {
        let tfs_clone = selected_tfs.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_min * 60));
            loop {
                interval.tick().await;
                let info = format!(
                    "⏱️ **Modelos Activos:** `{:?}`\n⚡ **Operación:** `{:.2}%` | **Apalancamiento:** `{:.0}X`",
                    tfs_clone, capital_percent, leverage
                );
                crate::discord::send_server_status_webhook(&client_for_status, &status_wh, &server_name, &info).await;
            }
        });
    }

    let mut last_operated_candle_ts: HashMap<String, i64> = HashMap::new();
    let mut last_heartbeat_log_ts: HashMap<String, i64> = HashMap::new();

    // Inicializar marcas de tiempo en el startup
    let startup_now = Utc::now().timestamp_millis();
    for tf in &selected_tfs {
        let tf_duration = tf_to_duration_millis(tf);
        let current_candle_start = (startup_now / tf_duration) * tf_duration;
        let last_closed_ts = current_candle_start - tf_duration;

        // Si ya pasaron más de 2 minutos desde el cierre de la última vela (ej. estamos a mitad de vela),
        // marcarla como ya procesada para esperar al siguiente cierre
        if startup_now > current_candle_start + 120_000 {
            last_operated_candle_ts.insert(tf.to_string(), last_closed_ts);
        }
    }

    loop {
        let now_ms = Utc::now().timestamp_millis();

        for tf in &selected_tfs {
            let tf_duration = tf_to_duration_millis(tf);
            let current_candle_start = (now_ms / tf_duration) * tf_duration;
            let target_closed_ts = current_candle_start - tf_duration;
            let window_expiry_ts = current_candle_start + 120_000; // 2 minutos post-cierre

            let already_operated = last_operated_candle_ts.get(tf) == Some(&target_closed_ts);

            // 1. Si estamos dentro de la ventana de 2 minutos post-cierre de vela y no ha sido operada
            if now_ms >= current_candle_start && now_ms <= window_expiry_ts && !already_operated {
                println!(
                    "\n📡 [{}] ¡Cierre de vela alcanzado! Sincronizando nueva vela cerrada desde Binance...",
                    tf
                );

                // Intentar sincronizar la nueva vela cerrada desde Binance
                sync_recent_klines_from_binance(client, conn_candles, tf).await;

                let last_db_ts = crate::db::get_latest_timestamp(conn_candles, tf).unwrap_or(None).unwrap_or(0);

                // Si Binance ya entregó la nueva vela cerrada (o si han pasado al menos 5 segundos para evaluar)
                if last_db_ts >= target_closed_ts || now_ms >= current_candle_start + 5_000 {
                    let candle_date_str = Utc.timestamp_millis_opt(target_closed_ts)
                        .single()
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_default();
                    println!(
                        "⏰ [{}] Ventana de operación activa (+2 min post-cierre). Ejecutando evaluación y operativa para vela {}...",
                        tf, candle_date_str
                    );

                    let res = run_live_trading_tick(
                        client,
                        conn_candles,
                        conn_models,
                        &mut model_mgr,
                        tf,
                        capital_percent,
                        leverage,
                        trade_webhook,
                        status_webhook,
                    ).await;

                    if res.is_ok() {
                        last_operated_candle_ts.insert(tf.to_string(), target_closed_ts);

                        let next_close = current_candle_start + tf_duration;
                        let next_date = Utc.timestamp_millis_opt(next_close).unwrap();
                        println!(
                            "✅ [{}] Operación de vela completada exitosamente. Próximo cierre de vela programado a las {} UTC.",
                            tf,
                            next_date.format("%Y-%m-%d %H:%M:%S")
                        );
                    }
                } else {
                    println!("⏳ [{}] Aguardando consolidación de vela en Binance (reintentando en el próximo tick dentro de los 2 min)...", tf);
                }
            } 
            // 2. Si se venció la ventana de 2 minutos sin poder operar
            else if now_ms > window_expiry_ts && !already_operated {
                let candle_date_str = Utc.timestamp_millis_opt(target_closed_ts)
                    .single()
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_default();
                println!(
                    "⚠️ [{}] Ventana de 2 minutos expirada para el cierre actual ({}). Marcando como procesada.",
                    tf, candle_date_str
                );
                last_operated_candle_ts.insert(tf.to_string(), target_closed_ts);
            }
            // 3. Esperando el próximo cierre de vela: Monitoreo en Tiempo Real
            else {
                let last_log = last_heartbeat_log_ts.get(tf).copied().unwrap_or(0);
                if now_ms - last_log >= 10_000 {
                    last_heartbeat_log_ts.insert(tf.to_string(), now_ms);

                    // Consultar balances y posiciones abiertas en BingX en tiempo real
                    let api_configs = get_all_api_configs(conn_models).unwrap_or_default();
                    if let Ok(pool) = account_manager::fetch_unified_accounts(client, &api_configs).await {
                        let positions = account_manager::fetch_open_positions_summary(client, &pool).await;
                        let now_utc_str = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

                        println!("\n📊 ESTADO EN VIVO BINGX [{}]", now_utc_str);
                        println!(
                            "💰 Capital Unificado: ${:.3} USDT | Margen Disponible: ${:.3} USDT",
                            pool.total_wallet_balance, pool.total_available_margin
                        );
                        if positions.is_empty() {
                            println!("⚪ Posición: FLAT (Sin posiciones activas en BingX)");
                        } else {
                            for pos in &positions {
                                let dir_str = match pos.direction {
                                    crate::binance::PositionDirection::Long => "LONG 🚀",
                                    crate::binance::PositionDirection::Short => "SHORT 🔻",
                                    crate::binance::PositionDirection::Flat => "FLAT ⚪",
                                };
                                let pnl_sign = if pos.unrealized_pnl >= 0.0 { "+" } else { "-" };
                                let pct_sign = if pos.pnl_percent >= 0.0 { "+" } else { "" };
                                println!(
                                    "⚡ [Cuenta #{}] {} | Entrada: ${:.3} | PnL: {}${:.3} ({}{:.3}%) | {:.0}X",
                                    pos.account_id, dir_str, pos.entry_price, pnl_sign, pos.unrealized_pnl.abs(), pct_sign, pos.pnl_percent, pos.leverage
                                );
                            }
                        }

                        if let Some(inst) = model_mgr.instances.get(tf) {
                            let pos_val = account_manager::get_pool_position_state(client, &pool).await;
                            if let Ok(Some(eval)) = signal_generator::evaluate_timeframe_signal_with_model(
                                conn_candles,
                                tf,
                                &inst.live_model,
                                inst.window_size,
                                inst.horizon,
                                pos_val,
                            ) {
                                let sig_str = match eval.signal {
                                    signal_generator::SignalType::Long => "LONG 🚀",
                                    signal_generator::SignalType::Short => "SHORT 🔻",
                                    signal_generator::SignalType::Flat => "FLAT ⚪",
                                };
                                let candle_date_str = Utc.timestamp_millis_opt(eval.candle_timestamp)
                                    .single()
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                    .unwrap_or_else(|| "N/A".to_string());
                                println!(
                                    "🧠 [{} | Vela: {}] Predicción Red: Long: {:.3} | Short: {:.3} | Flat: {:.3} | DynThr: {:.3} -> {}",
                                    tf, candle_date_str, eval.p_long, eval.p_short, eval.p_flat, eval.dynamic_thr, sig_str
                                );
                            }
                        }

                        let next_close_ms = current_candle_start + tf_duration;
                        let diff_ms = next_close_ms - now_ms;
                        if diff_ms > 0 {
                            let total_secs = diff_ms / 1000;
                            let hours = total_secs / 3600;
                            let mins = (total_secs % 3600) / 60;
                            let secs = total_secs % 60;
                            let next_date = Utc.timestamp_millis_opt(next_close_ms).unwrap();
                            println!(
                                "⏳ [{}] Próximo cierre: {:02}h:{:02}m:{:02}s (a las {} UTC)",
                                tf, hours, mins, secs, next_date.format("%Y-%m-%d %H:%M:%S")
                            );
                        }
                    }
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}
