use crate::db::Kline;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BinanceKlineTuple(
    i64,    // Open time
    String, // Open
    String, // High
    String, // Low
    String, // Close
    String, // Volume
    i64,    // Close time
    String, // Quote asset volume
    i64,    // Number of trades
    String, // Taker buy base asset volume
    String, // Taker buy quote asset volume
    String, // Ignore
);

pub async fn download_klines(
    client: &Client,
    symbol: &str,
    interval: &str,
    start_time: Option<i64>,
    limit: u32,
) -> Result<Vec<Kline>, Box<dyn Error>> {
    let base_urls = [
        "https://api.binance.com",
        "https://api1.binance.com",
        "https://api2.binance.com",
        "https://api3.binance.com",
        "https://api-gcp.binance.com",
    ];

    let mut last_err = None;

    for &base_url in &base_urls {
        let mut url = format!(
            "{}/api/v3/klines?symbol={}&interval={}&limit={}",
            base_url, symbol, interval, limit
        );
        
        if let Some(st) = start_time {
            url = format!("{}&startTime={}", url, st);
        }

        match client.get(&url).send().await {
            Ok(res) => {
                match res.text().await {
                    Ok(text_res) => {
                        match serde_json::from_str::<Vec<BinanceKlineTuple>>(&text_res) {
                            Ok(data) => {
                                let now_ms = chrono::Utc::now().timestamp_millis();
                                let klines = data
                                    .into_iter()
                                    .filter(|t| t.6 <= now_ms)
                                    .map(|t| Kline {
                                        timestamp: t.0,
                                        open: t.1.parse().unwrap_or(0.0),
                                        high: t.2.parse().unwrap_or(0.0),
                                        low: t.3.parse().unwrap_or(0.0),
                                        close: t.4.parse().unwrap_or(0.0),
                                        volume: t.5.parse().unwrap_or(0.0),
                                    })
                                    .collect();
                                return Ok(klines);
                            }
                            Err(e) => {
                                let err_msg = format!("Error deserializando respuesta de {}: {}. Respuesta: {}", base_url, e, text_res);
                                last_err = Some(err_msg);
                            }
                        }
                    }
                    Err(e) => {
                        last_err = Some(format!("Error leyendo texto de respuesta de {}: {}", base_url, e));
                    }
                }
            }
            Err(e) => {
                last_err = Some(format!("Error de conexión con {}: {}", base_url, e));
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Todos los endpoints de Binance fallaron al descargar klines. Último error: {:?}", last_err),
    )))
}

fn calculate_signature(secret: &str, query: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(query.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

fn normalize_symbol(symbol: &str) -> String {
    if symbol.contains('-') {
        symbol.to_string()
    } else {
        symbol.replace("USDT", "-USDT")
    }
}

pub async fn test_api_connection(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    use_testnet: bool,
) -> Result<(), Box<dyn Error>> {
    let base_url = if use_testnet {
        "https://open-api-vst.bingx.com"
    } else {
        "https://open-api.bingx.com"
    };
    let timestamp = chrono::Utc::now().timestamp_millis();
    let query = format!("timestamp={}", timestamp);
    let signature = calculate_signature(api_secret, &query);
    let url = format!("{}/openApi/swap/v2/user/balance?{}&signature={}", base_url, query, signature);

    let res = client
        .get(&url)
        .header("X-BX-APIKEY", api_key)
        .send()
        .await?;

    if res.status().is_success() {
        let text = res.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(code) = json.get("code").and_then(|c| c.as_i64()) {
            if code == 0 {
                Ok(())
            } else {
                let msg = json.get("msg").and_then(|m| m.as_str()).unwrap_or("Error desconocido");
                Err(format!("Error BingX (código {}): {}", code, msg).into())
            }
        } else {
            Err("Formato de respuesta de BingX no válido".into())
        }
    } else {
        let err_text = res.text().await?;
        Err(format!("Error HTTP en BingX: {}", err_text).into())
    }
}

pub async fn set_leverage(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    leverage: u32,
    position_side: &str,
    use_testnet: bool,
) -> Result<(), Box<dyn Error>> {
    let base_url = if use_testnet {
        "https://open-api-vst.bingx.com"
    } else {
        "https://open-api.bingx.com"
    };
    let normalized_symbol = normalize_symbol(symbol);
    let timestamp = chrono::Utc::now().timestamp_millis();
    let query = format!(
        "leverage={}&side={}&symbol={}&timestamp={}",
        leverage, position_side, normalized_symbol, timestamp
    );
    let signature = calculate_signature(api_secret, &query);
    let url = format!("{}/openApi/swap/v2/trade/leverage?{}&signature={}", base_url, query, signature);

    let res = client
        .post(&url)
        .header("X-BX-APIKEY", api_key)
        .send()
        .await?;

    if res.status().is_success() {
        let text = res.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(code) = json.get("code").and_then(|c| c.as_i64()) {
            if code == 0 {
                Ok(())
            } else {
                let msg = json.get("msg").and_then(|m| m.as_str()).unwrap_or("Error");
                Err(format!("Error BingX (código {}): {}", code, msg).into())
            }
        } else {
            Err("Respuesta no válida".into())
        }
    } else {
        let err_text = res.text().await?;
        Err(format!("Error HTTP: {}", err_text).into())
    }
}

pub async fn get_ticker_price(
    client: &Client,
    symbol: &str,
    use_testnet: bool,
) -> Result<f64, Box<dyn Error>> {
    let base_url = if use_testnet {
        "https://open-api-vst.bingx.com"
    } else {
        "https://open-api.bingx.com"
    };
    let normalized_symbol = normalize_symbol(symbol);
    let url = format!("{}/openApi/swap/v2/quote/price?symbol={}", base_url, normalized_symbol);
    
    let res = client.get(&url).send().await?;
    if res.status().is_success() {
        let text = res.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(price_str) = json.get("data").and_then(|d| d.get("price")).and_then(|p| p.as_str()) {
            let price: f64 = price_str.parse()?;
            Ok(price)
        } else {
            Err("Formato de precio no encontrado en BingX".into())
        }
    } else {
        let err_text = res.text().await?;
        Err(format!("Error obteniendo precio: {}", err_text).into())
    }
}

pub async fn open_market_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    side: &str,
    position_side: &str,
    quantity: f64,
    client_order_id: Option<&str>,
    use_testnet: bool,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let base_url = if use_testnet {
        "https://open-api-vst.bingx.com"
    } else {
        "https://open-api.bingx.com"
    };
    let normalized_symbol = normalize_symbol(symbol);
    let timestamp = chrono::Utc::now().timestamp_millis();
    let qty_str = format!("{:.4}", quantity);
    let qty_val: f64 = qty_str.parse().unwrap_or(0.0);
    if qty_val <= 0.0 {
        return Err("La cantidad calculada es menor que el mínimo permitido (0.0001 BTC).".into());
    }
    
    let mut query = format!(
        "positionSide={}&quantity={}&side={}&symbol={}&timestamp={}&type=MARKET",
        position_side, qty_str, side, normalized_symbol, timestamp
    );
    if let Some(cid) = client_order_id {
        query = format!("clientOrderID={}&{}", cid, query);
    }
    
    let signature = calculate_signature(api_secret, &query);
    let url = format!("{}/openApi/swap/v2/trade/order?{}&signature={}", base_url, query, signature);

    let res = client
        .post(&url)
        .header("X-BX-APIKEY", api_key)
        .send()
        .await?;

    let status = res.status();
    let text = res.text().await?;
    if status.is_success() {
        let json: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(code) = json.get("code").and_then(|c| c.as_i64()) {
            if code == 0 {
                Ok(json.get("data").cloned().unwrap_or(json))
            } else {
                let msg = json.get("msg").and_then(|m| m.as_str()).unwrap_or("Error");
                Err(format!("Error BingX al colocar orden (código {}): {}", code, msg).into())
            }
        } else {
            Err("Respuesta no válida de BingX".into())
        }
    } else {
        Err(format!("Error HTTP: {}", text).into())
    }
}

pub async fn get_open_positions(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    use_testnet: bool,
) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    let base_url = if use_testnet {
        "https://open-api-vst.bingx.com"
    } else {
        "https://open-api.bingx.com"
    };
    let normalized_symbol = normalize_symbol(symbol);
    let timestamp = chrono::Utc::now().timestamp_millis();
    let query = format!("symbol={}&timestamp={}", normalized_symbol, timestamp);
    let signature = calculate_signature(api_secret, &query);
    let url = format!("{}/openApi/swap/v2/user/positions?{}&signature={}", base_url, query, signature);

    let res = client
        .get(&url)
        .header("X-BX-APIKEY", api_key)
        .send()
        .await?;

    if res.status().is_success() {
        let text = res.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(code) = json.get("code").and_then(|c| c.as_i64()) {
            if code == 0 {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    return Ok(data.clone());
                }
                Ok(vec![])
            } else {
                let msg = json.get("msg").and_then(|m| m.as_str()).unwrap_or("Error");
                Err(format!("Error BingX al consultar posiciones (código {}): {}", code, msg).into())
            }
        } else {
            Err("Respuesta no válida de BingX".into())
        }
    } else {
        let err_text = res.text().await?;
        Err(format!("Error HTTP al obtener posiciones: {}", err_text).into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionDirection {
    Long,
    Short,
    Flat,
}

pub fn parse_bingx_position(pos: &serde_json::Value) -> (PositionDirection, f64) {
    let amt_str = pos.get("positionAmt")
        .or_else(|| pos.get("availableAmt"))
        .or_else(|| pos.get("volume"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.to_string())
            } else if let Some(n) = v.as_f64() {
                Some(n.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "0".to_string());

    let amt: f64 = amt_str.parse().unwrap_or(0.0);
    if amt.abs() < 1e-6 {
        return (PositionDirection::Flat, 0.0);
    }

    let pos_side = pos.get("positionSide").and_then(|s| s.as_str()).unwrap_or("").to_uppercase();
    let side = pos.get("side").and_then(|s| s.as_str()).unwrap_or("").to_uppercase();

    if pos_side == "SHORT" || side == "SELL" || amt < 0.0 {
        (PositionDirection::Short, amt.abs())
    } else if pos_side == "LONG" || side == "BUY" || amt > 0.0 {
        (PositionDirection::Long, amt.abs())
    } else {
        (PositionDirection::Flat, 0.0)
    }
}

#[derive(Debug, Clone)]
pub struct BingXAccountInfo {
    pub wallet_balance: f64,
    pub available_margin: f64,
    pub user_id: String,
}

pub async fn get_account_details(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    use_testnet: bool,
) -> Result<BingXAccountInfo, Box<dyn Error>> {
    let base_url = if use_testnet {
        "https://open-api-vst.bingx.com"
    } else {
        "https://open-api.bingx.com"
    };
    let timestamp = chrono::Utc::now().timestamp_millis();
    let query = format!("timestamp={}", timestamp);
    let signature = calculate_signature(api_secret, &query);
    let url = format!("{}/openApi/swap/v2/user/balance?{}&signature={}", base_url, query, signature);

    let res = client
        .get(&url)
        .header("X-BX-APIKEY", api_key)
        .send()
        .await?;

    if res.status().is_success() {
        let text = res.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(data) = json.get("data").and_then(|d| d.get("balance")) {
            let wallet_balance: f64 = data.get("balance")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .parse()
                .unwrap_or(0.0);
            let available_margin: f64 = data.get("availableMargin")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .parse()
                .unwrap_or(0.0);
            let user_id = data.get("userId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(BingXAccountInfo {
                wallet_balance,
                available_margin,
                user_id,
            })
        } else {
            Err("No se pudo obtener la información de balance/userId en la respuesta de BingX".into())
        }
    } else {
        let err_text = res.text().await?;
        Err(format!("Error HTTP al obtener balances: {}", err_text).into())
    }
}
