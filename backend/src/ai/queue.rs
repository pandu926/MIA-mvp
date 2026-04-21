use anyhow::Result;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::ai::prompts::NarrativePromptData;

// ─── Job type ─────────────────────────────────────────────────────────────────

/// A token that has crossed the buy-threshold and needs AI analysis.
#[derive(Debug, Clone)]
pub struct AiJob {
    pub token_address: String,
    pub prompt_data: NarrativePromptData,
}

// ─── Queue factory ────────────────────────────────────────────────────────────

/// Create a bounded Tokio MPSC channel for AI jobs.
///
/// The buffer_size limits memory usage: if the worker falls behind,
/// `try_send` in the indexer returns `TrySendError::Full` and the job
/// is silently skipped (logged as a warning). This prevents back-pressure
/// from slowing down the block indexer.
pub fn create_queue(buffer_size: usize) -> (mpsc::Sender<AiJob>, mpsc::Receiver<AiJob>) {
    mpsc::channel(buffer_size)
}

// ─── Eligibility check ────────────────────────────────────────────────────────

/// Returns `true` if the token has at least `threshold` buy transactions
/// within the last `window_secs` seconds.
///
/// This is the "smart batching" gate: only tokens with real traction
/// consume LLM credits.
pub async fn check_eligibility(
    pool: &PgPool,
    token_address: &str,
    threshold: u64,
    window_secs: u64,
) -> Result<bool> {
    let (count,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM token_transactions
        WHERE token_address = $1
          AND tx_type = 'buy'
          AND created_at >= NOW() - ($2 * INTERVAL '1 second')
        "#,
    )
    .bind(token_address)
    .bind(window_secs as i64)
    .fetch_one(pool)
    .await?;

    Ok(count >= threshold as i64)
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — pure queue logic (no DB or network)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::prompts::NarrativePromptData;

    fn sample_job(addr: &str) -> AiJob {
        AiJob {
            token_address: addr.to_string(),
            prompt_data: NarrativePromptData {
                token_address: addr.to_string(),
                token_name: Some("TestToken".to_string()),
                token_symbol: Some("TEST".to_string()),
                deployer_address: "0xdeployer".to_string(),
                deployer_trust_grade: "B".to_string(),
                deployer_rug_count: 0,
                deployer_graduated_count: 0,
                holder_count: 20,
                buy_count: 15,
                sell_count: 3,
                volume_bnb: 1.5,
                composite_risk_score: 40,
                risk_category: "medium".to_string(),
                top_holder_concentration_pct: Some(50.0),
                hours_since_deploy: 0.5,
                honeypot_detected: false,
                is_rug: false,
                graduated: false,
            },
        }
    }

    // ── create_queue ──────────────────────────────────────────────────────────

    // RED → GREEN: sender can send, receiver can receive
    #[tokio::test]
    async fn queue_sends_and_receives_job() {
        let (tx, mut rx) = create_queue(10);
        let job = sample_job("0xtoken1");

        tx.send(job.clone()).await.expect("send should succeed");

        let received = rx.recv().await.expect("should receive a job");
        assert_eq!(received.token_address, "0xtoken1");
    }

    // RED → GREEN: bounded queue blocks at capacity (try_send returns Err on full)
    #[tokio::test]
    async fn full_queue_try_send_returns_error() {
        let (tx, _rx) = create_queue(2); // capacity = 2, don't consume

        tx.try_send(sample_job("0xa")).expect("first send ok");
        tx.try_send(sample_job("0xb")).expect("second send ok");
        // Third send should fail — queue is full and no consumer
        let result = tx.try_send(sample_job("0xc"));
        assert!(result.is_err(), "try_send on full queue should return Err");
    }

    // RED → GREEN: multiple jobs preserve FIFO order
    #[tokio::test]
    async fn queue_preserves_fifo_order() {
        let (tx, mut rx) = create_queue(10);

        tx.send(sample_job("0x001")).await.unwrap();
        tx.send(sample_job("0x002")).await.unwrap();
        tx.send(sample_job("0x003")).await.unwrap();

        assert_eq!(rx.recv().await.unwrap().token_address, "0x001");
        assert_eq!(rx.recv().await.unwrap().token_address, "0x002");
        assert_eq!(rx.recv().await.unwrap().token_address, "0x003");
    }

    // RED → GREEN: receiver returns None after all senders are dropped
    #[tokio::test]
    async fn receiver_returns_none_when_all_senders_dropped() {
        let (tx, mut rx) = create_queue(10);
        drop(tx);
        assert!(
            rx.recv().await.is_none(),
            "Should get None after sender dropped"
        );
    }

    // RED → GREEN: AiJob fields are accessible
    #[test]
    fn ai_job_fields_are_accessible() {
        let job = sample_job("0xtest");
        assert_eq!(job.token_address, "0xtest");
        assert_eq!(job.prompt_data.buy_count, 15);
        assert_eq!(job.prompt_data.composite_risk_score, 40);
    }

    // RED → GREEN: clone preserves all fields
    #[test]
    fn ai_job_clone_is_equal() {
        let job = sample_job("0xclone");
        let cloned = job.clone();
        assert_eq!(cloned.token_address, job.token_address);
        assert_eq!(cloned.prompt_data.volume_bnb, job.prompt_data.volume_bnb);
    }
}
