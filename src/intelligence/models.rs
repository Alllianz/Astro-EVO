use crate::cnn1d::Cnn1DModel;
use crate::metrics::CandidateRankScore;
use crate::online_learning::{AstroSubMetrics, OnlineLearningConfig};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntelligenceLayerInfo {
    pub conv1_in: usize,
    pub conv1_out: usize,
    pub conv1_weights_count: usize,
    pub conv2_in: usize,
    pub conv2_out: usize,
    pub conv2_weights_count: usize,
    pub dense_in: usize,
    pub dense_out: usize,
    pub dense_weights_count: usize,
    pub total_weights_count: usize,
}

impl IntelligenceLayerInfo {
    pub fn from_model(model: &Cnn1DModel, level: usize) -> Self {
        let conv1_in = model.conv1.in_channels;
        let conv1_out = model.conv1.out_channels;
        let conv1_weights = model.conv1.weights.len() + model.conv1.bias.len();

        let conv2_in = model.conv2.in_channels;
        let conv2_out = model.conv2.out_channels;
        let conv2_weights = model.conv2.weights.len() + model.conv2.bias.len();

        let dense_in = model.dense.weights.shape()[0];
        let dense_out = model.dense.weights.shape()[1];
        let dense_weights = model.dense.weights.len() + model.dense.bias.len();

        let total = conv1_weights + conv2_weights + dense_weights;

        Self {
            conv1_in,
            conv1_out: if conv1_out > 0 { conv1_out } else { level },
            conv1_weights_count: conv1_weights,
            conv2_in: if conv2_in > 0 { conv2_in } else { level },
            conv2_out: if conv2_out > 0 { conv2_out } else { level * 2 },
            conv2_weights_count: conv2_weights,
            dense_in: if dense_in > 0 { dense_in } else { level * 2 },
            dense_out,
            dense_weights_count: dense_weights,
            total_weights_count: total,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IntelligenceLevelResult {
    pub level: usize,
    pub layer_info: IntelligenceLayerInfo,
    pub config: OnlineLearningConfig,
    pub metrics_nom: AstroSubMetrics,
    pub metrics_pct: AstroSubMetrics,
    pub model: Cnn1DModel,
    pub eq_curve_nom: Vec<(f32, f32)>,
    pub eq_curve_pct: Vec<(f32, f32)>,
    pub rank_score: CandidateRankScore,
    pub epoch: usize,
}

impl IntelligenceLevelResult {
    pub fn to_stored_record(&self) -> crate::db::StoredIntelligenceRecord {
        crate::db::StoredIntelligenceRecord {
            timeframe: self.config.tf.clone(),
            level: self.level,
            window_size: self.config.window_size,
            kernel_size: self.config.kernel_size,
            horizon: self.config.horizon,
            epoch: self.epoch,
            leverage: self.config.leverage,
            capital_percent: self.config.capital_percent,
            weight_decay: self.config.weight_decay as f64,
            l1_lambda: self.config.l1_lambda as f64,
            metrics_nom_json: serde_json::to_string(&self.metrics_nom).unwrap_or_default(),
            metrics_pct_json: serde_json::to_string(&self.metrics_pct).unwrap_or_default(),
            eq_curve_nom_json: serde_json::to_string(&self.eq_curve_nom).unwrap_or_default(),
            eq_curve_pct_json: serde_json::to_string(&self.eq_curve_pct).unwrap_or_default(),
            model_json: serde_json::to_string(&self.model).unwrap_or_default(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn from_stored_record(rec: &crate::db::StoredIntelligenceRecord) -> Option<Self> {
        let metrics_nom: AstroSubMetrics = serde_json::from_str(&rec.metrics_nom_json).ok()?;
        let metrics_pct: AstroSubMetrics = serde_json::from_str(&rec.metrics_pct_json).ok()?;
        let eq_curve_nom: Vec<(f32, f32)> = serde_json::from_str(&rec.eq_curve_nom_json).ok()?;
        let eq_curve_pct: Vec<(f32, f32)> = serde_json::from_str(&rec.eq_curve_pct_json).ok()?;
        let model: Cnn1DModel = serde_json::from_str(&rec.model_json).ok()?;

        let layer_info = IntelligenceLayerInfo::from_model(&model, rec.level);

        let config = OnlineLearningConfig {
            tf: rec.timeframe.clone(),
            capital_percent: rec.capital_percent,
            leverage: rec.leverage,
            window_size: rec.window_size,
            kernel_size: rec.kernel_size,
            horizon: rec.horizon,
            learning_rate: 0.001,
            weight_decay: rec.weight_decay as f32,
            l1_lambda: rec.l1_lambda as f32,
            seed_val: 42,
            pre_train_epochs: rec.epoch,
            intelligence: rec.level,
        };

        Some(Self {
            level: rec.level,
            layer_info,
            config,
            metrics_nom,
            metrics_pct,
            model,
            eq_curve_nom,
            eq_curve_pct,
            rank_score: CandidateRankScore {
                candidate_id: format!("Level_{}", rec.level),
                rank_slope: 999,
                rank_cap_dd: 999,
                rank_r2: 999,
                rank_smoothness: 999,
                weighted_avg_rank: 999.0,
            },
            epoch: rec.epoch,
        })
    }
}

