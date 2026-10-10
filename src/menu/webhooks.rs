use crate::db::{self, Connection};
use crate::discord;
use reqwest::Client;
use std::error::Error;
use std::io::{self, Write};

pub async fn show_webhooks_menu(client: &Client, conn: &Connection) -> Result<(), Box<dyn Error>> {
    let default_tfs = ["1D", "12H", "8H", "4H", "1H", "15m", "5m", "1m"];

    loop {
        let (capital_percent, leverage, trade_webhook, status_webhook, server_name, status_interval_minutes) =
            db::get_config(conn).unwrap_or((10.0, 10.0, String::new(), String::new(), "Astro-EVO-Server".to_string(), 60));

        let tf_webhooks = db::get_all_timeframe_webhooks(conn).unwrap_or_default();

        println!("\n================================================");
        println!("     CONFIGURAR WEBHOOKS DE DISCORD Y SERVIDOR  ");
        println!("================================================");
        println!("🤖 Nombre del Servidor:       {}", server_name);
        println!("⏱️ Intervalo de Estado:        {} minutos", status_interval_minutes);
        println!("🟢 Webhook de ESTADO Servidor: {}", if status_webhook.is_empty() { "[No configurado]" } else { &status_webhook });
        println!("⚡ Webhook de OPERACIONES (Global): {}", if trade_webhook.is_empty() { "[No configurado]" } else { &trade_webhook });

        println!("\n📊 Webhooks de Operaciones Específicos por Temporalidad/Modelo:");
        for tf in &default_tfs {
            if let Some(wh) = tf_webhooks.get(*tf) {
                println!("  - [{:<3}]: {}", tf, wh);
            } else {
                println!("  - [{:<3}]: [Hereda Global: {}]", tf, if trade_webhook.is_empty() { "Ninguno" } else { "Activo" });
            }
        }
        for (tf, wh) in &tf_webhooks {
            if !default_tfs.contains(&tf.as_str()) {
                println!("  - [{:<3}]: {}", tf, wh);
            }
        }

        println!("\n--- Opciones ---");
        println!("1) Configurar Webhook de Estado, Nombre de Servidor e Intervalo");
        println!("2) Configurar Webhook Global de Operaciones (Fallback)");
        println!("3) Configurar Webhook Específico para una Temporalidad / Modelo");
        println!("4) Eliminar Webhook Específico de una Temporalidad");
        println!("5) Enviar mensaje de prueba a un Webhook");
        println!("6) Volver al menú principal");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                println!("\n[Configurar Estado y Servidor]");
                print!("Nombre del Servidor (actual '{}'): ", server_name);
                io::stdout().flush()?;
                let mut name_in = String::new();
                io::stdin().read_line(&mut name_in)?;
                let new_name = if name_in.trim().is_empty() { server_name.clone() } else { name_in.trim().to_string() };

                print!("Intervalo de reporte en minutos (actual {}): ", status_interval_minutes);
                io::stdout().flush()?;
                let mut int_in = String::new();
                io::stdin().read_line(&mut int_in)?;
                let new_interval: u64 = int_in.trim().parse().unwrap_or(status_interval_minutes);

                print!("Webhook de Discord para ESTADO (deje vacío para mantener): ");
                io::stdout().flush()?;
                let mut status_in = String::new();
                io::stdin().read_line(&mut status_in)?;
                let new_status = if status_in.trim().is_empty() { status_webhook.clone() } else { status_in.trim().to_string() };

                db::save_config(conn, capital_percent, leverage, &trade_webhook, &new_status, &new_name, new_interval)?;
                println!("✅ Configuración de estado y servidor guardada con éxito.");
            }
            "2" => {
                println!("\n[Configurar Webhook Global de Operaciones]");
                print!("URL de Webhook Global (deje vacío para mantener): ");
                io::stdout().flush()?;
                let mut trade_in = String::new();
                io::stdin().read_line(&mut trade_in)?;
                let new_trade = if trade_in.trim().is_empty() { trade_webhook.clone() } else { trade_in.trim().to_string() };

                db::save_config(conn, capital_percent, leverage, &new_trade, &status_webhook, &server_name, status_interval_minutes)?;
                println!("✅ Webhook global de operaciones guardado con éxito.");
            }
            "3" => {
                println!("\n[Configurar Webhook Específico por Temporalidad / Modelo]");
                println!("Seleccione temporalidad:");
                for (idx, tf) in default_tfs.iter().enumerate() {
                    let current = tf_webhooks.get(*tf).map(|s| s.as_str()).unwrap_or("[Hereda Global]");
                    println!("{}) {} -> {}", idx + 1, tf, current);
                }
                println!("9) Otra temporalidad personalizada");
                print!("> ");
                io::stdout().flush()?;

                let mut tf_sel = String::new();
                io::stdin().read_line(&mut tf_sel)?;
                let tf_idx: usize = tf_sel.trim().parse().unwrap_or(0);

                let target_tf = if tf_idx > 0 && tf_idx <= default_tfs.len() {
                    default_tfs[tf_idx - 1].to_string()
                } else if tf_idx == 9 {
                    print!("Ingrese el identificador del Timeframe (ej: 3D, 2H): ");
                    io::stdout().flush()?;
                    let mut custom_tf = String::new();
                    io::stdin().read_line(&mut custom_tf)?;
                    custom_tf.trim().to_uppercase()
                } else {
                    println!("Selección inválida.");
                    continue;
                };

                if target_tf.is_empty() {
                    continue;
                }

                print!("Ingrese URL de Webhook de Discord para [{}]: ", target_tf);
                io::stdout().flush()?;
                let mut wh_in = String::new();
                io::stdin().read_line(&mut wh_in)?;
                let wh_val = wh_in.trim();

                if !wh_val.is_empty() {
                    db::save_timeframe_webhook(conn, &target_tf, wh_val)?;
                    println!("✅ Webhook para temporalidad [{}] guardado con éxito.", target_tf);
                } else {
                    println!("⚠️ URL vacía, no se realizaron cambios.");
                }
            }
            "4" => {
                println!("\n[Eliminar Webhook Específico de Temporalidad]");
                print!("Ingrese la temporalidad a desvincular (ej: 1D, 4H): ");
                io::stdout().flush()?;
                let mut del_tf = String::new();
                io::stdin().read_line(&mut del_tf)?;
                let del_tf = del_tf.trim().to_uppercase();

                if !del_tf.is_empty() {
                    let _ = db::delete_timeframe_webhook(conn, &del_tf);
                    println!("✅ Webhook específico de [{}] eliminado (ahora heredará el global).", del_tf);
                }
            }
            "5" => {
                test_webhook_menu(
                    client,
                    &status_webhook,
                    &trade_webhook,
                    &server_name,
                    &default_tfs,
                    &tf_webhooks,
                )
                .await?;
            }
            "6" => break,
            _ => {}
        }
    }

    Ok(())
}

async fn test_webhook_menu(
    client: &Client,
    status_webhook: &str,
    trade_webhook: &str,
    server_name: &str,
    default_tfs: &[&str],
    tf_webhooks: &std::collections::HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    println!("\n================================================");
    println!("        PROBAR CONEXIÓN DE WEBHOOKS DISCORD     ");
    println!("================================================");
    println!("Seleccione qué Webhook desea probar:");
    println!("1) Webhook de ESTADO Servidor [{}]", if status_webhook.is_empty() { "No configurado" } else { status_webhook });
    println!("2) Webhook GLOBAL de Operaciones [{}]", if trade_webhook.is_empty() { "No configurado" } else { trade_webhook });
    println!("3) Webhook por Temporalidad / Modelo");
    println!("4) Ingresar URL manual personalizada");
    println!("0) Volver");
    print!("> ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    match choice.trim() {
        "1" => {
            if status_webhook.trim().is_empty() {
                println!("⚠️ Webhook de ESTADO no configurado.");
                return Ok(());
            }
            println!("Enviando ping de prueba a Discord (ESTADO)...");
            let msg = format!("🤖 **[ASTRO EVO]** Prueba de Webhook de ESTADO del Servidor `{}` exitosa.", server_name);
            match discord::send_discord_webhook(client, status_webhook, &msg).await {
                Ok(_) => println!("✅ Mensaje de prueba enviado con éxito a Discord."),
                Err(e) => println!("❌ Error al enviar mensaje: {}", e),
            }
        }
        "2" => {
            if trade_webhook.trim().is_empty() {
                println!("⚠️ Webhook GLOBAL de Operaciones no configurado.");
                return Ok(());
            }
            println!("Enviando ping de prueba a Discord (GLOBAL OPERACIONES)...");
            let msg = "⚡ **[ASTRO EVO]** Prueba de Webhook GLOBAL de Operaciones exitosa.";
            match discord::send_discord_webhook(client, trade_webhook, msg).await {
                Ok(_) => println!("✅ Mensaje de prueba enviado con éxito a Discord."),
                Err(e) => println!("❌ Error al enviar mensaje: {}", e),
            }
        }
        "3" => {
            println!("\nSeleccione la Temporalidad a probar:");
            for (idx, tf) in default_tfs.iter().enumerate() {
                let status_str = if let Some(wh) = tf_webhooks.get(*tf) {
                    format!("Específico: {}", wh)
                } else if !trade_webhook.is_empty() {
                    format!("Hereda Global: {}", trade_webhook)
                } else {
                    "Sin Webhook".to_string()
                };
                println!("{}) [{:<3}] -> {}", idx + 1, tf, status_str);
            }

            let mut custom_tfs: Vec<String> = tf_webhooks
                .keys()
                .filter(|k| !default_tfs.contains(&k.as_str()))
                .cloned()
                .collect();
            custom_tfs.sort();

            for (idx, c_tf) in custom_tfs.iter().enumerate() {
                let wh = tf_webhooks.get(c_tf).unwrap();
                println!("{}) [{:<3}] -> Específico: {}", default_tfs.len() + idx + 1, c_tf, wh);
            }

            print!("> ");
            io::stdout().flush()?;

            let mut tf_sel = String::new();
            io::stdin().read_line(&mut tf_sel)?;
            let tf_idx: usize = tf_sel.trim().parse().unwrap_or(0);

            let selected_tf = if tf_idx > 0 && tf_idx <= default_tfs.len() {
                Some(default_tfs[tf_idx - 1].to_string())
            } else if tf_idx > default_tfs.len() && tf_idx <= default_tfs.len() + custom_tfs.len() {
                Some(custom_tfs[tf_idx - default_tfs.len() - 1].clone())
            } else {
                None
            };

            if let Some(tf) = selected_tf {
                let (target_url, is_specific) = if let Some(wh) = tf_webhooks.get(&tf) {
                    (wh.as_str(), true)
                } else if !trade_webhook.is_empty() {
                    (trade_webhook, false)
                } else {
                    ("", false)
                };

                if target_url.is_empty() {
                    println!("⚠️ No hay Webhook disponible para la temporalidad [{}] (ni específico ni global).", tf);
                } else {
                    let origin_desc = if is_specific { "Webhook Específico" } else { "Webhook Global (Heredado)" };
                    println!("Enviando ping de prueba para [{}] ({}) a Discord...", tf, origin_desc);
                    let msg = format!(
                        "📊 **[ASTRO EVO]** Prueba de Webhook para Temporalidad `[{}]` exitosa.\nℹ️ **Configuración:** {}",
                        tf, origin_desc
                    );
                    match discord::send_discord_webhook(client, target_url, &msg).await {
                        Ok(_) => println!("✅ Mensaje de prueba para [{}] enviado con éxito a Discord.", tf),
                        Err(e) => println!("❌ Error al enviar mensaje: {}", e),
                    }
                }
            } else {
                println!("Selección inválida.");
            }
        }
        "4" => {
            print!("Ingrese URL de Discord Webhook a probar: ");
            io::stdout().flush()?;
            let mut manual_url = String::new();
            io::stdin().read_line(&mut manual_url)?;
            let manual_url = manual_url.trim();

            if manual_url.is_empty() {
                println!("⚠️ URL vacía.");
                return Ok(());
            }

            println!("Enviando ping de prueba a Discord...");
            let msg = "🔔 **[ASTRO EVO]** Mensaje de prueba de conexión de Webhook manual exitoso.";
            match discord::send_discord_webhook(client, manual_url, msg).await {
                Ok(_) => println!("✅ Mensaje de prueba enviado con éxito a Discord."),
                Err(e) => println!("❌ Error al enviar mensaje: {}", e),
            }
        }
        _ => {}
    }

    Ok(())
}
