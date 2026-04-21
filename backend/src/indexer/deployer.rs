use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Trust grade assigned to a deployer based on historical performance.
#[derive(Debug, Clone, PartialEq)]
pub enum TrustGrade {
    /// No rug history, ≥1 graduated token
    A,
    /// No rug history, no graduated tokens (new/neutral)
    B,
    /// 1–2 rugs in history
    C,
    /// 3+ rugs in history
    D,
    /// 5+ rugs or honeypot detected
    F,
}

impl TrustGrade {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustGrade::A => "A",
            TrustGrade::B => "B",
            TrustGrade::C => "C",
            TrustGrade::D => "D",
            TrustGrade::F => "F",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TrustGrade::A => "Trusted",
            TrustGrade::B => "Neutral",
            TrustGrade::C => "Caution",
            TrustGrade::D => "Risky",
            TrustGrade::F => "Dangerous",
        }
    }

    /// Compute TrustGrade from raw counters.
    pub fn from_history(rug_count: i64, graduated_count: i64, honeypot_detected: bool) -> Self {
        if honeypot_detected || rug_count >= 5 {
            TrustGrade::F
        } else if rug_count >= 3 {
            TrustGrade::D
        } else if rug_count >= 1 {
            TrustGrade::C
        } else if graduated_count >= 1 {
            TrustGrade::A
        } else {
            TrustGrade::B
        }
    }
}

/// Intelligence profile for a token deployer address.
#[derive(Debug, Clone)]
pub struct DeployerProfile {
    pub address: String,
    pub total_tokens_deployed: i64,
    pub rug_count: i64,
    pub graduated_count: i64,
    pub honeypot_detected: bool,
    pub trust_grade: TrustGrade,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl DeployerProfile {
    pub fn new(
        address: String,
        total_tokens_deployed: i64,
        rug_count: i64,
        graduated_count: i64,
        honeypot_detected: bool,
        first_seen_at: Option<DateTime<Utc>>,
        last_seen_at: Option<DateTime<Utc>>,
    ) -> Self {
        let trust_grade = TrustGrade::from_history(rug_count, graduated_count, honeypot_detected);
        Self {
            address,
            total_tokens_deployed,
            rug_count,
            graduated_count,
            honeypot_detected,
            trust_grade,
            first_seen_at,
            last_seen_at,
        }
    }
}

/// Fetch deployer intelligence from the database.
///
/// Aggregates all tokens deployed by this address and derives a TrustGrade.
/// Returns `None` if the deployer is unknown (no tokens recorded).
pub async fn get_deployer_profile(
    pool: &PgPool,
    deployer_address: &str,
) -> Result<Option<DeployerProfile>> {
    let row: Option<(
        i64,
        i64,
        i64,
        bool,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint                                   AS total_tokens,
            COALESCE(SUM(CASE WHEN is_rug THEN 1 ELSE 0 END), 0)::bigint AS rug_count,
            COALESCE(SUM(CASE WHEN graduated THEN 1 ELSE 0 END), 0)::bigint AS graduated_count,
            COALESCE(BOOL_OR(honeypot_detected), false)       AS honeypot_detected,
            MIN(deployed_at)                                  AS first_seen_at,
            MAX(deployed_at)                                  AS last_seen_at
        FROM tokens
        WHERE deployer_address = $1
        "#,
    )
    .bind(deployer_address)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(None),
        Some((total, rug_count, graduated_count, honeypot, first_seen, last_seen)) => {
            if total == 0 {
                return Ok(None);
            }
            Ok(Some(DeployerProfile::new(
                deployer_address.to_string(),
                total,
                rug_count,
                graduated_count,
                honeypot,
                first_seen,
                last_seen,
            )))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — pure logic on TrustGrade (no DB needed)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── TrustGrade::from_history ─────────────────────────────────────────────

    #[test]
    fn no_history_new_deployer_is_grade_b() {
        let grade = TrustGrade::from_history(0, 0, false);
        assert_eq!(grade, TrustGrade::B);
    }

    #[test]
    fn no_rugs_with_graduation_is_grade_a() {
        let grade = TrustGrade::from_history(0, 2, false);
        assert_eq!(grade, TrustGrade::A);
    }

    #[test]
    fn one_rug_is_grade_c() {
        let grade = TrustGrade::from_history(1, 0, false);
        assert_eq!(grade, TrustGrade::C);
    }

    #[test]
    fn two_rugs_is_still_grade_c() {
        let grade = TrustGrade::from_history(2, 5, false);
        assert_eq!(grade, TrustGrade::C);
    }

    #[test]
    fn three_rugs_is_grade_d() {
        let grade = TrustGrade::from_history(3, 0, false);
        assert_eq!(grade, TrustGrade::D);
    }

    #[test]
    fn four_rugs_is_grade_d() {
        let grade = TrustGrade::from_history(4, 0, false);
        assert_eq!(grade, TrustGrade::D);
    }

    #[test]
    fn five_rugs_is_grade_f() {
        let grade = TrustGrade::from_history(5, 10, false);
        assert_eq!(grade, TrustGrade::F);
    }

    #[test]
    fn honeypot_detected_overrides_to_grade_f() {
        // Even a deployer with 0 rugs and 10 graduations is F if honeypot detected
        let grade = TrustGrade::from_history(0, 10, true);
        assert_eq!(grade, TrustGrade::F);
    }

    #[test]
    fn honeypot_plus_low_rugs_still_grade_f() {
        let grade = TrustGrade::from_history(1, 0, true);
        assert_eq!(grade, TrustGrade::F);
    }

    // ── as_str / label ───────────────────────────────────────────────────────

    #[test]
    fn trust_grade_as_str_values() {
        assert_eq!(TrustGrade::A.as_str(), "A");
        assert_eq!(TrustGrade::B.as_str(), "B");
        assert_eq!(TrustGrade::C.as_str(), "C");
        assert_eq!(TrustGrade::D.as_str(), "D");
        assert_eq!(TrustGrade::F.as_str(), "F");
    }

    #[test]
    fn trust_grade_label_values() {
        assert_eq!(TrustGrade::A.label(), "Trusted");
        assert_eq!(TrustGrade::B.label(), "Neutral");
        assert_eq!(TrustGrade::C.label(), "Caution");
        assert_eq!(TrustGrade::D.label(), "Risky");
        assert_eq!(TrustGrade::F.label(), "Dangerous");
    }

    // ── DeployerProfile::new ─────────────────────────────────────────────────

    #[test]
    fn deployer_profile_derives_trust_grade_automatically() {
        let profile = DeployerProfile::new("0xdeadbeef".to_string(), 10, 0, 3, false, None, None);
        assert_eq!(profile.trust_grade, TrustGrade::A);
    }

    #[test]
    fn deployer_profile_with_rugs_is_grade_c() {
        let profile = DeployerProfile::new("0xbad".to_string(), 5, 2, 0, false, None, None);
        assert_eq!(profile.trust_grade, TrustGrade::C);
    }

    #[test]
    fn deployer_profile_honeypot_is_grade_f() {
        let profile = DeployerProfile::new("0xevil".to_string(), 3, 0, 1, true, None, None);
        assert_eq!(profile.trust_grade, TrustGrade::F);
        assert_eq!(profile.trust_grade.label(), "Dangerous");
    }
}
