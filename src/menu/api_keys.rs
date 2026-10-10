use crate::db::{self, Connection};
use std::error::Error;
use std::io::{self, Write};

pub fn show_api_keys_menu(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let default_tfs = ["Cuenta Principal", "1D", "12H", "8H", "4H", "1H", "15m", "5m", "1m"];

    loop {
        println!("\n================================================");
        println!("       CONFIGURAR API KEYS (BINGX)              ");
        println!("================================================");
        println!("Seleccione temporalidad a configurar:");
        for (idx, tf) in default_tfs.iter().enumerate() {
            let count = db::get_api_configs_by_tf(conn, tf).map(|v| v.len()).unwrap_or(0);
            println!("{}) {} ({} cuentas activas)", idx + 1, tf, count);
        }
        println!("10) Clonar configuración de un TF a todos");
        println!("11) Borrar todas las credenciales de la DB");
        println!("12) Volver al menú principal");
        print!("> ");
        io::stdout().flush()?;

        let mut api_input = String::new();
        io::stdin().read_line(&mut api_input)?;
        let api_choice = api_input.trim();

        if api_choice == "12" {
            break;
        }

        if api_choice == "11" {
            print!("¿Está seguro de que desea eliminar todas las API Keys registradas? (s/N): ");
            io::stdout().flush()?;
            let mut confirm = String::new();
            io::stdin().read_line(&mut confirm)?;
            if matches!(confirm.trim().to_lowercase().as_str(), "s" | "si" | "y" | "yes") {
                let _ = conn.execute("DELETE FROM api_config", []);
                println!("¡Todas las credenciales fueron eliminadas con éxito!");
            }
            continue;
        }

        if api_choice == "10" {
            println!("\n[Clonar Configuración de Cuentas]");
            println!("Seleccione el TF origen:");
            for (idx, tf) in default_tfs.iter().enumerate() {
                println!("{}) {}", idx + 1, tf);
            }
            print!("> ");
            io::stdout().flush()?;
            let mut orig_in = String::new();
            io::stdin().read_line(&mut orig_in)?;
            let orig_idx: usize = orig_in.trim().parse().unwrap_or(0);
            if orig_idx > 0 && orig_idx <= default_tfs.len() {
                let orig_tf = default_tfs[orig_idx - 1];
                match db::clone_api_config_to_all(conn, orig_tf) {
                    Ok(_) => println!("¡Configuración clonada a todos los timeframes con éxito!"),
                    Err(e) => eprintln!("Error clonando configuración: {}", e),
                }
            } else {
                println!("Selección inválida.");
            }
            continue;
        }

        let idx: usize = api_choice.parse().unwrap_or(0);
        if idx > 0 && idx <= default_tfs.len() {
            let selected_tf = default_tfs[idx - 1];
            manage_timeframe_credentials(conn, selected_tf)?;
        }
    }

    Ok(())
}

fn manage_timeframe_credentials(conn: &Connection, selected_tf: &str) -> Result<(), Box<dyn Error>> {
    loop {
        println!("\n------------------------------------------------");
        println!("  Configuración de Credenciales para: {}", selected_tf);
        println!("------------------------------------------------");
        let configs = db::get_api_configs_by_tf(conn, selected_tf).unwrap_or_default();

        if !configs.is_empty() {
            println!("Credenciales activas:");
            for item in &configs {
                let key_trunc = if item.api_key.len() > 8 {
                    format!("{}...", &item.api_key[..8])
                } else {
                    item.api_key.clone()
                };
                println!(
                    "  - ID: #{} | Key: {} | Apalancamiento: {}X | Testnet (VST): {}",
                    item.id, key_trunc, item.leverage, item.use_testnet
                );
            }
            println!("\n1) Modificar credenciales existentes");
            println!("2) Agregar nuevas credenciales");
            println!("3) Eliminar credenciales por ID");
            println!("4) Volver");
            print!("> ");
            io::stdout().flush()?;

            let mut sub = String::new();
            io::stdin().read_line(&mut sub)?;

            match sub.trim() {
                "1" => {
                    print!("Ingrese ID de cuenta a modificar: ");
                    io::stdout().flush()?;
                    let mut id_in = String::new();
                    io::stdin().read_line(&mut id_in)?;
                    let id_val: u32 = id_in.trim().parse().unwrap_or(0);
                    if let Some(cfg) = configs.iter().find(|c| c.id == id_val) {
                        print!("Nueva API Key (deje vacío para mantener): ");
                        io::stdout().flush()?;
                        let mut new_key = String::new();
                        io::stdin().read_line(&mut new_key)?;
                        let key_to_save = if new_key.trim().is_empty() {
                            cfg.api_key.clone()
                        } else {
                            new_key.trim().to_string()
                        };

                        print!("Nuevo API Secret (deje vacío para mantener): ");
                        io::stdout().flush()?;
                        let mut new_secret = String::new();
                        io::stdin().read_line(&mut new_secret)?;
                        let sec_to_save = if new_secret.trim().is_empty() {
                            cfg.api_secret.clone()
                        } else {
                            new_secret.trim().to_string()
                        };

                        print!("Nuevo Apalancamiento (defecto {}): ", cfg.leverage);
                        io::stdout().flush()?;
                        let mut lev_in = String::new();
                        io::stdin().read_line(&mut lev_in)?;
                        let lev_to_save: u32 = lev_in.trim().parse().unwrap_or(cfg.leverage);

                        print!("¿Usar Testnet VST? (s/N): ");
                        io::stdout().flush()?;
                        let mut testnet_in = String::new();
                        io::stdin().read_line(&mut testnet_in)?;
                        let is_testnet = matches!(testnet_in.trim().to_lowercase().as_str(), "s" | "si" | "y" | "yes");

                        match db::update_api_config_by_id(
                            conn,
                            id_val,
                            &key_to_save,
                            &sec_to_save,
                            lev_to_save,
                            is_testnet,
                        ) {
                            Ok(_) => println!("✅ ¡Credenciales modificadas con éxito!"),
                            Err(e) => eprintln!("❌ Error al actualizar: {}", e),
                        }
                    } else {
                        println!("⚠️ ID #{} no encontrado.", id_val);
                    }
                }
                "2" => {
                    add_api_key_prompt(conn, selected_tf)?;
                }
                "3" => {
                    print!("Ingrese ID de cuenta a eliminar: ");
                    io::stdout().flush()?;
                    let mut id_in = String::new();
                    io::stdin().read_line(&mut id_in)?;
                    let id_val: u32 = id_in.trim().parse().unwrap_or(0);
                    let _ = db::delete_api_config_by_id(conn, id_val);
                    println!("✅ Credenciales eliminadas.");
                }
                "4" => break,
                _ => {}
            }
        } else {
            println!("No hay configuraciones activas. Ingrese una nueva cuenta:");
            add_api_key_prompt(conn, selected_tf)?;
            break;
        }
    }
    Ok(())
}

fn add_api_key_prompt(conn: &Connection, tf: &str) -> Result<(), Box<dyn Error>> {
    println!("\n[Agregar Nuevas Credenciales - BingX ({})]", tf);
    print!("API Key: ");
    io::stdout().flush()?;
    let mut key = String::new();
    io::stdin().read_line(&mut key)?;

    if key.trim().is_empty() {
        println!("⚠️ API Key no puede estar vacía.");
        return Ok(());
    }

    print!("API Secret: ");
    io::stdout().flush()?;
    let mut secret = String::new();
    io::stdin().read_line(&mut secret)?;

    if secret.trim().is_empty() {
        println!("⚠️ API Secret no puede estar vacío.");
        return Ok(());
    }

    print!("Apalancamiento de la cuenta (ej. 10): ");
    io::stdout().flush()?;
    let mut leverage_in = String::new();
    io::stdin().read_line(&mut leverage_in)?;
    let leverage: u32 = leverage_in.trim().parse().unwrap_or(10);

    print!("¿Usar Testnet VST? (s/N): ");
    io::stdout().flush()?;
    let mut testnet_in = String::new();
    io::stdin().read_line(&mut testnet_in)?;
    let use_testnet = matches!(testnet_in.trim().to_lowercase().as_str(), "s" | "si" | "y" | "yes");

    db::add_api_config_by_tf(conn, tf, key.trim(), secret.trim(), leverage, "BingX", use_testnet)?;
    println!("✅ ¡Credenciales configuradas con éxito!");
    Ok(())
}
