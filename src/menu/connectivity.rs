use crate::binance;
use crate::db::{self, Connection};
use reqwest::Client;
use std::error::Error;

pub async fn show_connectivity_test(client: &Client, conn: &Connection) -> Result<(), Box<dyn Error>> {
    println!("\n================================================");
    println!("        TEST DE CONECTIVIDAD API (BINGX)        ");
    println!("================================================");

    let all_configs = db::get_all_api_configs(conn).unwrap_or_default();
    if all_configs.is_empty() {
        println!("⚠️ No hay API Keys configuradas en la base de datos.");
        println!("Por favor configure al menos una cuenta en la opción de Configurar API Keys.");
        return Ok(());
    }

    println!("Probando conexión para {} cuenta(s)...", all_configs.len());
    let mut success_count = 0;

    for cfg in &all_configs {
        let key_trunc = if cfg.api_key.len() > 8 {
            format!("{}...", &cfg.api_key[..8])
        } else {
            cfg.api_key.clone()
        };
        print!("  - Cuenta #{} [{}] (Key: {}, Testnet: {})... ", cfg.id, cfg.timeframe, key_trunc, cfg.use_testnet);
        
        match binance::test_api_connection(client, &cfg.api_key, &cfg.api_secret, cfg.use_testnet).await {
            Ok(_) => {
                println!("✅ CONEXIÓN EXITOSA");
                success_count += 1;
            }
            Err(e) => {
                println!("❌ ERROR: {}", e);
            }
        }
    }

    println!("\nResumen: {}/{} cuentas conectadas correctamente.", success_count, all_configs.len());
    Ok(())
}
