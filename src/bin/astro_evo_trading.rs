use std::error::Error;
use std::io::{self, Write};
use reqwest::Client;

use astro_evo::db;
use astro_evo::live_trading;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn SetProcessAffinityMask(h_process: *mut std::ffi::c_void, dw_process_affinity_mask: usize) -> i32;
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
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

    let conn_candles = db::init_candles_db()?;
    let conn_models = db::init_models_db()?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| Client::new());

    loop {
        let (cp_live, lev_live, trade_wh_live, status_wh_live, _, _) =
            db::get_config(&conn_models).unwrap_or((10.0, 10.0, String::new(), String::new(), "Astro-EVO-Server".to_string(), 60));

        println!("\n================================================");
        println!("       ASTRO EVO TRADING (LIVE ENGINE)          ");
        println!("================================================");
        println!("1) Configurar API Keys (BingX)");
        println!("2) Consulta de Saldos (BingX)");
        println!("3) Test de Conectividad API (BingX)");
        println!("4) Configurar Webhooks de Discord (Servidor y por Temporalidad)");
        println!("5) Gestionar / Borrar Modelos Guardados (models.db)");
        println!("6) Iniciar Trading en Vivo en esta consola");
        println!("7) Salir");
        print!("> ");
        io::stdout().flush()?;

        let mut live_choice = String::new();
        io::stdin().read_line(&mut live_choice)?;

        match live_choice.trim() {
            "1" => {
                if let Err(e) = astro_evo::menu::api_keys::show_api_keys_menu(&conn_models) {
                    eprintln!("Error en menú de API Keys: {}", e);
                }
            }
            "2" => {
                if let Err(e) = astro_evo::menu::balances::show_balances_menu(&client, &conn_models).await {
                    eprintln!("Error en consulta de saldos: {}", e);
                }
            }
            "3" => {
                if let Err(e) = astro_evo::menu::connectivity::show_connectivity_test(&client, &conn_models).await {
                    eprintln!("Error en prueba de conectividad: {}", e);
                }
            }
            "4" => {
                if let Err(e) = astro_evo::menu::webhooks::show_webhooks_menu(&client, &conn_models).await {
                    eprintln!("Error en configuración de webhooks: {}", e);
                }
            }
            "5" => {
                if let Err(e) = astro_evo::menu::models_db::show_models_management_menu(&conn_models) {
                    eprintln!("Error en gestión de modelos: {}", e);
                }
            }
            "6" => {
                // 1. Obtener todas las cuentas con API Key
                let api_configs = db::get_all_api_configs(&conn_models).unwrap_or_default();
                if api_configs.is_empty() {
                    println!("⚠️ No hay API Keys configuradas en la base de datos.");
                    println!("Por favor configure sus claves en la Opción 1 antes de iniciar el Trading en Vivo.");
                    continue;
                }

                // 2. Consultar balances en tiempo real y mostrar Capital Unificado
                println!("🔍 Conectando con las cuentas y unificando capital...");
                let pool = match live_trading::account_manager::fetch_unified_accounts(&client, &api_configs).await {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("⚠️ Error consultando cuentas: {}", e);
                        continue;
                    }
                };

                live_trading::account_manager::print_unified_accounts_summary(&pool);

                if pool.is_empty() {
                    println!("⚠️ No se pudo obtener balance de ninguna cuenta conectada.");
                    continue;
                }

                // 3. Determinar el % de Capital Unificado a operar antes de iniciar
                print!("⚡ Ingrese % de Capital Unificado a operar por trade (defecto {:.1}%): ", cp_live);
                io::stdout().flush()?;
                let mut cp_in = String::new();
                io::stdin().read_line(&mut cp_in)?;
                let active_capital_percent: f64 = cp_in.trim().parse().unwrap_or(cp_live);

                print!("⚙️ Ingrese Apalancamiento a utilizar (defecto {:.0}X): ", lev_live);
                io::stdout().flush()?;
                let mut lev_in = String::new();
                io::stdin().read_line(&mut lev_in)?;
                let active_leverage: f64 = lev_in.trim().parse().unwrap_or(lev_live);

                print!("⏱️ Ingrese Temporalidades a operar en simultáneo (ej: 1D, 4H o ENTER para 1D): ");
                io::stdout().flush()?;
                let mut tf_in = String::new();
                io::stdin().read_line(&mut tf_in)?;
                let mut selected_tfs: Vec<String> = tf_in.split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if selected_tfs.is_empty() {
                    selected_tfs.push("1D".to_string());
                }

                println!("\n✅ Configuración confirmada: Operando {:.2}% del Capital Unificado a {:.0}X en temporalidades: {:?}.", active_capital_percent, active_leverage, selected_tfs);

                live_trading::run_live_trading_loop(
                    &client,
                    &conn_candles,
                    &conn_models,
                    active_capital_percent,
                    active_leverage,
                    &trade_wh_live,
                    &status_wh_live,
                    selected_tfs,
                ).await;
            }
            "7" => {
                println!("Saliendo de Astro EVO Trading...");
                break;
            }
            _ => println!("Opción no válida."),
        }
    }

    Ok(())
}
