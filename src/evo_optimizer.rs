use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockTrial {
    pub window_size: usize,
    pub kernel_size: usize,
    pub horizon: usize,
    pub weight_decay: f32,
    pub l1_lambda: f32,
    pub best_epoch: usize,
    pub fitness: f64,
}

pub struct EvoOptimizer {
    pub seed: u64,
    pub history: Vec<BlockTrial>,
    pub num_initial_trials: usize,
}

impl EvoOptimizer {
    pub fn new(seed_val: u64, num_initial_trials: usize) -> Self {
        Self {
            seed: seed_val,
            history: Vec::new(),
            num_initial_trials,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.seed >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f64 / u32::MAX as f64) as f32
    }

    fn next_gaussian(&mut self) -> f32 {
        let u1 = self.next_f32().max(1e-7);
        let u2 = self.next_f32();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
    }

    /// Genera una muestra inicial exploratoria respetando la física de los bloques
    pub fn sample_candidate(&mut self) -> (usize, usize, usize, f32, f32) {
        if self.history.len() < self.num_initial_trials || self.next_f32() < 0.25 {
            // Muestreo Exploratorio

            // Bloque 1: (Ventana W, Kernel K)
            let w = (20 + (self.next_u32() % 61)) as usize; // Ventana W entre 20 y 80
            let max_k = ((w + 1) / 2).max(2);
            let k = (2 + (self.next_u32() as usize % (max_k - 1))).min(w - 1);

            // Bloque 2: Horizonte H (independiente)
            let h = (5 + (self.next_u32() % 26)) as usize; // Horizonte H entre 5 y 30

            // Bloque 3: (Weight Decay WD, L1 Lambda L1) en espacio logarítmico
            let log_wd = -5.0 + (self.next_f32() * 4.0);
            let wd = 10.0f32.powf(log_wd);

            let l1 = if self.next_f32() < 0.30 {
                0.0f32
            } else {
                let log_l1 = -5.0 + (self.next_f32() * 2.7);
                10.0f32.powf(log_l1)
            };

            (w, k, h, wd, l1)
        } else {
            // Explotación Adaptativa mutando uno de los mejores padres históricos
            let mut sorted = self.history.clone();
            sorted.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal));

            let top_k = sorted.len().min(3);
            let parent_idx = (self.next_u32() as usize) % top_k;
            let parent = &sorted[parent_idx];

            let parent_tuple = (parent.window_size, parent.kernel_size, parent.horizon, parent.weight_decay, parent.l1_lambda);
            self.mutate_config(parent_tuple)
        }
    }

    /// Mutación por Bloques Coordinados
    pub fn mutate_config(&mut self, parent: (usize, usize, usize, f32, f32)) -> (usize, usize, usize, f32, f32) {
        let block_to_mutate = (self.next_u32() % 3) as usize;

        let mut w = parent.0;
        let mut k = parent.1;
        let mut h = parent.2;
        let mut wd = parent.3;
        let mut l1 = parent.4;

        match block_to_mutate {
            0 => {
                // Bloque 1: Mutación Coordinada de (Ventana W, Kernel K)
                let delta_w = (self.next_gaussian() * 6.0).round() as i32;
                let new_w = ((w as i32) + delta_w).clamp(20, 90) as usize;
                let ratio = (k as f32) / (w as f32);
                
                w = new_w;
                let max_k = ((w + 1) / 2).max(2);
                let target_k = (w as f32 * ratio).round() as usize;
                k = target_k.clamp(2, max_k.min(w - 1));
            }
            1 => {
                // Bloque 2: Mutación del Horizonte H (independiente)
                let delta_h = (self.next_gaussian() * 3.0).round() as i32;
                let new_h = ((h as i32) + delta_h).clamp(4, 35) as usize;
                h = new_h;
            }
            _ => {
                // Bloque 3: Mutación Coordinada de Regularización (WD, L1)
                let log_wd = wd.ln() + self.next_gaussian() * 0.30;
                wd = log_wd.exp().clamp(0.00001, 0.1);

                if l1 == 0.0 {
                    if self.next_f32() < 0.3 {
                        l1 = 0.0001f32;
                    }
                } else {
                    let log_l1 = l1.ln() + self.next_gaussian() * 0.35;
                    l1 = log_l1.exp().clamp(0.0, 0.005);
                    if l1 < 1e-6 {
                        l1 = 0.0;
                    }
                }
            }
        }

        (w, k, h, wd, l1)
    }

    pub fn record_trial(
        &mut self,
        window_size: usize,
        kernel_size: usize,
        horizon: usize,
        weight_decay: f32,
        l1_lambda: f32,
        best_epoch: usize,
        fitness: f64,
    ) {
        self.history.push(BlockTrial {
            window_size,
            kernel_size,
            horizon,
            weight_decay,
            l1_lambda,
            best_epoch,
            fitness,
        });
    }
}
