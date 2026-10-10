use super::models::IntelligenceLevelResult;
use crate::metrics::{compute_multicriteria_rankings, CandidateRankScore};

pub fn rank_intelligence_results(results: &mut [IntelligenceLevelResult]) {
    if results.is_empty() {
        return;
    }

    let rank_map = compute_multicriteria_rankings(
        results,
        |item| format!("Level_{}", item.level),
        |item| item.metrics_nom.slope_ratio,
        |item| item.metrics_nom.capital_dd_ratio,
        |item| item.metrics_nom.curve_r2,
        |item| item.metrics_nom.smoothness_score,
    );

    for res in results.iter_mut() {
        let key = format!("Level_{}", res.level);
        if let Some(score) = rank_map.get(&key) {
            res.rank_score = score.clone();
        } else {
            res.rank_score = CandidateRankScore {
                candidate_id: key,
                rank_slope: 999,
                rank_cap_dd: 999,
                rank_r2: 999,
                rank_smoothness: 999,
                weighted_avg_rank: 999.0,
            };
        }
    }

    // Ordenar de menor a mayor promedio ponderado de ranking (el menor rango ponderado es el #1)
    results.sort_by(|a, b| {
        a.rank_score
            .weighted_avg_rank
            .partial_cmp(&b.rank_score.weighted_avg_rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
