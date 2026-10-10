use chrono::Utc;
use reqwest::Client;

pub async fn send_discord_webhook(client: &Client, url: &str, content: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("URL de webhook vacía".to_string());
    }
    let body = serde_json::json!({
        "content": format!("{}\n", content)
    });
    match client.post(url).json(&body).send().await {
        Ok(res) => {
            if res.status().is_success() {
                Ok(())
            } else {
                let status = res.status();
                let err_text = res.text().await.unwrap_or_default();
                Err(format!("Error HTTP Discord ({}): {}", status, err_text))
            }
        }
        Err(e) => Err(format!("Error de conexión con Discord: {}", e)),
    }
}

pub async fn send_trade_alert_webhook(
    client: &Client,
    url: &str,
    tf: &str,
    action: &str,
    symbol: &str,
    price: f64,
    order_qty: f64,
    notional: f64,
    leverage: f64,
    acc_id: u32,
    details: &str,
) {
    if url.trim().is_empty() {
        return;
    }
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let emoji = match action {
        "LONG" => "🚀",
        "SHORT" => "🔻",
        "FLAT_CLOSE_LONG" | "FLAT_CLOSE_SHORT" => "🔄",
        _ => "⚡",
    };

    let title = match action {
        "LONG" => format!("{} [ASTRO EVO] Orden LONG Ejecutada", emoji),
        "SHORT" => format!("{} [ASTRO EVO] Orden SHORT Ejecutada", emoji),
        "FLAT_CLOSE_LONG" => format!("{} [ASTRO EVO] Cierre LONG por Señal FLAT", emoji),
        "FLAT_CLOSE_SHORT" => format!("{} [ASTRO EVO] Cierre SHORT por Señal FLAT", emoji),
        _ => format!("{} [ASTRO EVO] Alerta de Operación", emoji),
    };

    let msg = format!(
        "**{}**\n📅 **Fecha/Hora:** `{}`\n⏱️ **Temporalidad / Modelo:** `{}`\n🪙 **Símbolo:** `{}`\n💵 **Precio:** `${:.2}` | **Apalancamiento:** `{:.0}X`\n📊 **Cantidad:** `{:.4} BTC` (~`${:.2}` Notional)\n👤 **Cuenta ID:** `#{}`\nℹ️ **Detalle:** {}",
        title, now, tf, symbol, price, leverage, order_qty, notional, acc_id, details
    );

    let body = serde_json::json!({
        "content": msg
    });
    let _ = client.post(url).json(&body).send().await;
}

pub async fn send_server_status_webhook(client: &Client, url: &str, server_name: &str, info: &str) {
    if url.trim().is_empty() {
        return;
    }
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let msg = format!(
        "🤖 **[ESTADO DEL SERVIDOR: {}]**\n📅 **Fecha/Hora:** `{}`\n🟢 **Estado:** Activo y Operando\n{}",
        server_name, now, info
    );
    let body = serde_json::json!({
        "content": msg
    });
    let _ = client.post(url).json(&body).send().await;
}
