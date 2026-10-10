use crate::db::Kline;

pub fn get_liquidation_distance_pct(leverage: f64) -> f64 {
    let f = if leverage <= 5.0 { 0.05 }
    else if leverage <= 10.0 { 0.08 }
    else if leverage <= 15.0 { 0.10 }
    else if leverage <= 20.0 { 0.12 }
    else if leverage <= 30.0 { 0.15 }
    else if leverage <= 35.0 { 0.18 }
    else if leverage <= 45.0 { 0.20 }
    else if leverage <= 50.0 { 0.25 }
    else if leverage <= 100.0 { 0.30 }
    else { 0.35 };

    ((100.0 / leverage) - ((100.0 / leverage) * f)) - 0.045
}

pub fn parse_date_to_timestamp(input: &str) -> Option<i64> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let parts: Vec<&str> = if cleaned.contains('-') {
        cleaned.split('-').collect()
    } else if cleaned.contains('/') {
        cleaned.split('/').collect()
    } else {
        return None;
    };
    
    if parts.len() != 3 {
        return None;
    }
    
    let day: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let mut year: i32 = parts[2].parse().ok()?;
    
    if year < 100 {
        year += 2000;
    }
    
    use chrono::TimeZone;
    chrono::Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp_millis())
}

pub fn precalculate_volume_sma(klines: &[Kline]) -> Vec<f64> {
    let mut sma = vec![0.0; klines.len()];
    let mut current_sum = 0.0;
    for idx in 0..klines.len() {
        current_sum += klines[idx].volume;
        if idx >= 10000 {
            current_sum -= klines[idx - 10000].volume;
            sma[idx] = current_sum / 10000.0;
        } else {
            sma[idx] = current_sum / (idx + 1) as f64;
        }
    }
    sma
}

pub fn precalculate_macd_hist(klines: &[Kline]) -> Vec<f64> {
    let mut hist = vec![0.0; klines.len()];
    if klines.is_empty() { return hist; }
    
    let mut ema_12 = klines[0].close;
    let mut ema_26 = klines[0].close;
    let mut macd_line = vec![0.0; klines.len()];
    
    let alpha_12 = 2.0 / 13.0;
    let alpha_26 = 2.0 / 27.0;
    let alpha_9 = 2.0 / 10.0;
    
    for i in 0..klines.len() {
        let close = klines[i].close;
        ema_12 = close * alpha_12 + ema_12 * (1.0 - alpha_12);
        ema_26 = close * alpha_26 + ema_26 * (1.0 - alpha_26);
        macd_line[i] = ema_12 - ema_26;
    }
    
    let mut signal = macd_line[0];
    for i in 0..klines.len() {
        signal = macd_line[i] * alpha_9 + signal * (1.0 - alpha_9);
        hist[i] = macd_line[i] - signal;
    }
    hist
}

pub fn precalculate_volume_smas(klines: &[Kline]) -> (Vec<f64>, Vec<f64>) {
    let mut sma5 = vec![0.0; klines.len()];
    let mut sma20 = vec![0.0; klines.len()];
    let mut sum5 = 0.0;
    let mut sum20 = 0.0;
    for i in 0..klines.len() {
        sum5 += klines[i].volume;
        if i >= 5 {
            sum5 -= klines[i - 5].volume;
            sma5[i] = sum5 / 5.0;
        } else {
            sma5[i] = sum5 / (i + 1) as f64;
        }
        
        sum20 += klines[i].volume;
        if i >= 20 {
            sum20 -= klines[i - 20].volume;
            sma20[i] = sum20 / 20.0;
        } else {
            sma20[i] = sum20 / (i + 1) as f64;
        }
    }
    (sma5, sma20)
}

pub fn precalculate_ema_20(klines: &[Kline]) -> Vec<f64> {
    let mut ema = vec![0.0; klines.len()];
    if klines.is_empty() { return ema; }
    
    let mut current_ema = klines[0].close;
    let alpha = 2.0 / 21.0;
    
    for i in 0..klines.len() {
        let close = klines[i].close;
        current_ema = close * alpha + current_ema * (1.0 - alpha);
        ema[i] = current_ema;
    }
    ema
}

pub fn precalculate_ema_200_hl2(klines: &[Kline]) -> Vec<f64> {
    let mut ema = vec![0.0; klines.len()];
    if klines.is_empty() { return ema; }
    
    let mut current_ema = (klines[0].high + klines[0].low) / 2.0;
    let alpha = 2.0 / 201.0;
    
    for i in 0..klines.len() {
        let mid = (klines[i].high + klines[i].low) / 2.0;
        current_ema = mid * alpha + current_ema * (1.0 - alpha);
        ema[i] = current_ema;
    }
    ema
}

pub fn precalculate_atr(klines: &[Kline], n: usize) -> Vec<f64> {
    let mut atr = vec![0.0; klines.len()];
    if klines.is_empty() { return atr; }
    
    let mut tr = vec![0.0; klines.len()];
    for i in 1..klines.len() {
        let kh = klines[i].high;
        let kl = klines[i].low;
        let prev_c = klines[i - 1].close;
        tr[i] = (kh - kl)
            .max((kh - prev_c).abs())
            .max((kl - prev_c).abs());
    }
    tr[0] = klines[0].high - klines[0].low;
    
    let mut current_sum = 0.0;
    for i in 0..klines.len() {
        current_sum += tr[i];
        if i >= n {
            current_sum -= tr[i - n];
            atr[i] = current_sum / n as f64;
        } else {
            atr[i] = current_sum / (i + 1) as f64;
        }
    }
    atr
}

pub fn compute_markov_state(klines: &[Kline], k_idx: usize) -> f64 {
    if k_idx < 19 {
        return 2.0;
    }

    let mut sum_close = 0.0;
    for j in (k_idx - 9)..=k_idx {
        sum_close += klines[j].close;
    }
    let sma_10 = sum_close / 10.0;

    let mut sum_xy = 0.0;
    for i in 0..10 {
        let val = klines[k_idx - 9 + i].close;
        sum_xy += (i as f64 - 4.5) * val;
    }
    let slope = sum_xy / 82.5;

    let trend = if klines[k_idx].close > sma_10 && slope > 0.0 {
        2.0
    } else if klines[k_idx].close < sma_10 && slope < 0.0 {
        0.0
    } else {
        1.0
    };

    let mut tr_sum = 0.0;
    for j in (k_idx - 9)..=k_idx {
        let kh = klines[j].high;
        let kl = klines[j].low;
        let prev_c = klines[j - 1].close;
        let tr = (kh - kl)
            .max((kh - prev_c).abs())
            .max((kl - prev_c).abs());
        tr_sum += tr;
    }
    let current_natr = if klines[k_idx].close > 0.0 { (tr_sum / 10.0) / klines[k_idx].close } else { 0.0 };

    let mut natr_sum_10 = 0.0;
    for offset in 0..10 {
        let target_idx = k_idx - 9 + offset;
        let mut sub_tr_sum = 0.0;
        for j in (target_idx - 9)..=target_idx {
            let kh = klines[j].high;
            let kl = klines[j].low;
            let prev_c = klines[j - 1].close;
            let tr = (kh - kl)
                .max((kh - prev_c).abs())
                .max((kl - prev_c).abs());
            sub_tr_sum += tr;
        }
        let natr_val = if klines[target_idx].close > 0.0 { (sub_tr_sum / 10.0) / klines[target_idx].close } else { 0.0 };
        natr_sum_10 += natr_val;
    }
    let avg_natr_10 = natr_sum_10 / 10.0;

    let volatility = if current_natr > avg_natr_10 { 1.0 } else { 0.0 };

    if trend == 0.0 {
        if volatility == 0.0 { 0.0 } else { 1.0 }
    } else if trend == 1.0 {
        if volatility == 0.0 { 2.0 } else { 3.0 }
    } else {
        if volatility == 0.0 { 4.0 } else { 5.0 }
    }
}
