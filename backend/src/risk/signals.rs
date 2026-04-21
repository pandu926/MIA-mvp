/// Risk signal functions — all pure, no I/O, no side effects.
///
/// Convention: all functions return a value in [0, 100] where:
///   0   = lowest risk (safest)
///   100 = highest risk (most dangerous)
///
/// This makes testing trivial and the composite scorer deterministic.

/// Signal 1 (weight 25%): Deployer historical track record.
///
/// Grade table:
///   0 rugs + ≥3 graduations → 0  (trusted serial deployer)
///   0 rugs + ≥1 graduation  → 10 (clean but unproven)
///   0 rugs + 0 graduations  → 20 (new deployer, no history)
///   1 rug                   → 35–50 (depends on grads as mitigation)
///   2 rugs                  → 70
///   3 rugs                  → 85
///   4+ rugs                 → 100 (blacklisted tier)
pub fn deployer_history_score(rug_count: u32, graduation_count: u32) -> u8 {
    match rug_count {
        0 if graduation_count >= 3 => 0,
        0 if graduation_count >= 1 => 10,
        0 => 20,
        1 => {
            // Each graduation reduces single-rug penalty (max 15 points reduction)
            let mitigation = (graduation_count * 5).min(15) as u8;
            50u8.saturating_sub(mitigation)
        }
        2 => 70,
        3 => 85,
        _ => 100,
    }
}

/// Signal 2 (weight 20%): Percentage of liquidity that is locked.
///
///   100% locked → 0  (safe — devs can't rug liquidity)
///   50% locked  → 50
///   0% locked   → 100 (fully unlocked, rug-pull risk)
pub fn liquidity_lock_score(locked_pct: f64) -> u8 {
    let locked = locked_pct.clamp(0.0, 100.0);
    (100.0 - locked) as u8
}

/// Signal 3 (weight 20%): Top-10 wallet supply concentration.
///
///   10% concentration → 10  (well distributed)
///   50% concentration → 50  (moderate)
///   90%+ concentration → 90+ (likely coordinated hold)
pub fn wallet_concentration_score(top10_pct: f64) -> u8 {
    top10_pct.clamp(0.0, 100.0) as u8
}

/// Signal 4 (weight 15%): Buy/sell velocity ratio.
///
/// High sell pressure is a risk signal:
///   all buys (sell_ratio = 0)  → 0  (healthy momentum)
///   50/50 ratio                → 50
///   all sells (sell_ratio = 1) → 100 (exit pressure)
///   no transactions            → 50 (neutral, insufficient data)
pub fn buy_sell_velocity_score(buy_count: u64, sell_count: u64) -> u8 {
    let total = buy_count + sell_count;
    if total == 0 {
        return 50;
    }
    let sell_ratio = sell_count as f64 / total as f64;
    (sell_ratio * 100.0) as u8
}

/// Signal 5 (weight 10%): Contract audit flags.
///
/// Honeypot is an instant 100 — users cannot sell.
/// Blacklist and mint functions add risk but are not instant disqualifiers.
pub fn contract_audit_score(has_honeypot: bool, has_blacklist: bool, has_mint: bool) -> u8 {
    if has_honeypot {
        return 100;
    }
    let mut score = 0u8;
    if has_blacklist {
        score = score.saturating_add(40);
    }
    if has_mint {
        score = score.saturating_add(30);
    }
    score
}

/// Signal 6 (weight 5%): Social channel bot ratio.
///
/// Pass `None` when social data is unavailable (Phase 2 default: neutral 50).
///   0% bots → 0   (organic community)
///   50% bots → 50
///   100% bots → 100 (manufactured hype)
pub fn social_authenticity_score(bot_ratio: Option<f64>) -> u8 {
    match bot_ratio {
        None => 50, // no social data available yet
        Some(r) => (r.clamp(0.0, 1.0) * 100.0) as u8,
    }
}

/// Signal 7 (weight 5%): Volume consistency (detects pump patterns).
///
/// Uses Coefficient of Variation (CV = std_dev / mean):
///   CV ≈ 0   → stable organic growth → low score
///   CV > 2.0 → extremely spiky       → max score (100)
///
/// Returns 50 (neutral) when fewer than 3 data points.
pub fn volume_consistency_score(volumes: &[f64]) -> u8 {
    if volumes.len() < 3 {
        return 50;
    }

    let mean = volumes.iter().sum::<f64>() / volumes.len() as f64;
    if mean == 0.0 {
        return 50;
    }

    let variance = volumes.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / volumes.len() as f64;

    let cv = variance.sqrt() / mean;
    // Normalize: CV of 2.0 maps to score 100
    ((cv / 2.0) * 100.0).clamp(0.0, 100.0) as u8
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — written to define expected behavior before implementation
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── deployer_history_score ───────────────────────────────────────────────

    #[test]
    fn trusted_deployer_scores_zero() {
        assert_eq!(deployer_history_score(0, 5), 0);
    }

    #[test]
    fn clean_deployer_with_one_grad_scores_ten() {
        assert_eq!(deployer_history_score(0, 1), 10);
    }

    #[test]
    fn new_deployer_no_history_scores_twenty() {
        assert_eq!(deployer_history_score(0, 0), 20);
    }

    #[test]
    fn one_rug_no_grads_scores_fifty() {
        assert_eq!(deployer_history_score(1, 0), 50);
    }

    #[test]
    fn one_rug_with_grads_has_reduced_score() {
        let score_no_grads = deployer_history_score(1, 0);
        let score_with_grads = deployer_history_score(1, 3);
        assert!(
            score_with_grads < score_no_grads,
            "Graduations should reduce penalty"
        );
    }

    #[test]
    fn two_rugs_scores_seventy() {
        assert_eq!(deployer_history_score(2, 0), 70);
    }

    #[test]
    fn three_rugs_scores_eighty_five() {
        assert_eq!(deployer_history_score(3, 0), 85);
    }

    #[test]
    fn serial_rugger_scores_hundred() {
        assert_eq!(deployer_history_score(10, 0), 100);
    }

    // ── liquidity_lock_score ─────────────────────────────────────────────────

    #[test]
    fn fully_locked_liquidity_scores_zero() {
        assert_eq!(liquidity_lock_score(100.0), 0);
    }

    #[test]
    fn zero_locked_liquidity_scores_hundred() {
        assert_eq!(liquidity_lock_score(0.0), 100);
    }

    #[test]
    fn half_locked_liquidity_scores_fifty() {
        assert_eq!(liquidity_lock_score(50.0), 50);
    }

    #[test]
    fn liquidity_score_clamps_above_100() {
        assert_eq!(liquidity_lock_score(120.0), 0);
    }

    #[test]
    fn liquidity_score_clamps_below_zero() {
        assert_eq!(liquidity_lock_score(-10.0), 100);
    }

    // ── wallet_concentration_score ───────────────────────────────────────────

    #[test]
    fn low_concentration_scores_low() {
        assert!(wallet_concentration_score(10.0) <= 10);
    }

    #[test]
    fn high_concentration_scores_high() {
        assert!(wallet_concentration_score(90.0) >= 90);
    }

    #[test]
    fn concentration_clamps_to_100() {
        assert_eq!(wallet_concentration_score(150.0), 100);
    }

    // ── buy_sell_velocity_score ──────────────────────────────────────────────

    #[test]
    fn all_buys_scores_zero() {
        assert_eq!(buy_sell_velocity_score(100, 0), 0);
    }

    #[test]
    fn all_sells_scores_hundred() {
        assert_eq!(buy_sell_velocity_score(0, 100), 100);
    }

    #[test]
    fn equal_buys_sells_scores_fifty() {
        assert_eq!(buy_sell_velocity_score(50, 50), 50);
    }

    #[test]
    fn no_transactions_scores_fifty_neutral() {
        assert_eq!(buy_sell_velocity_score(0, 0), 50);
    }

    // ── contract_audit_score ─────────────────────────────────────────────────

    #[test]
    fn no_flags_scores_zero() {
        assert_eq!(contract_audit_score(false, false, false), 0);
    }

    #[test]
    fn honeypot_scores_hundred_immediately() {
        assert_eq!(contract_audit_score(true, false, false), 100);
    }

    #[test]
    fn honeypot_overrides_other_flags() {
        assert_eq!(contract_audit_score(true, true, true), 100);
    }

    #[test]
    fn blacklist_adds_forty() {
        assert_eq!(contract_audit_score(false, true, false), 40);
    }

    #[test]
    fn mint_function_adds_thirty() {
        assert_eq!(contract_audit_score(false, false, true), 30);
    }

    #[test]
    fn blacklist_plus_mint_adds_seventy() {
        assert_eq!(contract_audit_score(false, true, true), 70);
    }

    // ── social_authenticity_score ────────────────────────────────────────────

    #[test]
    fn no_social_data_scores_fifty_neutral() {
        assert_eq!(social_authenticity_score(None), 50);
    }

    #[test]
    fn zero_bots_scores_zero() {
        assert_eq!(social_authenticity_score(Some(0.0)), 0);
    }

    #[test]
    fn all_bots_scores_hundred() {
        assert_eq!(social_authenticity_score(Some(1.0)), 100);
    }

    #[test]
    fn half_bots_scores_fifty() {
        assert_eq!(social_authenticity_score(Some(0.5)), 50);
    }

    #[test]
    fn bot_ratio_clamps_above_one() {
        assert_eq!(social_authenticity_score(Some(1.5)), 100);
    }

    // ── volume_consistency_score ─────────────────────────────────────────────

    #[test]
    fn insufficient_data_scores_fifty() {
        assert_eq!(volume_consistency_score(&[1.0, 2.0]), 50);
    }

    #[test]
    fn completely_flat_volume_scores_zero() {
        let flat = vec![10.0; 10];
        assert_eq!(volume_consistency_score(&flat), 0);
    }

    #[test]
    fn stable_growing_volume_scores_low() {
        let growing = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(volume_consistency_score(&growing) < 50);
    }

    #[test]
    fn spike_pattern_scores_high() {
        // Flat baseline with one massive pump
        let spike = vec![1.0, 1.0, 1.0, 1.0, 1000.0];
        assert!(volume_consistency_score(&spike) > 50);
    }

    #[test]
    fn all_zero_volume_scores_fifty_neutral() {
        let zeros = vec![0.0, 0.0, 0.0];
        assert_eq!(volume_consistency_score(&zeros), 50);
    }
}
