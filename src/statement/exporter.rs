use super::calculator::calculate_statement_row;
use super::models::{SimulatedTrade, StatementRow};
use crate::db::Kline;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Encabezado estándar idéntico al reporte de Myfxbook
pub const MYFXBOOK_CSV_HEADER: &str = "Open Date,Close Date,Symbol,Action,Units/Lots,SL,TP,Open Price,Close Price,Commission,Swap,Pips,Profit,Gain,Duration (DD:HH:MM:SS),Profitable(%),Profitable(time duration),Drawdown,Risk:Reward,Max(pips),Max(EUR),Min(pips),Min(EUR),Entry Accuracy(%),Exit Accuracy(%),ProfitMissed(pips),ProfitMissed(EUR)";

/// Convierte una fila StatementRow en una línea CSV con comas
pub fn statement_row_to_csv_line(row: &StatementRow) -> String {
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        row.open_date,
        row.close_date,
        row.symbol,
        row.action,
        row.units_lots,
        row.sl,
        row.tp,
        row.open_price,
        row.close_price,
        row.commission,
        row.swap,
        row.pips,
        row.profit,
        row.gain,
        row.duration,
        row.profitable_pct,
        row.profitable_duration,
        row.drawdown,
        row.risk_reward,
        row.max_pips,
        row.max_currency,
        row.min_pips,
        row.min_currency,
        row.entry_accuracy_pct,
        row.exit_accuracy_pct,
        row.profit_missed_pips,
        row.profit_missed_currency
    )
}

/// Genera el contenido completo del CSV a partir de trades y velas
pub fn generate_statement_csv_content(
    trades: &[SimulatedTrade],
    klines: &[Kline],
    timeframe: &str,
) -> String {
    // Calculamos todas las filas
    let mut rows: Vec<StatementRow> = trades
        .iter()
        .map(|t| calculate_statement_row(t, klines, timeframe))
        .collect();

    // Orden cronológico inverso (el trade más reciente primero), idéntico al CSV original
    rows.reverse();

    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(MYFXBOOK_CSV_HEADER.to_string());
    for row in rows {
        lines.push(statement_row_to_csv_line(&row));
    }

    lines.join("\n")
}

/// Exporta el historial de operaciones a un archivo CSV en disco
pub fn export_statement_csv(
    trades: &[SimulatedTrade],
    klines: &[Kline],
    timeframe: &str,
    output_path: &Path,
) -> io::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = generate_statement_csv_content(trades, klines, timeframe);
    let mut file = File::create(output_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statement::models::TradeAction;

    #[test]
    fn test_csv_header_columns_count() {
        let cols: Vec<&str> = MYFXBOOK_CSV_HEADER.split(',').collect();
        assert_eq!(cols.len(), 27, "El header de Myfxbook debe tener exactamente 27 columnas");
    }

    #[test]
    fn test_statement_row_generation_and_order() {
        let klines = vec![
            Kline { timestamp: 1600000000000, open: 10000.0, high: 10200.0, low: 9900.0, close: 10100.0, volume: 100.0 },
            Kline { timestamp: 1600086400000, open: 10100.0, high: 10300.0, low: 10050.0, close: 10250.0, volume: 120.0 },
        ];

        let trades = vec![
            SimulatedTrade {
                trade_id: 1,
                open_idx: 0,
                close_idx: 0,
                open_timestamp: 1600000000000,
                close_timestamp: 1600000000000 + 3600000,
                symbol: "BTCUSDT".to_string(),
                action: TradeAction::Buy,
                units_lots: 0.1,
                sl_price: 9500.0,
                tp_price: 0.0,
                open_price: 10000.0,
                close_price: 10100.0,
                commission: -1.0,
                swap: 0.0,
                net_profit: 10.0,
                capital_at_open: 10000.0,
                is_liquidation: false,
            },
            SimulatedTrade {
                trade_id: 2,
                open_idx: 1,
                close_idx: 1,
                open_timestamp: 1600086400000,
                close_timestamp: 1600086400000 + 7200000,
                symbol: "BTCUSDT".to_string(),
                action: TradeAction::Sell,
                units_lots: 0.1,
                sl_price: 10500.0,
                tp_price: 0.0,
                open_price: 10100.0,
                close_price: 10250.0,
                commission: -1.0,
                swap: 0.0,
                net_profit: -15.0,
                capital_at_open: 10010.0,
                is_liquidation: false,
            },
        ];

        let csv = generate_statement_csv_content(&trades, &klines, "1D");
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "Debe tener 1 fila de header y 2 filas de datos");
        assert_eq!(lines[0], MYFXBOOK_CSV_HEADER);
        // Orden inverso: trade 2 debe ir primero
        assert!(lines[1].contains("Sell"));
        assert!(lines[2].contains("Buy"));

        // Verificar que cada fila tenga 27 columnas
        for line in &lines {
            assert_eq!(line.split(',').count(), 27);
        }
    }
}
