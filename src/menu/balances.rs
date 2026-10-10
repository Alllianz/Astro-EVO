use crate::binance;
use crate::db::{self, Connection};
use reqwest::Client;
use std::error::Error;
use std::io::{self, Write};

pub async fn show_balances_menu(client: &Client, conn: &Connection) -> Result<(), Box<dyn Error>> {
    println!("\n================================================");
    println!("          CONSULTA DE SALDOS (BINGX)           ");
    println!("================================================");

    let default_tfs = ["Cuenta Principal", "1D", "12H", "8H", "4H", "1H", "15m", "5m", "1m"];
    println!("Seleccione opción:");
    println!("0) Consultar TODAS las cuentas registradas");
    for (idx, tf) in default_tfs.iter().enumerate() {
        println!("{}) {}", idx + 1, tf);
    }
    print!("> ");
    io::stdout().flush()?;

    let mut tf_in = String::new();
    io::stdin().read_line(&mut tf_in)?;
    let tf_idx: usize = tf_in.trim().parse().unwrap_or(999);

    if tf_idx == 0 {
        let all_configs = db::get_all_api_configs(conn).unwrap_or_default();
        if all_configs.is_empty() {
            println!("⚠️ No hay API Keys configuradas en la base de datos.");
            return Ok(());
        }
        println!("\n🔍 Consultando {} cuentas registradas en BingX...", all_configs.len());
        for cfg in &all_configs {
            print_account_balance(client, cfg).await;
        }
    } else if tf_idx > 0 && tf_idx <= default_tfs.len() {
        let selected_tf = default_tfs[tf_idx - 1];
        let configs = db::get_api_configs_by_tf(conn, selected_tf).unwrap_or_default();
        if configs.is_empty() {
            println!("⚠️ No hay API Keys configuradas para la temporalidad {}", selected_tf);
        } else {
            for cfg in &configs {
                print_account_balance(client, cfg).await;
            }
        }
    } else {
        println!("Selección inválida.");
    }

    Ok(())
}

async fn print_account_balance(client: &Client, cfg: &crate::config::ApiConfig) {
    let key_trunc = if cfg.api_key.len() > 8 {
        format!("{}...", &cfg.api_key[..8])
    } else {
        cfg.api_key.clone()
    };
    println!("\n------------------------------------------------");
    println!("Cuenta ID #{} | TF: {} | Key: {} | Testnet: {}", cfg.id, cfg.timeframe, key_trunc, cfg.use_testnet);
    println!("------------------------------------------------");

    match binance::get_account_details(client, &cfg.api_key, &cfg.api_secret, cfg.use_testnet).await {
        Ok(info) => {
            println!("👤 ID Usuario BingX:    {}", info.user_id);
            println!("💰 Saldo de Billetera:  ${:.3} USDT", info.wallet_balance);
            println!("💵 Margen Disponible:   ${:.3} USDT", info.available_margin);

            if let Ok(positions) = binance::get_open_positions(client, &cfg.api_key, &cfg.api_secret, "BTC-USDT", cfg.use_testnet).await {
                let mut has_pos = false;
                for pos in &positions {
                    let (dir, amt) = binance::parse_bingx_position(pos);
                    if dir != binance::PositionDirection::Flat && amt > 0.0 {
                        has_pos = true;
                        let side_str = match dir {
                            binance::PositionDirection::Long => "LONG",
                            binance::PositionDirection::Short => "SHORT",
                            binance::PositionDirection::Flat => "FLAT",
                        };
                        let entry_val: f64 = pos.get("avgPrice")
                            .or_else(|| pos.get("entryPrice"))
                            .and_then(|p| p.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0.0);
                        let pnl_val: f64 = pos.get("unrealizedProfit")
                            .and_then(|p| p.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0.0);
                        println!("⚡ Posición Activa:      {} @ ${:.3} (PnL: ${:.3})", side_str, entry_val, pnl_val);
                    }
                }
                if !has_pos {
                    println!("⚪ Posición Actual:     Sin posiciones abiertas (FLAT)");
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Error al consultar cuenta #{}: {}", cfg.id, e);
        }
    }
}
