use super::models::{AccountBalanceInfo, UnifiedAccountPool};
use crate::config::ApiConfig;
use reqwest::Client;
use std::error::Error;

pub async fn fetch_unified_accounts(
    client: &Client,
    configs: &[ApiConfig],
) -> Result<UnifiedAccountPool, Box<dyn Error>> {
    let mut pool = UnifiedAccountPool::new();

    if configs.is_empty() {
        return Ok(pool);
    }

    let mut tasks = Vec::new();
    for cfg in configs {
        let client_cloned = client.clone();
        let cfg_cloned = cfg.clone();
        tasks.push(tokio::spawn(async move {
            let res = crate::binance::get_account_details(
                &client_cloned,
                &cfg_cloned.api_key,
                &cfg_cloned.api_secret,
                cfg_cloned.use_testnet,
            ).await.map_err(|e| e.to_string());
            (cfg_cloned, res)
        }));
    }

    let mut total_wallet = 0.0;
    let mut total_avail = 0.0;

    for task in tasks {
        if let Ok((cfg, res)) = task.await {
            match res {
                Ok(details) => {
                    total_wallet += details.wallet_balance;
                    total_avail += details.available_margin;
                    pool.accounts.push(AccountBalanceInfo {
                        id: cfg.id,
                        timeframe: cfg.timeframe,
                        api_key: cfg.api_key,
                        api_secret: cfg.api_secret,
                        leverage: cfg.leverage,
                        exchange: cfg.exchange,
                        use_testnet: cfg.use_testnet,
                        wallet_balance: details.wallet_balance,
                        available_margin: details.available_margin,
                        user_id: details.user_id,
                    });
                }
                Err(e) => {
                    eprintln!("⚠️ Error al consultar balance de cuenta #{}: {}", cfg.id, e);
                }
            }
        }
    }

    pool.total_wallet_balance = total_wallet;
    pool.total_available_margin = total_avail;
    pool.last_updated = chrono::Utc::now().timestamp();

    Ok(pool)
}

pub fn print_unified_accounts_summary(pool: &UnifiedAccountPool) {
    println!("\n╔═══════════════════════════════════════════════════════════════════════════╗");
    println!("║                   RESUMEN DE CUENTAS Y CAPITAL UNIFICADO                  ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════╣");
    if pool.accounts.is_empty() {
        println!("║ ⚠️ No se detectaron cuentas activas con API Key válida o conexión exitosa ║");
    } else {
        for (i, acc) in pool.accounts.iter().enumerate() {
            let net_str = if acc.use_testnet { "TESTNET" } else { "PRODUCCIÓN" };
            let user_str = if acc.user_id.is_empty() { "N/A" } else { &acc.user_id };
            let masked_key = if acc.api_key.len() > 8 {
                format!("{}...{}", &acc.api_key[..4], &acc.api_key[acc.api_key.len()-4..])
            } else {
                acc.api_key.clone()
            };
            println!(
                "║ [{}] Cuenta #{} (UID: {}) | Ex: {} [{}] | Key: {} | Bal: ${:.3} | Disp: ${:.3}",
                i + 1, acc.id, user_str, acc.exchange, net_str, masked_key, acc.wallet_balance, acc.available_margin
            );
        }
        println!("╠═══════════════════════════════════════════════════════════════════════════╣");
        println!(
            "║ 💰 CAPITAL UNIFICADO TOTAL: ${:.3} USDT (Disponible: ${:.3} USDT)",
            pool.total_wallet_balance, pool.total_available_margin
        );
    }
    println!("╚═══════════════════════════════════════════════════════════════════════════╝\n");
}

pub async fn get_pool_position_state(
    client: &Client,
    pool: &UnifiedAccountPool,
) -> f32 {
    if pool.accounts.is_empty() {
        return 0.0;
    }

    let mut long_count = 0;
    let mut short_count = 0;

    for acc in &pool.accounts {
        if let Ok(positions) = crate::binance::get_open_positions(
            client,
            &acc.api_key,
            &acc.api_secret,
            "BTC-USDT",
            acc.use_testnet,
        ).await {
            for pos in &positions {
                let (dir, amt) = crate::binance::parse_bingx_position(pos);
                if amt > 0.0 {
                    match dir {
                        crate::binance::PositionDirection::Long => long_count += 1,
                        crate::binance::PositionDirection::Short => short_count += 1,
                        crate::binance::PositionDirection::Flat => {}
                    }
                }
            }
        }
    }

    if long_count > short_count {
        1.0
    } else if short_count > long_count {
        -1.0
    } else {
        0.0
    }
}

#[derive(Debug, Clone)]
pub struct PositionSummary {
    pub account_id: u32,
    pub symbol: String,
    pub direction: crate::binance::PositionDirection,
    pub amount: f64,
    pub entry_price: f64,
    pub unrealized_pnl: f64,
    pub pnl_percent: f64,
    pub leverage: f64,
}

pub async fn fetch_open_positions_summary(
    client: &Client,
    pool: &UnifiedAccountPool,
) -> Vec<PositionSummary> {
    let mut list = Vec::new();
    for acc in &pool.accounts {
        if let Ok(positions) = crate::binance::get_open_positions(
            client,
            &acc.api_key,
            &acc.api_secret,
            "BTC-USDT",
            acc.use_testnet,
        ).await {
            for pos in &positions {
                let (dir, amt) = crate::binance::parse_bingx_position(pos);
                if dir != crate::binance::PositionDirection::Flat && amt > 0.0 {
                    let entry: f64 = pos.get("avgPrice")
                        .or_else(|| pos.get("entryPrice"))
                        .and_then(|p| p.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let pnl: f64 = pos.get("unrealizedProfit")
                        .and_then(|p| p.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let lev: f64 = pos.get("leverage")
                        .and_then(|l| if let Some(s) = l.as_str() { s.parse().ok() } else { l.as_f64() })
                        .unwrap_or(acc.leverage as f64);

                    let initial_margin = if lev > 0.0 && entry > 0.0 {
                        (amt * entry) / lev
                    } else {
                        1.0
                    };
                    let pnl_pct = if initial_margin > 0.0 { (pnl / initial_margin) * 100.0 } else { 0.0 };

                    list.push(PositionSummary {
                        account_id: acc.id,
                        symbol: "BTC-USDT".to_string(),
                        direction: dir,
                        amount: amt,
                        entry_price: entry,
                        unrealized_pnl: pnl,
                        pnl_percent: pnl_pct,
                        leverage: lev,
                    });
                }
            }
        }
    }
    list
}

