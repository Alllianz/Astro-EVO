use crate::db::Kline;
use crate::indicators::precalculate_atr;

/// Configuración para el etiquetado causal mediante el Método de la Triple Barrera (Triple Barrier Method)
/// Adaptado estrictamente para mercados de derivados con apalancamiento sin Look-Ahead Bias.
#[derive(Clone, Debug, PartialEq)]
pub struct TripleBarrierConfig {
    /// Horizonte de tiempo máximo (Barrera Vertical) en cantidad de velas
    pub horizon: usize,
    /// Multiplicador de volatilidad ATR para la barrera de Take Profit
    pub pt_atr_mult: f64,
    /// Multiplicador de volatilidad ATR para la barrera de Stop Loss
    pub sl_atr_mult: f64,
    /// Pérdida máxima porcentual permitida (techo de protección antes de la liquidación de margen)
    pub max_loss_cap: f64,
    /// Umbral mínimo de movimiento relativo a ATR para clasificar una tendencia no plana
    pub min_return_threshold: f64,
}

impl Default for TripleBarrierConfig {
    fn default() -> Self {
        Self {
            horizon: 10,
            pt_atr_mult: 2.0,
            sl_atr_mult: 1.0,
            max_loss_cap: 0.075, // 7.5% de pérdida máxima (protege sobradamente antes del 9.95% de liquidación a 10X)
            min_return_threshold: 0.5,
        }
    }
}

impl TripleBarrierConfig {
    pub fn new(
        horizon: usize,
        pt_atr_mult: f64,
        sl_atr_mult: f64,
        max_loss_cap: f64,
        min_return_threshold: f64,
    ) -> Self {
        Self {
            horizon,
            pt_atr_mult,
            sl_atr_mult,
            max_loss_cap,
            min_return_threshold,
        }
    }

    /// Construye una configuración optimizada según el timeframe y el apalancamiento
    pub fn for_timeframe(tf: &str, horizon: usize, leverage: f64) -> Self {
        let lev = if leverage > 0.0 { leverage } else { 10.0 };
        // El umbral de liquidación a 10X es 1/10 = 10%. Fijamos el cap de pérdida técnica al 75% del margen
        let safe_sl_cap = (1.0 / lev) * 0.75;

        match tf.to_uppercase().as_str() {
            "1D" => Self {
                horizon,
                pt_atr_mult: 2.0,
                sl_atr_mult: 1.0,
                max_loss_cap: safe_sl_cap.min(0.08),
                min_return_threshold: 0.5,
            },
            "4H" => Self {
                horizon,
                pt_atr_mult: 1.8,
                sl_atr_mult: 1.0,
                max_loss_cap: safe_sl_cap.min(0.085),
                min_return_threshold: 0.5,
            },
            _ => Self {
                horizon,
                pt_atr_mult: 1.5,
                sl_atr_mult: 1.0,
                max_loss_cap: safe_sl_cap.min(0.09),
                min_return_threshold: 0.4,
            },
        }
    }
}

/// Genera etiquetas causales (0: LONG, 1: SHORT, 2: FLAT) utilizando el Método de la Triple Barrera.
///
/// Principios clave para evitar sesgos y liquidaciones:
/// 1. **Causalidad Temporal (Sin Look-Ahead)**: Las velas futuras se inspeccionan paso a paso (t+1..=t+H).
/// 2. **Prioridad del Drawdown**: Si el precio cae por debajo del Stop Loss antes de alcanzar el Take Profit,
///    la operación se considera fallida y JAMÁS se etiqueta como LONG.
/// 3. **Liquidación Preventiva**: La barrera de pérdida está acotada por `max_loss_cap` para evitar que la red
///    aprenda patrones que en ejecución real causan liquidación inmediata.
pub fn generate_triple_barrier_labels(klines: &[Kline], config: &TripleBarrierConfig) -> Vec<usize> {
    let mut labels = vec![2; klines.len()];
    if klines.len() < 205 + config.horizon {
        return labels;
    }

    let atr_50 = precalculate_atr(klines, 50);

    for idx in 200..klines.len() {
        if idx + config.horizon >= klines.len() {
            break;
        }

        let current_price = klines[idx].close;
        if current_price <= 0.0 {
            continue;
        }

        let current_atr = atr_50[idx];
        let atr_ratio = (current_atr / current_price).max(0.001);

        // Magnitudes relativas de las barreras horizontales
        let pt_pct = config.pt_atr_mult * atr_ratio;
        let sl_pct = (config.sl_atr_mult * atr_ratio).min(config.max_loss_cap);
        let min_thr = config.min_return_threshold * atr_ratio;

        // Barreras para posición de COMPRA (LONG)
        let long_tp = current_price * (1.0 + pt_pct);
        let long_sl = current_price * (1.0 - sl_pct);

        // Barreras para posición de VENTA (SHORT)
        let short_tp = current_price * (1.0 - pt_pct);
        let short_sl = current_price * (1.0 + sl_pct);

        let mut long_hit_step: Option<usize> = None;
        let mut short_hit_step: Option<usize> = None;
        let mut long_stopped = false;
        let mut short_stopped = false;

        // Recorrido cronológico vela por vela en el horizonte futuro
        for i in 1..=config.horizon {
            let next_k = &klines[idx + i];

            // Evaluación de la trayectoria para LONG:
            if !long_stopped && long_hit_step.is_none() {
                let sl_hit = next_k.low <= long_sl;
                let tp_hit = next_k.high >= long_tp;

                if sl_hit && tp_hit {
                    // Si en la misma vela toca ambas barreras, en ejecución apalancada
                    // se asume el escenario conservador (Stop Loss antes de Take Profit)
                    long_stopped = true;
                } else if sl_hit {
                    long_stopped = true;
                } else if tp_hit {
                    long_hit_step = Some(i);
                }
            }

            // Evaluación de la trayectoria para SHORT:
            if !short_stopped && short_hit_step.is_none() {
                let sl_hit = next_k.high >= short_sl;
                let tp_hit = next_k.low <= short_tp;

                if sl_hit && tp_hit {
                    short_stopped = true;
                } else if sl_hit {
                    short_stopped = true;
                } else if tp_hit {
                    short_hit_step = Some(i);
                }
            }

            // Si ambas trayectorias concluyeron (o tocaron TP o fueron detenidas por SL), salir antes
            if (long_stopped || long_hit_step.is_some()) && (short_stopped || short_hit_step.is_some()) {
                break;
            }
        }

        // Determinación final de la etiqueta:
        match (long_hit_step, short_hit_step) {
            (Some(_l_step), None) => {
                labels[idx] = 0; // Ganó LONG sin tocar SL
            }
            (None, Some(_s_step)) => {
                labels[idx] = 1; // Ganó SHORT sin tocar SL
            }
            (Some(l_step), Some(s_step)) => {
                if l_step < s_step {
                    labels[idx] = 0; // LONG tocó Take Profit antes que SHORT
                } else if s_step < l_step {
                    labels[idx] = 1; // SHORT tocó Take Profit antes que LONG
                } else {
                    labels[idx] = 2; // Empate en la misma vela -> FLAT por ambigüedad
                }
            }
            (None, None) => {
                // Ninguna barrera fue alcanzada (Barrera Vertical alcanzada al final del horizonte).
                // Evaluamos el retorno neto al vencimiento del horizonte (idx + horizon), verificando
                // que la excursión adversa máxima (drawdown) no haya violado el límite de riesgo.
                let final_close = klines[idx + config.horizon].close;
                let net_return = (final_close - current_price) / current_price;

                let mut min_during = current_price;
                let mut max_during = current_price;
                for j in 1..=config.horizon {
                    let k = &klines[idx + j];
                    if k.low < min_during { min_during = k.low; }
                    if k.high > max_during { max_during = k.high; }
                }

                let max_adverse_long = (current_price - min_during) / current_price;
                let max_adverse_short = (max_during - current_price) / current_price;

                if net_return > min_thr && max_adverse_long <= config.max_loss_cap {
                    labels[idx] = 0; // LONG moderado sin violación de riesgo
                } else if -net_return > min_thr && max_adverse_short <= config.max_loss_cap {
                    labels[idx] = 1; // SHORT moderado sin violación de riesgo
                } else {
                    labels[idx] = 2; // FLAT
                }
            }
        }
    }

    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_kline(timestamp: i64, open: f64, high: f64, low: f64, close: f64) -> Kline {
        Kline {
            timestamp,
            open,
            high,
            low,
            close,
            volume: 100.0,
        }
    }

    #[test]
    fn test_long_prevented_by_earlier_drawdown() {
        // Escenario crítico en 1D: El precio sube +20% al final, pero primero cae -10% (liquidación).
        // El etiquetador de Triple Barrera DEBE descartar el LONG.
        let mut klines = Vec::new();
        for i in 0..220 {
            klines.push(create_test_kline(i * 86400000, 100.0, 101.0, 99.0, 100.0));
        }

        let base_idx = 205;
        // Día 1: Cae fuertemente a 90.0 (-10%, supera el Stop Loss / Cap)
        klines[base_idx + 1] = create_test_kline((base_idx as i64 + 1) * 86400000, 100.0, 100.5, 90.0, 91.0);
        // Día 5: Sube fuertemente a 125.0 (+25%)
        klines[base_idx + 5] = create_test_kline((base_idx as i64 + 5) * 86400000, 95.0, 125.0, 94.0, 120.0);

        let cfg = TripleBarrierConfig {
            horizon: 10,
            pt_atr_mult: 2.0,
            sl_atr_mult: 1.0,
            max_loss_cap: 0.075, // Cap de 7.5%
            min_return_threshold: 0.5,
        };

        let labels = generate_triple_barrier_labels(&klines, &cfg);
        // Debe ser 2 (FLAT) o 1 (SHORT), pero NUNCA 0 (LONG), porque se detuvo por SL el día 1.
        assert_ne!(labels[base_idx], 0, "El trade debió ser descalificado como LONG debido a la caída previa");
    }

    #[test]
    fn test_clean_long_trigger() {
        let mut klines = Vec::new();
        for i in 0..220 {
            klines.push(create_test_kline(i * 86400000, 100.0, 101.0, 99.0, 100.0));
        }

        let base_idx = 205;
        // Subida limpia a 108.0 sin caídas
        klines[base_idx + 2] = create_test_kline((base_idx as i64 + 2) * 86400000, 101.0, 108.0, 100.5, 107.5);

        let cfg = TripleBarrierConfig {
            horizon: 10,
            pt_atr_mult: 2.0,
            sl_atr_mult: 1.0,
            max_loss_cap: 0.075,
            min_return_threshold: 0.5,
        };

        let labels = generate_triple_barrier_labels(&klines, &cfg);
        assert_eq!(labels[base_idx], 0, "Debe etiquetarse como LONG");
    }
}
