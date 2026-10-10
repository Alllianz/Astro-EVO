/// Módulo de cálculo de linealidad matemática (R², Regresión MCO, Error Estándar)
/// para curvas de equidad en series temporales de trading.

#[derive(Clone, Debug, Default)]
pub struct LinearityMetrics {
    /// Coeficiente de determinación R² global de la curva (0.0 a 1.0)
    pub r2_global: f64,
    /// Coeficiente de determinación R² del tramo In-Sample
    pub r2_is: f64,
    /// Coeficiente de determinación R² del tramo Out-Of-Sample
    pub r2_oos: f64,
    /// Pendiente de la recta de regresión lineal (MCO)
    pub slope: f64,
    /// Error estándar residual respecto a la tendencia lineal ideal
    pub residual_std_error: f64,
    /// Score compuesto de linealidad equilibrada IS/OOS
    pub composite_linearity_score: f64,
}

/// Calcula la regresión lineal por Mínimos Cuadrados Ordinarios (MCO) y R² sobre una serie de equidad.
pub fn calculate_series_r2(curve: &[(f32, f32)]) -> (f64, f64, f64) {
    let n = curve.len();
    if n < 2 {
        return (0.0, 0.0, 0.0);
    }

    let mut sum_x = 0.0f64;
    let mut sum_y = 0.0f64;
    for (i, &(_, eq)) in curve.iter().enumerate() {
        sum_x += i as f64;
        sum_y += eq as f64;
    }
    let mean_x = sum_x / (n as f64);
    let mean_y = sum_y / (n as f64);

    let mut s_xx = 0.0f64;
    let mut s_yy = 0.0f64;
    let mut s_xy = 0.0f64;

    for (i, &(_, eq)) in curve.iter().enumerate() {
        let dx = (i as f64) - mean_x;
        let dy = (eq as f64) - mean_y;
        s_xx += dx * dx;
        s_yy += dy * dy;
        s_xy += dx * dy;
    }

    if s_xx <= 1e-12 || s_yy <= 1e-12 {
        return (0.0, 0.0, 0.0);
    }

    let slope = s_xy / s_xx;
    if slope <= 0.0 {
        // Curva decreciente o plana: R² efectivo para trading alcista es 0.0
        return (0.0, slope, 0.0);
    }

    let r2 = ((s_xy * s_xy) / (s_xx * s_yy)).clamp(0.0, 1.0);

    // Calcular error estándar residual
    let mut ss_res = 0.0f64;
    let intercept = mean_y - (slope * mean_x);
    for (i, &(_, eq)) in curve.iter().enumerate() {
        let y_pred = intercept + slope * (i as f64);
        let diff = (eq as f64) - y_pred;
        ss_res += diff * diff;
    }
    let std_err = (ss_res / (n as f64)).sqrt();

    (r2, slope, std_err)
}

/// Evalúa la linealidad desagregada y compuesta tanto en In-Sample (IS) como en Out-Of-Sample (OOS).
pub fn evaluate_equity_linearity(
    curve: &[(f32, f32)],
    oos_start_idx: usize,
) -> LinearityMetrics {
    if curve.len() < 4 {
        return LinearityMetrics::default();
    }

    let (r2_global, slope_global, std_err_global) = calculate_series_r2(curve);

    // Particionar en tramo IS y OOS
    let is_slice: Vec<(f32, f32)> = curve.iter()
        .filter(|&&(x, _)| (x as usize) < oos_start_idx)
        .copied()
        .collect();

    let oos_slice: Vec<(f32, f32)> = curve.iter()
        .filter(|&&(x, _)| (x as usize) >= oos_start_idx)
        .copied()
        .collect();

    let (r2_is, _, _) = if is_slice.len() >= 2 {
        calculate_series_r2(&is_slice)
    } else {
        (0.0, 0.0, 0.0)
    };

    let (r2_oos, _, _) = if oos_slice.len() >= 2 {
        calculate_series_r2(&oos_slice)
    } else {
        (0.0, 0.0, 0.0)
    };

    // Score compuesto: Penaliza fuertemente si uno de los dos tramos se descalabra
    let min_seg_r2 = r2_is.min(r2_oos);
    let avg_seg_r2 = (r2_is + r2_oos) / 2.0;
    let composite = 0.40 * r2_global + 0.35 * avg_seg_r2 + 0.25 * min_seg_r2;

    LinearityMetrics {
        r2_global,
        r2_is,
        r2_oos,
        slope: slope_global,
        residual_std_error: std_err_global,
        composite_linearity_score: composite.clamp(0.0, 1.0),
    }
}
