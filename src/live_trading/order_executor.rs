use super::models::{AccountBalanceInfo, UnifiedAccountPool};
use super::signal_generator::{SignalEvaluation, SignalType};
use reqwest::Client;
use std::error::Error;

pub async fn execute_unified_signal(
    client: &Client,
    pool: &UnifiedAccountPool,
    tf: &str,
    eval: &SignalEvaluation,
    capital_percent: f64,
    leverage: f64,
    trade_webhook: &str,
) -> Result<(), Box<dyn Error>> {
    if pool.accounts.is_empty() {
        return Ok(());
    }

    match eval.signal {
        SignalType::Flat => {
            println!("⚪ [{}] Señal FLAT detectada. Verificando y cerrando posiciones abiertas...", tf);
        }
        SignalType::Long => {
            println!("🚀 [{}] Señal LONG detectada. Ejecutando órdenes...", tf);
        }
        SignalType::Short => {
            println!("🔻 [{}] Señal SHORT detectada. Ejecutando órdenes...", tf);
        }
    }

    // Filtrar cuentas asignadas específicamente a este timeframe o cuentas globales/principales
    let matching_accounts: Vec<_> = pool.accounts.iter()
        .filter(|acc| {
            acc.timeframe.eq_ignore_ascii_case(tf) 
                || acc.timeframe.eq_ignore_ascii_case("Cuenta Principal")
                || acc.timeframe.eq_ignore_ascii_case("ALL")
                || acc.timeframe.is_empty()
        })
        .collect();

    let target_accounts = if matching_accounts.is_empty() {
        pool.accounts.iter().collect::<Vec<_>>()
    } else {
        matching_accounts
    };

    let mut tasks = Vec::new();

    for &acc in &target_accounts {
        let client_cloned = client.clone();
        let acc_cloned = acc.clone();
        let tf_owned = tf.to_string();
        let eval_cloned = eval.clone();
        let trade_wh_owned = trade_webhook.to_string();

        tasks.push(tokio::spawn(async move {
            let res = execute_account_signal(
                &client_cloned,
                &acc_cloned,
                &tf_owned,
                &eval_cloned,
                capital_percent,
                leverage,
                &trade_wh_owned,
            ).await;
            (acc_cloned.id, res)
        }));
    }

    for task in tasks {
        if let Ok((acc_id, res)) = task.await {
            if let Err(e) = res {
                eprintln!("⚠️ [{}] Error en ejecución de orden para cuenta #{}: {}", tf, acc_id, e);
            }
        }
    }

    Ok(())
}

async fn execute_account_signal(
    client: &Client,
    acc: &AccountBalanceInfo,
    tf: &str,
    eval: &SignalEvaluation,
    capital_percent: f64,
    leverage: f64,
    trade_webhook: &str,
) -> Result<(), String> {
    // 0. Sincronizar apalancamiento en la cuenta del Exchange (BingX)
    let lev_u32 = leverage.round().max(1.0) as u32;
    let _ = crate::binance::set_leverage(client, &acc.api_key, &acc.api_secret, "BTC-USDT", lev_u32, "LONG", acc.use_testnet).await;
    let _ = crate::binance::set_leverage(client, &acc.api_key, &acc.api_secret, "BTC-USDT", lev_u32, "SHORT", acc.use_testnet).await;

    // 1. Obtener posiciones abiertas en la cuenta
    let open_positions = crate::binance::get_open_positions(
        client,
        &acc.api_key,
        &acc.api_secret,
        "BTC-USDT",
        acc.use_testnet,
    ).await.unwrap_or_default();

    let mut current_pos_side = "NONE";
    let mut current_pos_amt = 0.0f64;
    for pos in &open_positions {
        let (dir, amt) = crate::binance::parse_bingx_position(pos);
        if amt > 0.0 {
            match dir {
                crate::binance::PositionDirection::Long => {
                    current_pos_side = "LONG";
                    current_pos_amt = amt;
                }
                crate::binance::PositionDirection::Short => {
                    current_pos_side = "SHORT";
                    current_pos_amt = amt;
                }
                crate::binance::PositionDirection::Flat => {}
            }
        }
    }

    // 2. Calcular tamaño proporcional de orden sobre el balance de esta cuenta
    let base_bal = if acc.available_margin > 0.0 { acc.available_margin } else { acc.wallet_balance };
    let notional = base_bal * (capital_percent / 100.0) * leverage;
    let order_qty = (notional / eval.last_price).max(0.001);

    // Redondear a 4 decimales
    let order_qty = (order_qty * 10000.0).round() / 10000.0;

    match eval.signal {
        SignalType::Long => {
            if current_pos_side == "SHORT" {
                let close_qty = if current_pos_amt > 0.0 { current_pos_amt } else { order_qty };
                println!("🔄 [{}] Cuenta #{}: Cerrando SHORT previo (Qty: {:.3})...", tf, acc.id, close_qty);
                let _ = crate::binance::open_market_order(
                    client, &acc.api_key, &acc.api_secret, "BTC-USDT", "BUY", "SHORT", close_qty, None, acc.use_testnet
                ).await;
                if !trade_webhook.is_empty() {
                    crate::discord::send_trade_alert_webhook(
                        client,
                        trade_webhook,
                        tf,
                        "FLAT_CLOSE_SHORT",
                        "BTC-USDT",
                        eval.last_price,
                        close_qty,
                        close_qty * eval.last_price,
                        leverage,
                        acc.id,
                        "Cierre de SHORT previo para dar paso a señal LONG",
                    ).await;
                }
            }
            if current_pos_side != "LONG" {
                if order_qty < 0.0001 {
                    println!("⚠️ [{}] Cuenta #{}: Capital insuficiente para el tamaño mínimo de orden.", tf, acc.id);
                    return Ok(());
                }
                println!(
                    "🚀 [{}] Cuenta #{}: Abriendo LONG (Qty: {:.3} BTC | Notional: ${:.3})...",
                    tf, acc.id, order_qty, notional
                );
                let is_ok = crate::binance::open_market_order(
                    client, &acc.api_key, &acc.api_secret, "BTC-USDT", "BUY", "LONG", order_qty, None, acc.use_testnet
                ).await.is_ok();
                if is_ok && !trade_webhook.is_empty() {
                    crate::discord::send_trade_alert_webhook(
                        client,
                        trade_webhook,
                        tf,
                        "LONG",
                        "BTC-USDT",
                        eval.last_price,
                        order_qty,
                        notional,
                        leverage,
                        acc.id,
                        &format!("Prob Long: {:.2}% | DynThr: {:.2}%", eval.p_long * 100.0, eval.dynamic_thr * 100.0),
                    ).await;
                }
            } else {
                println!("✓ [{}] Cuenta #{}: Posición LONG ya activa.", tf, acc.id);
            }
        }
        SignalType::Short => {
            if current_pos_side == "LONG" {
                let close_qty = if current_pos_amt > 0.0 { current_pos_amt } else { order_qty };
                println!("🔄 [{}] Cuenta #{}: Cerrando LONG previo (Qty: {:.3})...", tf, acc.id, close_qty);
                let _ = crate::binance::open_market_order(
                    client, &acc.api_key, &acc.api_secret, "BTC-USDT", "SELL", "LONG", close_qty, None, acc.use_testnet
                ).await;
                if !trade_webhook.is_empty() {
                    crate::discord::send_trade_alert_webhook(
                        client,
                        trade_webhook,
                        tf,
                        "FLAT_CLOSE_LONG",
                        "BTC-USDT",
                        eval.last_price,
                        close_qty,
                        close_qty * eval.last_price,
                        leverage,
                        acc.id,
                        "Cierre de LONG previo para dar paso a señal SHORT",
                    ).await;
                }
            }
            if current_pos_side != "SHORT" {
                if order_qty < 0.0001 {
                    println!("⚠️ [{}] Cuenta #{}: Capital insuficiente para el tamaño mínimo de orden.", tf, acc.id);
                    return Ok(());
                }
                println!(
                    "🔻 [{}] Cuenta #{}: Abriendo SHORT (Qty: {:.3} BTC | Notional: ${:.3})...",
                    tf, acc.id, order_qty, notional
                );
                let is_ok = crate::binance::open_market_order(
                    client, &acc.api_key, &acc.api_secret, "BTC-USDT", "SELL", "SHORT", order_qty, None, acc.use_testnet
                ).await.is_ok();
                if is_ok && !trade_webhook.is_empty() {
                    crate::discord::send_trade_alert_webhook(
                        client,
                        trade_webhook,
                        tf,
                        "SHORT",
                        "BTC-USDT",
                        eval.last_price,
                        order_qty,
                        notional,
                        leverage,
                        acc.id,
                        &format!("Prob Short: {:.2}% | DynThr: {:.2}%", eval.p_short * 100.0, eval.dynamic_thr * 100.0),
                    ).await;
                }
            } else {
                println!("✓ [{}] Cuenta #{}: Posición SHORT ya activa.", tf, acc.id);
            }
        }
        SignalType::Flat => {
            if current_pos_side == "LONG" {
                let close_qty = if current_pos_amt > 0.0 { current_pos_amt } else { order_qty };
                println!("🔄 [{}] Cuenta #{}: Cerrando LONG por señal FLAT (Qty: {:.3})...", tf, acc.id, close_qty);
                let _ = crate::binance::open_market_order(
                    client, &acc.api_key, &acc.api_secret, "BTC-USDT", "SELL", "LONG", close_qty, None, acc.use_testnet
                ).await;
                if !trade_webhook.is_empty() {
                    crate::discord::send_trade_alert_webhook(
                        client,
                        trade_webhook,
                        tf,
                        "FLAT_CLOSE_LONG",
                        "BTC-USDT",
                        eval.last_price,
                        close_qty,
                        close_qty * eval.last_price,
                        leverage,
                        acc.id,
                        &format!("Posición LONG cerrada por señal FLAT (Prob Flat: {:.2}%)", eval.p_flat * 100.0),
                    ).await;
                }
            } else if current_pos_side == "SHORT" {
                let close_qty = if current_pos_amt > 0.0 { current_pos_amt } else { order_qty };
                println!("🔄 [{}] Cuenta #{}: Cerrando SHORT por señal FLAT (Qty: {:.3})...", tf, acc.id, close_qty);
                let _ = crate::binance::open_market_order(
                    client, &acc.api_key, &acc.api_secret, "BTC-USDT", "BUY", "SHORT", close_qty, None, acc.use_testnet
                ).await;
                if !trade_webhook.is_empty() {
                    crate::discord::send_trade_alert_webhook(
                        client,
                        trade_webhook,
                        tf,
                        "FLAT_CLOSE_SHORT",
                        "BTC-USDT",
                        eval.last_price,
                        close_qty,
                        close_qty * eval.last_price,
                        leverage,
                        acc.id,
                        &format!("Posición SHORT cerrada por señal FLAT (Prob Flat: {:.2}%)", eval.p_flat * 100.0),
                    ).await;
                }
            } else {
                println!("⚪ [{}] Cuenta #{}: Sin posiciones abiertas (FLAT confirmado).", tf, acc.id);
            }
        }
    }

    Ok(())
}
