use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelArchitectureSpec {
    pub input_features: usize,
    pub conv1_channels: usize,
    pub conv2_channels: usize,
    pub dense_dim: usize,
    pub output_dim: usize,
}

impl ModelArchitectureSpec {
    pub const fn new(
        input_features: usize,
        conv1_channels: usize,
        conv2_channels: usize,
        dense_dim: usize,
        output_dim: usize,
    ) -> Self {
        Self {
            input_features,
            conv1_channels,
            conv2_channels,
            dense_dim,
            output_dim,
        }
    }
}

/// Devuelve la especificación de arquitectura exacta según la temporalidad:
/// - 1D: Arquitectura compacta macro (conv1: 8 filtros, conv2: 16 filtros, dense: 16 -> 3)
/// - 4H: Modelo base estándar (conv1: 64, conv2: 128, dense: 128 -> 3)
/// - 1H: Duplica la cantidad de parámetros (2x, conv1: 90, conv2: 180, dense: 180 -> 3) + Feature Hora del Día (35 features)
pub fn get_model_spec_for_timeframe(timeframe: &str) -> ModelArchitectureSpec {
    let tf_upper = timeframe.to_uppercase();
    match tf_upper.as_str() {
        "1D" | "1d" => ModelArchitectureSpec::new(
            34, // Features base 1D (33 técnicas + 1 posición de cuenta)
            8,  // Conv1: 8 filtros (detecta giros de tendencia y patrones de varios días)
            16, // Conv2: 16 filtros (combina giros con la tendencia general del mes)
            16, // Global Average Pooling resume a 16 -> Dense dimensión (16 -> 3)
            3,  // Long, Short, Flat
        ),
        "4H" | "4h" => ModelArchitectureSpec::new(
            34,  // Features base 4H (33 técnicas + 1 posición de cuenta)
            64,  // Conv1 canales estándar
            128, // Conv2 canales estándar
            128, // Dense dimensión estándar
            3,   // Long, Short, Flat
        ),
        "1H" | "1h" => ModelArchitectureSpec::new(
            35,  // Features 1H: 33 base + 1 Hora del Día normalizada (0..1) + 1 posición de cuenta
            90,  // Conv1 canales escalados
            180, // Conv2 canales escalados (~2x parámetros totales vs 4H)
            180, // Dense dimensión escalada
            3,   // Long, Short, Flat
        ),
        // Para otras temporalidades intradiarias (ej: 15m, 5m, 1m) adaptamos proporcionalmente
        "15M" | "15m" | "5M" | "5m" | "1M" | "1m" => ModelArchitectureSpec::new(
            35,
            90,
            180,
            180,
            3,
        ),
        "12H" | "12h" | "8H" | "8h" => ModelArchitectureSpec::new(
            34,
            64,
            128,
            128,
            3,
        ),
        _ => ModelArchitectureSpec::new(
            34,
            64,
            128,
            128,
            3,
        ),
    }
}

/// Devuelve la especificación de arquitectura escalada según el nivel de inteligencia L (L -> 2L -> 2L)
pub fn get_model_spec_for_intelligence(timeframe: &str, intelligence: usize) -> ModelArchitectureSpec {
    let base_spec = get_model_spec_for_timeframe(timeframe);
    let c1 = intelligence.max(1);
    let c2 = (intelligence * 2).max(2);
    let dense_dim = (intelligence * 2).max(2);
    ModelArchitectureSpec::new(
        base_spec.input_features,
        c1,
        c2,
        dense_dim,
        base_spec.output_dim,
    )
}

/// Devuelve la cantidad de features brutas (sin incluir la columna de posición de cuenta) para la temporalidad
pub fn get_raw_feature_count_for_timeframe(timeframe: &str) -> usize {
    let spec = get_model_spec_for_timeframe(timeframe);
    // El último canal siempre está reservado para la posición actual de la cuenta
    spec.input_features.saturating_sub(1)
}

