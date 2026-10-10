use ndarray::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Conv1D {
    pub in_channels: usize,
    pub out_channels: usize,
    pub kernel_size: usize,
    pub weights: Array3<f32>,
    pub bias: Array1<f32>,
    #[serde(skip)]
    pub last_input: Option<Array2<f32>>,
}

#[inline(always)]
pub fn update_adam_param_slice(
    weights: &mut [f32],
    grads: &[f32],
    m: &mut [f32],
    v: &mut [f32],
    lr: f32,
    wd: f32,
    l1_lambda: f32,
    t: f32,
) {
    let beta1 = 0.9f32;
    let beta2 = 0.999f32;
    let eps = 1e-8f32;
    let correction1 = 1.0 - beta1.powf(t);
    let correction2 = 1.0 - beta2.powf(t);
    let l1_penalty = lr * l1_lambda;
    let lr_decay_factor = 1.0 - lr * wd;

    for i in 0..weights.len() {
        let g = grads[i];
        let m_val = beta1 * m[i] + (1.0 - beta1) * g;
        let v_val = beta2 * v[i] + (1.0 - beta2) * (g * g);
        m[i] = m_val;
        v[i] = v_val;

        let m_hat = m_val / correction1;
        let v_hat = v_val / correction2;

        let mut w = weights[i] * lr_decay_factor;
        if w > l1_penalty {
            w -= l1_penalty;
        } else if w < -l1_penalty {
            w += l1_penalty;
        } else {
            w = 0.0;
        }

        weights[i] = w - (lr * m_hat / (v_hat.sqrt() + eps));
    }
}

#[inline(always)]
pub fn update_adam_bias_slice(
    bias: &mut [f32],
    grads: &[f32],
    m: &mut [f32],
    v: &mut [f32],
    lr: f32,
    t: f32,
) {
    let beta1 = 0.9f32;
    let beta2 = 0.999f32;
    let eps = 1e-8f32;
    let correction1 = 1.0 - beta1.powf(t);
    let correction2 = 1.0 - beta2.powf(t);

    for i in 0..bias.len() {
        let g = grads[i];
        let m_val = beta1 * m[i] + (1.0 - beta1) * g;
        let v_val = beta2 * v[i] + (1.0 - beta2) * (g * g);
        m[i] = m_val;
        v[i] = v_val;

        let m_hat = m_val / correction1;
        let v_hat = v_val / correction2;

        bias[i] -= lr * m_hat / (v_hat.sqrt() + eps);
    }
}

impl Conv1D {
    pub fn new(in_channels: usize, out_channels: usize, kernel_size: usize, seed_val: u32) -> Self {
        let mut seed = seed_val ^ (in_channels as u32) ^ (out_channels as u32);
        let limit = (6.0 / ((in_channels * kernel_size + out_channels * kernel_size) as f32)).sqrt();
        let mut weights = Array3::zeros((out_channels, in_channels, kernel_size));
        for val in weights.iter_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = (seed as f32) / (u32::MAX as f32);
            *val = (r * 2.0 - 1.0) * limit;
        }
        let bias = Array1::zeros(out_channels);

        Self {
            in_channels,
            out_channels,
            kernel_size,
            weights,
            bias,
            last_input: None,
        }
    }

    pub fn forward(&mut self, x: &Array2<f32>) -> Array2<f32> {
        self.last_input = Some(x.clone());
        self.predict(x)
    }

    pub fn predict(&self, x: &Array2<f32>) -> Array2<f32> {
        let (w, in_c) = x.dim();
        assert_eq!(in_c, self.in_channels, "La dimension de canales de entrada no coincide");
        assert!(w >= self.kernel_size, "La dimension temporal W debe ser mayor o igual al kernel_size");
        
        let w_out = w - self.kernel_size + 1;
        let mut out = Array2::zeros((w_out, self.out_channels));

        for o in 0..self.out_channels {
            let b = self.bias[o];
            for t in 0..w_out {
                let mut sum = 0.0f32;
                for c in 0..self.in_channels {
                    for k in 0..self.kernel_size {
                        sum += x[[t + k, c]] * self.weights[[o, c, k]];
                    }
                }
                out[[t, o]] = sum + b;
            }
        }
        out
    }

    pub fn backward(
        &mut self,
        dout: &Array2<f32>,
        lr: f32,
        wd: f32,
        l1_lambda: f32,
        m_w: &mut Array3<f32>,
        v_w: &mut Array3<f32>,
        m_b: &mut Array1<f32>,
        v_b: &mut Array1<f32>,
        t: f32,
    ) -> Array2<f32> {
        let x = self.last_input.as_ref().expect("last_input debe estar guardado en el pase forward");
        let (w, _in_c) = x.dim();
        let w_out = w - self.kernel_size + 1;
        
        let mut dweights = Array3::zeros(self.weights.raw_dim());
        let mut dbias = Array1::zeros(self.bias.raw_dim());
        let mut dx = Array2::zeros(x.raw_dim());

        for o in 0..self.out_channels {
            let mut sum_bias = 0.0f32;
            for t_idx in 0..w_out {
                let text_val = dout[[t_idx, o]];
                sum_bias += text_val;

                for c in 0..self.in_channels {
                    for k in 0..self.kernel_size {
                        dweights[[o, c, k]] += text_val * x[[t_idx + k, c]];
                        dx[[t_idx + k, c]] += text_val * self.weights[[o, c, k]];
                    }
                }
            }
            dbias[o] = sum_bias;
        }

        // Gradient clipping in-place
        if let Some(dw_slice) = dweights.as_slice_mut() {
            let dw_norm: f32 = dw_slice.iter().map(|&val| val * val).sum::<f32>().sqrt();
            if dw_norm > 1.0 {
                let inv_norm = 1.0 / dw_norm;
                for val in dw_slice.iter_mut() {
                    *val *= inv_norm;
                }
            }

            if let (Some(w_slice), Some(m_slice), Some(v_slice)) = (
                self.weights.as_slice_mut(),
                m_w.as_slice_mut(),
                v_w.as_slice_mut(),
            ) {
                update_adam_param_slice(w_slice, dw_slice, m_slice, v_slice, lr, wd, l1_lambda, t);
            }
        }

        if let (Some(b_slice), Some(db_slice), Some(mb_slice), Some(vb_slice)) = (
            self.bias.as_slice_mut(),
            dbias.as_slice_mut(),
            m_b.as_slice_mut(),
            v_b.as_slice_mut(),
        ) {
            update_adam_bias_slice(b_slice, db_slice, mb_slice, vb_slice, lr, t);
        }

        dx
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Dense {
    pub weights: Array2<f32>,
    pub bias: Array1<f32>,
    #[serde(skip)]
    pub last_input: Option<Array2<f32>>,
}

impl Dense {
    pub fn new(input_dim: usize, output_dim: usize, seed_val: u32) -> Self {
        let mut seed = seed_val ^ (input_dim as u32) ^ (output_dim as u32);
        let limit = (6.0 / ((input_dim + output_dim) as f32)).sqrt();
        let mut weights = Array2::zeros((input_dim, output_dim));
        for val in weights.iter_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = (seed as f32) / (u32::MAX as f32);
            *val = (r * 2.0 - 1.0) * limit;
        }
        let bias = Array1::zeros(output_dim);

        Self {
            weights,
            bias,
            last_input: None,
        }
    }

    pub fn forward(&mut self, x: &Array2<f32>) -> Array2<f32> {
        self.last_input = Some(x.clone());
        self.predict(x)
    }

    pub fn predict(&self, x: &Array2<f32>) -> Array2<f32> {
        let mut out = x.dot(&self.weights);
        for mut row in out.rows_mut() {
            row += &self.bias;
        }
        out
    }

    pub fn backward(
        &mut self,
        dout: &Array2<f32>,
        lr: f32,
        wd: f32,
        l1_lambda: f32,
        m_w: &mut Array2<f32>,
        v_w: &mut Array2<f32>,
        m_b: &mut Array1<f32>,
        v_b: &mut Array1<f32>,
        t: f32,
    ) -> Array2<f32> {
        let x = self.last_input.as_ref().expect("last_input debe estar guardado");
        let mut dw = x.t().dot(dout);
        let mut db = Array1::zeros(self.bias.len());
        for row in dout.rows() {
            db += &row;
        }
        let dx = dout.dot(&self.weights.t());

        // Gradient clipping in-place
        if let Some(dw_slice) = dw.as_slice_mut() {
            let dw_norm: f32 = dw_slice.iter().map(|&val| val * val).sum::<f32>().sqrt();
            if dw_norm > 1.0 {
                let inv_norm = 1.0 / dw_norm;
                for val in dw_slice.iter_mut() {
                    *val *= inv_norm;
                }
            }

            if let (Some(w_slice), Some(m_slice), Some(v_slice)) = (
                self.weights.as_slice_mut(),
                m_w.as_slice_mut(),
                v_w.as_slice_mut(),
            ) {
                update_adam_param_slice(w_slice, dw_slice, m_slice, v_slice, lr, wd, l1_lambda, t);
            }
        }

        if let (Some(b_slice), Some(db_slice), Some(mb_slice), Some(vb_slice)) = (
            self.bias.as_slice_mut(),
            db.as_slice_mut(),
            m_b.as_slice_mut(),
            v_b.as_slice_mut(),
        ) {
            update_adam_bias_slice(b_slice, db_slice, mb_slice, vb_slice, lr, t);
        }

        dx
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Cnn1DModel {
    pub timeframe: String,
    pub input_dim: usize,
    pub window_size: usize,
    pub kernel_size: usize,
    #[serde(default)]
    pub dropout_rate: f32,
    pub conv1: Conv1D,
    pub conv2: Conv1D,
    pub dense: Dense,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Cnn1DOptimizerState {
    pub t: f32,
    pub conv1_m_w: Array3<f32>,
    pub conv1_v_w: Array3<f32>,
    pub conv1_m_b: Array1<f32>,
    pub conv1_v_b: Array1<f32>,
    pub conv2_m_w: Array3<f32>,
    pub conv2_v_w: Array3<f32>,
    pub conv2_m_b: Array1<f32>,
    pub conv2_v_b: Array1<f32>,
    pub dense_m_w: Array2<f32>,
    pub dense_v_w: Array2<f32>,
    pub dense_m_b: Array1<f32>,
    pub dense_v_b: Array1<f32>,
}

impl Cnn1DOptimizerState {
    pub fn new_for_model(model: &Cnn1DModel) -> Self {
        let in_c = model.conv1.in_channels;
        let c1 = model.conv1.out_channels;
        let c2 = model.conv2.out_channels;
        let k = model.kernel_size;
        let dense_in = model.dense.weights.shape()[0];
        let dense_out = model.dense.weights.shape()[1];

        Self {
            t: 0.0,
            conv1_m_w: Array3::zeros((c1, in_c, k)),
            conv1_v_w: Array3::zeros((c1, in_c, k)),
            conv1_m_b: Array1::zeros(c1),
            conv1_v_b: Array1::zeros(c1),
            conv2_m_w: Array3::zeros((c2, c1, k)),
            conv2_v_w: Array3::zeros((c2, c1, k)),
            conv2_m_b: Array1::zeros(c2),
            conv2_v_b: Array1::zeros(c2),
            dense_m_w: Array2::zeros((dense_in, dense_out)),
            dense_v_w: Array2::zeros((dense_in, dense_out)),
            dense_m_b: Array1::zeros(dense_out),
            dense_v_b: Array1::zeros(dense_out),
        }
    }

    pub fn new(input_dim: usize, kernel_size: usize) -> Self {
        // Constructor general por defecto (canales 64 -> 128 -> 3)
        Self {
            t: 0.0,
            conv1_m_w: Array3::zeros((64, input_dim, kernel_size)),
            conv1_v_w: Array3::zeros((64, input_dim, kernel_size)),
            conv1_m_b: Array1::zeros(64),
            conv1_v_b: Array1::zeros(64),
            conv2_m_w: Array3::zeros((128, 64, kernel_size)),
            conv2_v_w: Array3::zeros((128, 64, kernel_size)),
            conv2_m_b: Array1::zeros(128),
            conv2_v_b: Array1::zeros(128),
            dense_m_w: Array2::zeros((128, 3)),
            dense_v_w: Array2::zeros((128, 3)),
            dense_m_b: Array1::zeros(3),
            dense_v_b: Array1::zeros(3),
        }
    }
}

impl Cnn1DModel {
    pub fn new_for_timeframe(timeframe: String, window_size: usize, kernel_size: usize, seed_val: u32) -> Self {
        let spec = crate::models::specs::get_model_spec_for_timeframe(&timeframe);
        Self::new_custom(
            timeframe,
            spec.input_features,
            spec.conv1_channels,
            spec.conv2_channels,
            spec.dense_dim,
            spec.output_dim,
            window_size,
            kernel_size,
            seed_val,
        )
    }

    pub fn new_custom(
        timeframe: String,
        input_dim: usize,
        c1: usize,
        c2: usize,
        dense_dim: usize,
        output_dim: usize,
        window_size: usize,
        kernel_size: usize,
        seed_val: u32,
    ) -> Self {
        let max_viable = (window_size + 1) / 2;
        let actual_kernel = if kernel_size > max_viable {
            let clamped = max_viable.max(1);
            println!("Advertencia: kernel_size {} es inviable para ventana {}. Ajustando a máximo viable: {}", kernel_size, window_size, clamped);
            clamped
        } else {
            kernel_size
        };

        let conv1 = Conv1D::new(input_dim, c1, actual_kernel, seed_val);
        let conv2 = Conv1D::new(c1, c2, actual_kernel, seed_val.wrapping_add(42));
        let dense = Dense::new(dense_dim, output_dim, seed_val.wrapping_add(84));

        Self {
            timeframe,
            input_dim,
            window_size,
            kernel_size: actual_kernel,
            dropout_rate: 0.0,
            conv1,
            conv2,
            dense,
        }
    }

    pub fn new(timeframe: String, input_dim: usize, window_size: usize, kernel_size: usize) -> Self {
        Self::new_with_seed(timeframe, input_dim, window_size, kernel_size, 42)
    }

    pub fn new_with_seed(timeframe: String, input_dim: usize, window_size: usize, kernel_size: usize, seed_val: u32) -> Self {
        let spec = crate::models::specs::get_model_spec_for_timeframe(&timeframe);
        Self::new_custom(
            timeframe,
            input_dim,
            spec.conv1_channels,
            spec.conv2_channels,
            spec.dense_dim,
            spec.output_dim,
            window_size,
            kernel_size,
            seed_val,
        )
    }

    pub fn new_with_intelligence(timeframe: String, window_size: usize, kernel_size: usize, seed_val: u32, intelligence: usize) -> Self {

        let spec = crate::models::specs::get_model_spec_for_intelligence(&timeframe, intelligence);
        Self::new_custom(
            timeframe,
            spec.input_features,
            spec.conv1_channels,
            spec.conv2_channels,
            spec.dense_dim,
            spec.output_dim,
            window_size,
            kernel_size,
            seed_val,
        )
    }

    pub fn total_parameters(&self) -> usize {
        let conv1_w = self.conv1.weights.len();
        let conv1_b = self.conv1.bias.len();
        let conv2_w = self.conv2.weights.len();
        let conv2_b = self.conv2.bias.len();
        let dense_w = self.dense.weights.len();
        let dense_b = self.dense.bias.len();
        conv1_w + conv1_b + conv2_w + conv2_b + dense_w + dense_b
    }

    pub fn layer_parameter_breakdown(&self) -> (usize, usize, usize) {
        let conv1 = self.conv1.weights.len() + self.conv1.bias.len();
        let conv2 = self.conv2.weights.len() + self.conv2.bias.len();
        let dense = self.dense.weights.len() + self.dense.bias.len();
        (conv1, conv2, dense)
    }


    pub fn forward(&mut self, x: &Array2<f32>) -> Array1<f32> {
        let h1 = self.conv1.forward(x);
        let h1_relu = h1.mapv(|v| if v > 0.0 { v } else { 0.0 });
        
        let h2 = self.conv2.forward(&h1_relu);
        let h2_relu = h2.mapv(|v| if v > 0.0 { v } else { 0.0 });

        let (w_gap, channels) = h2_relu.dim();
        let mut gap = Array2::zeros((1, channels));
        for c in 0..channels {
            let mut sum = 0.0f32;
            for t in 0..w_gap {
                sum += h2_relu[[t, c]];
            }
            gap[[0, c]] = sum / (w_gap as f32);
        }

        let logits_mat = self.dense.forward(&gap);
        let logits = logits_mat.row(0).to_owned();

        let max_val = logits.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exp_logits = logits.mapv(|v| (v - max_val).exp());
        let sum_exp = exp_logits.sum();
        exp_logits / sum_exp
    }

    pub fn predict(&self, x: &Array2<f32>) -> Array1<f32> {
        let h1 = self.conv1.predict(x);
        let h1_relu = h1.mapv(|v| if v > 0.0 { v } else { 0.0 });
        
        let h2 = self.conv2.predict(&h1_relu);
        let h2_relu = h2.mapv(|v| if v > 0.0 { v } else { 0.0 });

        let (w_gap, channels) = h2_relu.dim();
        let mut gap = Array2::zeros((1, channels));
        for c in 0..channels {
            let mut sum = 0.0f32;
            for t in 0..w_gap {
                sum += h2_relu[[t, c]];
            }
            gap[[0, c]] = sum / (w_gap as f32);
        }

        let logits_mat = self.dense.predict(&gap);
        let logits = logits_mat.row(0).to_owned();

        let max_val = logits.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exp_logits = logits.mapv(|v| (v - max_val).exp());
        let sum_exp = exp_logits.sum();
        exp_logits / sum_exp
    }

    pub fn train_step(&mut self, x: &Array2<f32>, y: usize, lr: f32, wd: f32, l1_lambda: f32, state: &mut Cnn1DOptimizerState) -> f32 {
        state.t += 1.0;

        let h1 = self.conv1.forward(x);
        let h1_relu = h1.mapv(|v| if v > 0.0 { v } else { 0.0 });
        
        let h2 = self.conv2.forward(&h1_relu);
        let h2_relu = h2.mapv(|v| if v > 0.0 { v } else { 0.0 });

        let (w_gap, channels) = h2_relu.dim();
        let mut gap = Array2::zeros((1, channels));
        for c in 0..channels {
            let mut sum = 0.0f32;
            for t in 0..w_gap {
                sum += h2_relu[[t, c]];
            }
            gap[[0, c]] = sum / (w_gap as f32);
        }

        let mut drop_mask_opt: Option<Array2<f32>> = None;
        let gap_dropped = if self.dropout_rate > 0.0 {
            let mut mask = Array2::ones(gap.raw_dim());
            let p_keep = (1.0 - self.dropout_rate).clamp(0.01, 1.0);
            let scale = 1.0 / p_keep;
            let mut seed = (state.t as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            for c in 0..channels {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let rnd = ((seed >> 32) as f32) / (u32::MAX as f32);
                if rnd > p_keep {
                    mask[[0, c]] = 0.0;
                } else {
                    mask[[0, c]] = scale;
                }
            }
            let res = &gap * &mask;
            drop_mask_opt = Some(mask);
            res
        } else {
            gap.clone()
        };

        let logits_mat = self.dense.forward(&gap_dropped);
        let logits = logits_mat.row(0).to_owned();

        let max_val = logits.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exp_logits = logits.mapv(|v| (v - max_val).exp());
        let sum_exp = exp_logits.sum();
        let probs = exp_logits / sum_exp;

        let eps = 1e-15f32;
        let loss = - (probs[y] + eps).ln();

        let mut dlogits = Array2::zeros((1, 3));
        for i in 0..3 {
            dlogits[[0, i]] = probs[i];
        }
        dlogits[[0, y]] -= 1.0;

        let mut dgap = self.dense.backward(&dlogits, lr, wd, l1_lambda, &mut state.dense_m_w, &mut state.dense_v_w, &mut state.dense_m_b, &mut state.dense_v_b, state.t);

        if let Some(ref mask) = drop_mask_opt {
            dgap = dgap * mask;
        }

        let mut dh2_relu = Array2::zeros(h2_relu.raw_dim());
        for c in 0..channels {
            let val = dgap[[0, c]] / (w_gap as f32);
            for t in 0..w_gap {
                dh2_relu[[t, c]] = val;
            }
        }

        let mut dh2 = Array2::zeros(h2.raw_dim());
        for t in 0..w_gap {
            for c in 0..channels {
                if h2_relu[[t, c]] > 0.0 {
                    dh2[[t, c]] = dh2_relu[[t, c]];
                }
            }
        }

        let dh1_relu = self.conv2.backward(&dh2, lr, wd, l1_lambda, &mut state.conv2_m_w, &mut state.conv2_v_w, &mut state.conv2_m_b, &mut state.conv2_v_b, state.t);

        let (w1, c1) = h1_relu.dim();
        let mut dh1 = Array2::zeros(h1.raw_dim());
        for t in 0..w1 {
            for c in 0..c1 {
                if h1_relu[[t, c]] > 0.0 {
                    dh1[[t, c]] = dh1_relu[[t, c]];
                }
            }
        }

        self.conv1.backward(&dh1, lr, wd, l1_lambda, &mut state.conv1_m_w, &mut state.conv1_v_w, &mut state.conv1_m_b, &mut state.conv1_v_b, state.t);

        loss
    }

    pub fn train_step_smoothed(
        &mut self,
        x: &Array2<f32>,
        y: usize,
        lr: f32,
        wd: f32,
        l1_lambda: f32,
        state: &mut Cnn1DOptimizerState,
        smoothing: f32,
    ) -> f32 {
        state.t += 1.0;

        let h1 = self.conv1.forward(x);
        let h1_relu = h1.mapv(|v| if v > 0.0 { v } else { 0.0 });
        
        let h2 = self.conv2.forward(&h1_relu);
        let h2_relu = h2.mapv(|v| if v > 0.0 { v } else { 0.0 });

        let (w_gap, channels) = h2_relu.dim();
        let mut gap = Array2::zeros((1, channels));
        for c in 0..channels {
            let mut sum = 0.0f32;
            for t in 0..w_gap {
                sum += h2_relu[[t, c]];
            }
            gap[[0, c]] = sum / (w_gap as f32);
        }

        let logits_mat = self.dense.forward(&gap);
        let logits = logits_mat.row(0).to_owned();

        let max_val = logits.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exp_logits = logits.mapv(|v| (v - max_val).exp());
        let sum_exp = exp_logits.sum();
        let probs = exp_logits / sum_exp;

        let eps = 1e-15f32;
        let mut target_probs = Array1::from_elem(3, smoothing / 3.0f32);
        target_probs[y] += 1.0 - smoothing;

        let mut loss = 0.0f32;
        for i in 0..3 {
            loss -= target_probs[i] * (probs[i] + eps).ln();
        }

        let mut dlogits = Array2::zeros((1, 3));
        for i in 0..3 {
            dlogits[[0, i]] = probs[i] - target_probs[i];
        }

        let dgap = self.dense.backward(&dlogits, lr, wd, l1_lambda, &mut state.dense_m_w, &mut state.dense_v_w, &mut state.dense_m_b, &mut state.dense_v_b, state.t);

        let mut dh2_relu = Array2::zeros(h2_relu.raw_dim());
        for c in 0..channels {
            let val = dgap[[0, c]] / (w_gap as f32);
            for t in 0..w_gap {
                dh2_relu[[t, c]] = val;
            }
        }

        let mut dh2 = Array2::zeros(h2.raw_dim());
        for t in 0..w_gap {
            for c in 0..channels {
                if h2_relu[[t, c]] > 0.0 {
                    dh2[[t, c]] = dh2_relu[[t, c]];
                }
            }
        }

        let dh1_relu = self.conv2.backward(&dh2, lr, wd, l1_lambda, &mut state.conv2_m_w, &mut state.conv2_v_w, &mut state.conv2_m_b, &mut state.conv2_v_b, state.t);

        let (w1, c1) = h1_relu.dim();
        let mut dh1 = Array2::zeros(h1.raw_dim());
        for t in 0..w1 {
            for c in 0..c1 {
                if h1_relu[[t, c]] > 0.0 {
                    dh1[[t, c]] = dh1_relu[[t, c]];
                }
            }
        }

        self.conv1.backward(&dh1, lr, wd, l1_lambda, &mut state.conv1_m_w, &mut state.conv1_v_w, &mut state.conv1_m_b, &mut state.conv1_v_b, state.t);

        loss
    }
}
