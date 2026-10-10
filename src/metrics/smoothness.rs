/// Módulo de cálculo de suavidad de curva de equidad, Sortino Ratio y penalización de varianza de drawdowns.

#[derive(Clone, Debug, Default)]
pub struct SmoothnessMetrics {
    /// Sortino Ratio basado en retornos por trade / punto de equidad
    pub sortino_ratio: f64,
    /// Desviación a la baja (Downside Deviation)
    pub downside_deviation: f64,
    /// Ratio de Suavidad (Linealidad / (1.0 + Drawdown Volatility))
    pub smoothness_score: f64,
    /// Ulcer Index (penalización cuadrática por profundidad y duración bajo máximos)
    pub ulcer_index: f64,
}

/// Calcula el Ulcer Index y métricas de suavidad sobre la curva de equidad.
pub fn calculate_equity_smoothness(
    curve: &[(f32, f32)],
    trade_pnls: &[f64],
    composite_linearity: f64,
) -> SmoothnessMetrics {
    if curve.len() < 2 {
        return SmoothnessMetrics::default();
    }

    // 1. Cálculo de Ulcer Index sobre la curva de equidad
    let mut peak = curve[0].1 as f64;
    let mut sum_sq_dd = 0.0f64;

    for &(_, eq) in curve {
        let eq_f64 = eq as f64;
        if eq_f64 > peak {
            peak = eq_f64;
        }
        let dd_pct = if peak > 0.0 {
            ((peak - eq_f64) / peak) * 100.0
        } else {
            0.0
        };
        sum_sq_dd += dd_pct * dd_pct;
    }
    let ulcer_index = (sum_sq_dd / (curve.len() as f64)).sqrt();

    // 2. Cálculo de Sortino Ratio sobre los trades individuales
    let (sortino_ratio, downside_dev) = if trade_pnls.is_empty() {
        (0.0, 0.0)
    } else {
        let mean_pnl = trade_pnls.iter().sum::<f64>() / (trade_pnls.len() as f64);
        let mut sum_downside_sq = 0.0f64;
        let mut downside_count = 0;

        for &pnl in trade_pnls {
            if pnl < 0.0 {
                sum_downside_sq += pnl * pnl;
                downside_count += 1;
            }
        }

        let downside_dev = if downside_count > 0 {
            (sum_downside_sq / (trade_pnls.len() as f64)).sqrt()
        } else {
            0.0
        };

        let sortino = if downside_dev > 1e-6 {
            mean_pnl / downside_dev
        } else if mean_pnl > 0.0 {
            10.0 // Gran consistencia sin downside significativo
        } else {
            0.0
        };

        (sortino.max(0.0), downside_dev)
    };

    // 3. Score de Suavidad Compuesto
    // Combina alta linealidad (R²) castigando el Ulcer Index (tiempo y profundidad de drawdown)
    let smoothness = (composite_linearity / (1.0 + (ulcer_index / 10.0))).clamp(0.0, 1.0);

    SmoothnessMetrics {
        sortino_ratio,
        downside_deviation: downside_dev,
        smoothness_score: smoothness,
        ulcer_index,
    }
}
