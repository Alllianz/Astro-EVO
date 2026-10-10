use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntelligenceSweepConfig {
    pub tf: String,
    pub start_intelligence: usize,
    pub iterations: usize,
    pub capital_percent: f64,
    pub leverage: f64,
    pub window_size: usize,
    pub kernel_size: usize,
    pub horizon: usize,
    pub learning_rate: f32,
    pub weight_decay: f32,
    pub l1_lambda: f32,
    pub seed_val: u32,
    pub pre_train_epochs: usize,
    pub num_threads: usize,
}

impl IntelligenceSweepConfig {
    pub fn get_intelligence_levels(&self) -> Vec<usize> {
        let start = self.start_intelligence.max(1);
        let count = self.iterations.max(1);
        (start..(start + count)).collect()
    }
}
