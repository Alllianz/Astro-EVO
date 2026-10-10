pub use rusqlite::{params, Connection, Result};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kline {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

pub fn init_candles_db() -> Result<Connection> {
    let db_path = "candles.db";

    let conn = Connection::open(db_path)?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    conn.busy_timeout(std::time::Duration::from_secs(60))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS klines (
            timeframe TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            open REAL NOT NULL,
            high REAL NOT NULL,
            low REAL NOT NULL,
            close REAL NOT NULL,
            volume REAL NOT NULL,
            PRIMARY KEY (timeframe, timestamp)
        )",
        [],
    )?;
    Ok(conn)
}

pub fn init_models_db() -> Result<Connection> {
    let conn = Connection::open("models.db")?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    conn.busy_timeout(std::time::Duration::from_secs(60))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            capital_percent REAL NOT NULL,
            leverage REAL NOT NULL,
            trade_webhook TEXT NOT NULL DEFAULT '',
            status_webhook TEXT NOT NULL DEFAULT '',
            server_name TEXT NOT NULL DEFAULT 'Astro-EVO-Server',
            status_interval_minutes INTEGER NOT NULL DEFAULT 60
        )",
        [],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO config (id, capital_percent, leverage, trade_webhook, status_webhook, server_name, status_interval_minutes) 
         VALUES (1, 10.0, 10.0, '', '', 'Astro-EVO-Server', 60)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS api_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timeframe TEXT NOT NULL,
            api_key TEXT NOT NULL,
            api_secret TEXT NOT NULL,
            leverage INTEGER NOT NULL DEFAULT 10,
            exchange TEXT NOT NULL,
            use_testnet INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS timeframe_webhooks (
            timeframe TEXT PRIMARY KEY,
            trade_webhook TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS cnn1d_models (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timeframe TEXT NOT NULL,
            window_size INTEGER NOT NULL,
            kernel_size INTEGER NOT NULL,
            epoch INTEGER NOT NULL,
            model_json TEXT NOT NULL,
            thr_long REAL DEFAULT 0.40,
            thr_short REAL DEFAULT 0.40,
            is_online INTEGER NOT NULL DEFAULT 0,
            leverage REAL DEFAULT 10.0,
            capital_percent REAL DEFAULT 10.0,
            weight_decay REAL DEFAULT 0.001,
            l1_lambda REAL DEFAULT 0.0002,
            dropout_rate REAL DEFAULT 0.0,
            horizon INTEGER DEFAULT 10
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS intelligence_history (
            timeframe TEXT NOT NULL,
            level INTEGER NOT NULL,
            window_size INTEGER NOT NULL,
            kernel_size INTEGER NOT NULL,
            horizon INTEGER NOT NULL,
            epoch INTEGER NOT NULL,
            leverage REAL NOT NULL,
            capital_percent REAL NOT NULL,
            weight_decay REAL NOT NULL,
            l1_lambda REAL NOT NULL,
            metrics_nom_json TEXT NOT NULL,
            metrics_pct_json TEXT NOT NULL,
            eq_curve_nom_json TEXT NOT NULL,
            eq_curve_pct_json TEXT NOT NULL,
            model_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (timeframe, level)
        )",
        [],
    )?;

    Ok(conn)
}


// --- Métodos de configuración global ---

pub fn get_config(conn: &Connection) -> Result<(f64, f64, String, String, String, u64)> {
    let mut stmt = conn.prepare("SELECT capital_percent, leverage, trade_webhook, status_webhook, server_name, status_interval_minutes FROM config WHERE id = 1")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let cp: f64 = row.get(0)?;
        let lev: f64 = row.get(1)?;
        let trade: String = row.get(2)?;
        let status: String = row.get(3)?;
        let server: String = row.get(4).unwrap_or_else(|_| "Astro-EVO-Server".to_string());
        let interval_i64: i64 = row.get(5).unwrap_or(60);
        Ok((cp, lev, trade, status, server, interval_i64.max(0) as u64))
    } else {
        Ok((10.0, 10.0, String::new(), String::new(), "Astro-EVO-Server".to_string(), 60))
    }
}

pub fn save_config(conn: &Connection, cp: f64, lev: f64, trade: &str, status: &str, server_name: &str, status_interval: u64) -> Result<()> {
    let interval_i64 = status_interval as i64;
    conn.execute(
        "INSERT OR REPLACE INTO config (id, capital_percent, leverage, trade_webhook, status_webhook, server_name, status_interval_minutes) 
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
        params![cp, lev, trade, status, server_name, interval_i64],
    )?;
    Ok(())
}

// --- Métodos de Webhooks por Temporalidad / Modelo ---

pub fn get_timeframe_webhook(conn: &Connection, tf: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT trade_webhook FROM timeframe_webhooks WHERE timeframe = ?1")?;
    let mut rows = stmt.query(params![tf])?;
    if let Some(row) = rows.next()? {
        let wh: String = row.get(0)?;
        if !wh.trim().is_empty() {
            return Ok(Some(wh));
        }
    }
    Ok(None)
}

pub fn get_all_timeframe_webhooks(conn: &Connection) -> Result<std::collections::HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT timeframe, trade_webhook FROM timeframe_webhooks")?;
    let rows = stmt.query_map([], |row| {
        let tf: String = row.get(0)?;
        let wh: String = row.get(1)?;
        Ok((tf, wh))
    })?;
    let mut map = std::collections::HashMap::new();
    for r in rows {
        let (tf, wh) = r?;
        map.insert(tf, wh);
    }
    Ok(map)
}

pub fn save_timeframe_webhook(conn: &Connection, tf: &str, webhook: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO timeframe_webhooks (timeframe, trade_webhook) VALUES (?1, ?2)",
        params![tf, webhook],
    )?;
    Ok(())
}

pub fn delete_timeframe_webhook(conn: &Connection, tf: &str) -> Result<()> {
    conn.execute("DELETE FROM timeframe_webhooks WHERE timeframe = ?1", params![tf])?;
    Ok(())
}

// --- Métodos de API Keys por temporalidad ---

pub fn get_api_configs_by_tf(conn: &Connection, tf: &str) -> Result<Vec<crate::config::ApiConfig>> {
    let mut stmt = conn.prepare("SELECT id, timeframe, api_key, api_secret, leverage, exchange, use_testnet FROM api_config WHERE timeframe = ?1")?;
    let rows = stmt.query_map(params![tf], |row| {
        let id: u32 = row.get(0)?;
        let timeframe: String = row.get(1)?;
        let api_key: String = row.get(2)?;
        let api_secret: String = row.get(3)?;
        let leverage: u32 = row.get(4)?;
        let exchange: String = row.get(5)?;
        let use_testnet_int: i32 = row.get(6)?;
        Ok(crate::config::ApiConfig {
            id,
            timeframe,
            api_key,
            api_secret,
            leverage,
            exchange,
            use_testnet: use_testnet_int == 1,
        })
    })?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn get_all_api_configs(conn: &Connection) -> Result<Vec<crate::config::ApiConfig>> {
    let mut stmt = conn.prepare("SELECT id, timeframe, api_key, api_secret, leverage, exchange, use_testnet FROM api_config")?;
    let rows = stmt.query_map([], |row| {
        let id: u32 = row.get(0)?;
        let timeframe: String = row.get(1)?;
        let api_key: String = row.get(2)?;
        let api_secret: String = row.get(3)?;
        let leverage: u32 = row.get(4)?;
        let exchange: String = row.get(5)?;
        let use_testnet_int: i32 = row.get(6)?;
        Ok(crate::config::ApiConfig {
            id,
            timeframe,
            api_key,
            api_secret,
            leverage,
            exchange,
            use_testnet: use_testnet_int == 1,
        })
    })?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn add_api_config_by_tf(
    conn: &Connection,
    tf: &str,
    api_key: &str,
    api_secret: &str,
    leverage: u32,
    exchange: &str,
    use_testnet: bool
) -> Result<()> {
    conn.execute(
        "INSERT INTO api_config (timeframe, api_key, api_secret, leverage, exchange, use_testnet) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![tf, api_key, api_secret, leverage, exchange, if use_testnet { 1 } else { 0 }],
    )?;
    Ok(())
}

pub fn update_api_config_by_id(
    conn: &Connection,
    id: u32,
    api_key: &str,
    api_secret: &str,
    leverage: u32,
    use_testnet: bool
) -> Result<()> {
    conn.execute(
        "UPDATE api_config SET api_key = ?1, api_secret = ?2, leverage = ?3, use_testnet = ?4 WHERE id = ?5",
        params![api_key, api_secret, leverage, if use_testnet { 1 } else { 0 }, id],
    )?;
    Ok(())
}

pub fn delete_api_config_by_id(conn: &Connection, id: u32) -> Result<()> {
    conn.execute("DELETE FROM api_config WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn clone_api_config_to_all(conn: &Connection, source_tf: &str) -> Result<()> {
    let source_configs = get_api_configs_by_tf(conn, source_tf)?;
    let default_tfs = ["Cuenta Principal", "1D", "12H", "8H", "4H", "1H", "15m", "5m", "1m"];
    for tf in &default_tfs {
        if *tf == source_tf {
            continue;
        }
        conn.execute("DELETE FROM api_config WHERE timeframe = ?1", params![tf])?;
        for config in &source_configs {
            add_api_config_by_tf(conn, tf, &config.api_key, &config.api_secret, config.leverage, &config.exchange, config.use_testnet)?;
        }
    }
    Ok(())
}

// --- Métodos de velas de candles.db ---

pub fn count_db(conn: &Connection) -> Result<()> {
    for tf in ["1D", "12H", "8H", "4H", "1H", "15m", "5m", "1m"] {
        let mut stmt = conn.prepare("SELECT COUNT(*), MIN(timestamp), MAX(timestamp) FROM klines WHERE timeframe = ?1")?;
        let mut rows = stmt.query(params![tf])?;
        if let Some(row) = rows.next()? {
            let count: i64 = row.get(0)?;
            if count > 0 {
                let min_ts: i64 = row.get(1)?;
                let max_ts: i64 = row.get(2)?;
                let min_date = Utc.timestamp_millis_opt(min_ts).unwrap();
                let max_date = Utc.timestamp_millis_opt(max_ts).unwrap();
                println!("- {}: {} registros (Desde {} hasta {})", tf, count, min_date.format("%Y-%m-%d %H:%M:%S"), max_date.format("%Y-%m-%d %H:%M:%S"));
            } else {
                println!("- {}: 0 registros", tf);
            }
        } else {
            println!("- {}: 0 registros", tf);
        }
    }
    Ok(())
}

pub fn get_latest_timestamp(conn: &Connection, tf: &str) -> Result<Option<i64>> {
    let mut stmt = conn.prepare("SELECT MAX(timestamp) FROM klines WHERE timeframe = ?1")?;
    let mut rows = stmt.query(params![tf])?;
    if let Some(row) = rows.next()? {
        let ts: rusqlite::types::ValueRef<'_> = row.get_ref(0)?;
        if ts != rusqlite::types::ValueRef::Null {
            return Ok(Some(row.get(0)?));
        }
    }
    Ok(None)
}

pub fn insert_klines(conn: &mut Connection, tf: &str, klines: &[Kline]) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO klines (timeframe, timestamp, open, high, low, close, volume)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;
        for k in klines {
            stmt.execute(params![
                tf,
                k.timestamp,
                k.open,
                k.high,
                k.low,
                k.close,
                k.volume,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn tf_to_duration_millis(tf: &str) -> i64 {
    match tf {
        "1m" => 60 * 1000,
        "3m" => 3 * 60 * 1000,
        "5m" => 5 * 60 * 1000,
        "15m" => 15 * 60 * 1000,
        "30m" => 30 * 60 * 1000,
        "1h" | "1H" => 60 * 60 * 1000,
        "2h" | "2H" => 2 * 60 * 60 * 1000,
        "4h" | "4H" => 4 * 60 * 60 * 1000,
        "6h" | "6H" => 6 * 60 * 60 * 1000,
        "8h" | "8H" => 8 * 60 * 60 * 1000,
        "12h" | "12H" => 12 * 60 * 60 * 1000,
        "1d" | "1D" => 24 * 60 * 60 * 1000,
        _ => 24 * 60 * 60 * 1000,
    }
}

pub fn get_all_klines(conn: &Connection, tf: &str) -> Result<Vec<Kline>> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut stmt = conn.prepare(
        "SELECT timestamp, open, high, low, close, volume FROM klines WHERE timeframe = ?1 AND timestamp <= ?2 ORDER BY timestamp ASC"
    )?;
    let klines = stmt.query_map(params![tf, now_ms], |row| {
        Ok(Kline {
            timestamp: row.get(0)?,
            open: row.get(1)?,
            high: row.get(2)?,
            low: row.get(3)?,
            close: row.get(4)?,
            volume: row.get(5)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    Ok(klines)
}

// --- Métodos de guardado y carga de modelos ---

pub fn save_cnn1d_model_full(
    conn: &Connection,
    timeframe: &str,
    window_size: usize,
    kernel_size: usize,
    epoch: usize,
    model: &crate::cnn1d::Cnn1DModel,
    thr_long: f32,
    thr_short: f32,
    is_online: bool,
    leverage: f64,
    capital_percent: f64,
    weight_decay: f64,
    l1_lambda: f64,
    dropout_rate: f64,
    horizon: usize,
) -> Result<i64> {
    let model_json = serde_json::to_string(model).unwrap_or_default();
    let online_val = if is_online { 1i64 } else { 0i64 };
    conn.execute(
        "INSERT INTO cnn1d_models (timeframe, window_size, kernel_size, epoch, model_json, thr_long, thr_short, is_online, leverage, capital_percent, weight_decay, l1_lambda, dropout_rate, horizon)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            timeframe, window_size as i64, kernel_size as i64, epoch as i64, model_json,
            thr_long as f64, thr_short as f64, online_val, leverage, capital_percent,
            weight_decay, l1_lambda, dropout_rate, horizon as i64
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_latest_cnn1d_model_for_tf(
    conn: &Connection,
    timeframe: &str,
) -> Result<Option<(crate::cnn1d::Cnn1DModel, f32, f32, usize, usize, f32, f32)>> {
    let mut stmt = conn.prepare(
        "SELECT model_json, thr_long, thr_short, window_size, COALESCE(horizon, 10), COALESCE(weight_decay, 0.001), COALESCE(l1_lambda, 0.0002) FROM cnn1d_models
         WHERE timeframe = ?1
         ORDER BY id DESC LIMIT 1"
    )?;
    let mut rows = stmt.query(params![timeframe])?;
    if let Some(row) = rows.next()? {
        let model_json: String = row.get(0)?;
        let thr_long: f32 = row.get::<_, f64>(1).unwrap_or(0.40) as f32;
        let thr_short: f32 = row.get::<_, f64>(2).unwrap_or(0.40) as f32;
        let window_size: usize = row.get::<_, i64>(3).unwrap_or(30) as usize;
        let horizon: usize = row.get::<_, i64>(4).unwrap_or(10) as usize;
        let weight_decay: f32 = row.get::<_, f64>(5).unwrap_or(0.001) as f32;
        let l1_lambda: f32 = row.get::<_, f64>(6).unwrap_or(0.0002) as f32;
        if let Ok(model) = serde_json::from_str::<crate::cnn1d::Cnn1DModel>(&model_json) {
            return Ok(Some((model, thr_long, thr_short, window_size, horizon, weight_decay, l1_lambda)));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredModelSummary {
    pub id: i64,
    pub timeframe: String,
    pub window_size: usize,
    pub kernel_size: usize,
    pub epoch: usize,
    pub horizon: usize,
    pub leverage: f64,
    pub capital_percent: f64,
    pub weight_decay: f64,
    pub l1_lambda: f64,
    pub is_online: bool,
}

pub fn get_all_stored_models_summary(conn: &Connection) -> Result<Vec<StoredModelSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, timeframe, window_size, kernel_size, epoch, COALESCE(horizon, 10), leverage, capital_percent, COALESCE(weight_decay, 0.001), COALESCE(l1_lambda, 0.0002), is_online 
         FROM cnn1d_models 
         ORDER BY id DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let timeframe: String = row.get(1)?;
        let window_size: usize = row.get::<_, i64>(2)? as usize;
        let kernel_size: usize = row.get::<_, i64>(3)? as usize;
        let epoch: usize = row.get::<_, i64>(4)? as usize;
        let horizon: usize = row.get::<_, i64>(5)? as usize;
        let leverage: f64 = row.get(6)?;
        let capital_percent: f64 = row.get(7)?;
        let weight_decay: f64 = row.get(8)?;
        let l1_lambda: f64 = row.get(9)?;
        let is_online_int: i64 = row.get(10)?;
        Ok(StoredModelSummary {
            id,
            timeframe,
            window_size,
            kernel_size,
            epoch,
            horizon,
            leverage,
            capital_percent,
            weight_decay,
            l1_lambda,
            is_online: is_online_int == 1,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn delete_model_by_id(conn: &Connection, id: i64) -> Result<usize> {
    let affected = conn.execute("DELETE FROM cnn1d_models WHERE id = ?1", params![id])?;
    Ok(affected)
}

pub fn delete_models_by_timeframe(conn: &Connection, tf: &str) -> Result<usize> {
    let affected = conn.execute("DELETE FROM cnn1d_models WHERE timeframe = ?1", params![tf])?;
    Ok(affected)
}

pub fn delete_all_models(conn: &Connection) -> Result<usize> {
    let affected = conn.execute("DELETE FROM cnn1d_models", [])?;
    Ok(affected)
}

// --- Métodos de persistencia de Historial de Inteligencia (intelligence_history) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredIntelligenceRecord {
    pub timeframe: String,
    pub level: usize,
    pub window_size: usize,
    pub kernel_size: usize,
    pub horizon: usize,
    pub epoch: usize,
    pub leverage: f64,
    pub capital_percent: f64,
    pub weight_decay: f64,
    pub l1_lambda: f64,
    pub metrics_nom_json: String,
    pub metrics_pct_json: String,
    pub eq_curve_nom_json: String,
    pub eq_curve_pct_json: String,
    pub model_json: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredIntelligenceSummary {
    pub timeframe: String,
    pub level: usize,
    pub window_size: usize,
    pub kernel_size: usize,
    pub horizon: usize,
    pub epoch: usize,
    pub leverage: f64,
    pub capital_percent: f64,
    pub updated_at: i64,
}

pub fn save_intelligence_record(conn: &Connection, rec: &StoredIntelligenceRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO intelligence_history (
            timeframe, level, window_size, kernel_size, horizon, epoch,
            leverage, capital_percent, weight_decay, l1_lambda,
            metrics_nom_json, metrics_pct_json, eq_curve_nom_json, eq_curve_pct_json,
            model_json, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            rec.timeframe,
            rec.level as i64,
            rec.window_size as i64,
            rec.kernel_size as i64,
            rec.horizon as i64,
            rec.epoch as i64,
            rec.leverage,
            rec.capital_percent,
            rec.weight_decay,
            rec.l1_lambda,
            rec.metrics_nom_json,
            rec.metrics_pct_json,
            rec.eq_curve_nom_json,
            rec.eq_curve_pct_json,
            rec.model_json,
            rec.updated_at,
        ],
    )?;
    Ok(())
}

pub fn get_all_intelligence_records_for_tf(conn: &Connection, tf: &str) -> Result<Vec<StoredIntelligenceRecord>> {
    let mut stmt = conn.prepare(
        "SELECT timeframe, level, window_size, kernel_size, horizon, epoch,
                leverage, capital_percent, weight_decay, l1_lambda,
                metrics_nom_json, metrics_pct_json, eq_curve_nom_json, eq_curve_pct_json,
                model_json, updated_at
         FROM intelligence_history
         WHERE UPPER(timeframe) = UPPER(?1)
         ORDER BY level ASC"
    )?;
    let rows = stmt.query_map(params![tf], |row| {
        Ok(StoredIntelligenceRecord {
            timeframe: row.get(0)?,
            level: row.get::<_, i64>(1)? as usize,
            window_size: row.get::<_, i64>(2)? as usize,
            kernel_size: row.get::<_, i64>(3)? as usize,
            horizon: row.get::<_, i64>(4)? as usize,
            epoch: row.get::<_, i64>(5)? as usize,
            leverage: row.get(6)?,
            capital_percent: row.get(7)?,
            weight_decay: row.get(8)?,
            l1_lambda: row.get(9)?,
            metrics_nom_json: row.get(10)?,
            metrics_pct_json: row.get(11)?,
            eq_curve_nom_json: row.get(12)?,
            eq_curve_pct_json: row.get(13)?,
            model_json: row.get(14)?,
            updated_at: row.get(15)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn get_all_stored_intelligence_summaries(conn: &Connection) -> Result<Vec<StoredIntelligenceSummary>> {
    let mut stmt = conn.prepare(
        "SELECT timeframe, level, window_size, kernel_size, horizon, epoch, leverage, capital_percent, updated_at
         FROM intelligence_history
         ORDER BY timeframe ASC, level ASC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(StoredIntelligenceSummary {
            timeframe: row.get(0)?,
            level: row.get::<_, i64>(1)? as usize,
            window_size: row.get::<_, i64>(2)? as usize,
            kernel_size: row.get::<_, i64>(3)? as usize,
            horizon: row.get::<_, i64>(4)? as usize,
            epoch: row.get::<_, i64>(5)? as usize,
            leverage: row.get(6)?,
            capital_percent: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

pub fn delete_intelligence_records_for_tf(conn: &Connection, tf: &str) -> Result<usize> {
    let affected = conn.execute("DELETE FROM intelligence_history WHERE UPPER(timeframe) = UPPER(?1)", params![tf])?;
    Ok(affected)
}

pub fn delete_intelligence_record_by_level(conn: &Connection, tf: &str, level: usize) -> Result<usize> {
    let affected = conn.execute(
        "DELETE FROM intelligence_history WHERE UPPER(timeframe) = UPPER(?1) AND level = ?2",
        params![tf, level as i64],
    )?;
    Ok(affected)
}

pub fn delete_all_intelligence_records(conn: &Connection) -> Result<usize> {
    let affected = conn.execute("DELETE FROM intelligence_history", [])?;
    Ok(affected)
}


