use crate::db::{self, Connection};
use std::error::Error;
use std::io::{self, Write};

pub fn show_models_management_menu(conn: &Connection) -> Result<(), Box<dyn Error>> {
    loop {
        println!("\n================================================");
        println!("       GESTIÓN Y BORRADO DE MODELOS (DB)        ");
        println!("================================================");
        println!("1) Listar todos los modelos guardados (cnn1d_models)");
        println!("2) Borrar UN modelo específico por ID");
        println!("3) Borrar TODOS los modelos de una Temporalidad");
        println!("4) Borrar TODOS los modelos de la Base de Datos");
        println!("5) Listar Historial de Niveles de Inteligencia (intelligence_history)");
        println!("6) Borrar TODO el Historial de Inteligencia");
        println!("7) Volver");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                list_models(conn)?;
            }
            "2" => {
                list_models(conn)?;
                print!("\nIngrese el ID del modelo a borrar (o presione ENTER para cancelar): ");
                io::stdout().flush()?;
                let mut id_in = String::new();
                io::stdin().read_line(&mut id_in)?;
                let id_str = id_in.trim();
                if !id_str.is_empty() {
                    if let Ok(id) = id_str.parse::<i64>() {
                        match db::delete_model_by_id(conn, id) {
                            Ok(count) => {
                                if count > 0 {
                                    println!("✅ Modelo ID #{} eliminado exitosamente.", id);
                                } else {
                                    println!("⚠️ No se encontró ningún modelo con ID #{}.", id);
                                }
                            }
                            Err(e) => eprintln!("❌ Error al borrar modelo: {}", e),
                         }
                    } else {
                        println!("ID inválido.");
                    }
                }
            }
            "3" => {
                print!("\nIngrese la Temporalidad cuyos modelos desea borrar (ej: 1D, 4H, 1H): ");
                io::stdout().flush()?;
                let mut tf_in = String::new();
                io::stdin().read_line(&mut tf_in)?;
                let tf = tf_in.trim().to_uppercase();
                if !tf.is_empty() {
                    print!("¿Está seguro de borrar TODOS los modelos de {}? (s/N): ", tf);
                    io::stdout().flush()?;
                    let mut conf = String::new();
                    io::stdin().read_line(&mut conf)?;
                    if conf.trim().eq_ignore_ascii_case("s") || conf.trim().eq_ignore_ascii_case("si") {
                        match db::delete_models_by_timeframe(conn, &tf) {
                            Ok(count) => println!("✅ Se eliminaron {} modelos de la temporalidad {}.", count, tf),
                            Err(e) => eprintln!("❌ Error al borrar modelos de {}: {}", tf, e),
                        }
                    } else {
                        println!("Operación cancelada.");
                    }
                }
            }
            "4" => {
                print!("\n⚠️ ¿Está seguro de que desea borrar ABSOLUTAMENTE TODOS los modelos de models.db? (s/N): ");
                io::stdout().flush()?;
                let mut conf = String::new();
                io::stdin().read_line(&mut conf)?;
                if conf.trim().eq_ignore_ascii_case("s") || conf.trim().eq_ignore_ascii_case("si") {
                    match db::delete_all_models(conn) {
                        Ok(count) => println!("✅ Base de datos limpiada: {} modelos eliminados en total.", count),
                        Err(e) => eprintln!("❌ Error al limpiar base de datos de modelos: {}", e),
                    }
                } else {
                    println!("Operación cancelada.");
                }
            }
            "5" => {
                list_intelligence_history(conn)?;
            }
            "6" => {
                print!("\n⚠️ ¿Está seguro de borrar TODO el historial acumulado de inteligencia? (s/N): ");
                io::stdout().flush()?;
                let mut conf = String::new();
                io::stdin().read_line(&mut conf)?;
                if conf.trim().eq_ignore_ascii_case("s") || conf.trim().eq_ignore_ascii_case("si") {
                    match db::delete_all_intelligence_records(conn) {
                        Ok(count) => println!("✅ Historial de inteligencia limpiado: {} registros eliminados.", count),
                        Err(e) => eprintln!("❌ Error al limpiar historial de inteligencia: {}", e),
                    }
                }
            }
            _ => break,
        }
    }
    Ok(())
}

fn list_models(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let models = db::get_all_stored_models_summary(conn)?;
    if models.is_empty() {
        println!("\nℹ️ No hay modelos guardados en la tabla cnn1d_models.");
        return Ok(());
    }

    println!("\n📜 [MODELOS GUARDADOS EN MODELS.DB - TOTAL: {}]", models.len());
    println!("───────────────────────────────────────────────────────────────────────────────────────────────────");
    println!(" ID   | TF   | Ventana W | Kernel K | Épocas | H   | Lev. | Cap%  | Weight Decay | L1 Lambda");
    println!("───────────────────────────────────────────────────────────────────────────────────────────────────");
    for m in &models {
        println!(
            " {:<4} | {:<4} | {:<9} | {:<8} | {:<6} | {:<3} | {:<4.0}X | {:<5.1}% | {:<12.5} | {:<9.6}",
            m.id, m.timeframe, m.window_size, m.kernel_size, m.epoch, m.horizon, m.leverage, m.capital_percent, m.weight_decay, m.l1_lambda
        );
    }
    println!("───────────────────────────────────────────────────────────────────────────────────────────────────");
    println!("💡 Nota: El motor de Trading en Vivo siempre carga el modelo más reciente (ID más alto) de cada TF.");
    Ok(())
}

fn list_intelligence_history(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let summaries = db::get_all_stored_intelligence_summaries(conn)?;
    if summaries.is_empty() {
        println!("\nℹ️ No hay registros guardados en intelligence_history.");
        return Ok(());
    }

    println!("\n📜 [NIVELES DE INTELIGENCIA HISTÓRICOS GUARDADOS — TOTAL: {}]", summaries.len());
    println!("──────────────────────────────────────────────────────────────────────────────────────────");
    println!(" TF   | Nivel | Arquitectura (L➔2L➔2L) | Ventana W | Kernel K | H   | Épocas | Lev. | Cap%");
    println!("──────────────────────────────────────────────────────────────────────────────────────────");
    for s in &summaries {
        let arch = format!("{} ➔ {} ➔ {}", s.level, s.level * 2, s.level * 2);
        println!(
            " {:<4} | L={:<4} | {:<22} | {:<9} | {:<8} | {:<3} | {:<6} | {:<4.0}X | {:<5.1}%",
            s.timeframe, s.level, arch, s.window_size, s.kernel_size, s.horizon, s.epoch, s.leverage, s.capital_percent
        );
    }
    println!("──────────────────────────────────────────────────────────────────────────────────────────");
    Ok(())
}

