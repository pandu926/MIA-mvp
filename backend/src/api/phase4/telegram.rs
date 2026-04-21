use super::types::{TelegramConfigResponse, TelegramConfigUpdateRequest, TelegramWebhookUpdate};
use crate::{error::AppError, AppState};
use axum::{extract::State, Json};
use chrono::{DateTime, Utc};

/// POST /api/v1/telegram/webhook
/// Supports:
/// - /mia
/// - /risk <ticker|symbol>
pub async fn telegram_webhook(
    State(state): State<AppState>,
    Json(payload): Json<TelegramWebhookUpdate>,
) -> Result<Json<serde_json::Value>, AppError> {
    let Some(message) = payload.message else {
        return Ok(Json(
            serde_json::json!({"ok": true, "ignored": "no message"}),
        ));
    };
    let text = message.text.unwrap_or_default();
    let lower = text.trim().to_lowercase();

    let reply = if lower == "/mia" {
        let rows: Vec<(i16, String, f64)> = sqlx::query_as(
            r#"
            SELECT rank, token_address, alpha_score::double precision
            FROM alpha_rankings
            WHERE window_end = (SELECT MAX(window_end) FROM alpha_rankings)
            ORDER BY rank ASC
            LIMIT 5
            "#,
        )
        .fetch_all(&state.db)
        .await?;

        if rows.is_empty() {
            "No alpha feed yet. Try again shortly.".to_string()
        } else {
            let mut out = String::from("*MIA Top 5 Alpha*\n");
            for (rank, token, score) in rows {
                out.push_str(&format!("{rank}. `{token}` — {:.2}\n", score));
            }
            out
        }
    } else if lower.starts_with("/risk ") {
        let symbol = text
            .trim()
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_uppercase();
        if symbol.is_empty() {
            "Usage: /risk <ticker>".to_string()
        } else {
            let row: Option<(String, i16, Option<String>)> = sqlx::query_as(
                r#"
                SELECT t.contract_address, rs.composite_score, n.narrative_text
                FROM tokens t
                LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
                LEFT JOIN ai_narratives n ON n.token_address = t.contract_address
                WHERE UPPER(COALESCE(t.symbol, '')) = $1
                ORDER BY t.deployed_at DESC
                LIMIT 1
                "#,
            )
            .bind(&symbol)
            .fetch_optional(&state.db)
            .await?;

            match row {
                Some((token, score, narrative)) => format!(
                    "*Risk {symbol}*\nToken: `{token}`\nScore: {score}/100\n{}",
                    narrative.unwrap_or_else(|| "Narrative not available yet.".to_string())
                ),
                None => format!("Ticker {symbol} not found."),
            }
        }
    } else {
        "Unknown command. Use /mia or /risk <ticker>.".to_string()
    };

    if let (Some(token), Some(_)) = (
        state.config.telegram_bot_token.as_ref(),
        state.config.telegram_chat_id.as_ref(),
    ) {
        let url = format!("https://api.telegram.org/bot{token}/sendMessage");
        let _ = reqwest::Client::new()
            .post(url)
            .json(&serde_json::json!({
                "chat_id": message.chat.id,
                "text": reply,
                "parse_mode": "Markdown",
                "disable_web_page_preview": true
            }))
            .send()
            .await;
    }

    Ok(Json(serde_json::json!({"ok": true})))
}

/// GET /api/v1/telegram/config
pub async fn get_telegram_config(
    State(state): State<AppState>,
) -> Result<Json<TelegramConfigResponse>, AppError> {
    let row: Option<(bool, Option<String>, f64, bool, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT enabled, chat_id, threshold_bnb::double precision, alpha_digest_enabled, updated_at
        FROM telegram_runtime_config
        WHERE id = 1
        "#,
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some((enabled, chat_id, threshold_bnb, alpha_digest_enabled, updated_at)) = row {
        return Ok(Json(TelegramConfigResponse {
            enabled,
            chat_id,
            threshold_bnb,
            alpha_digest_enabled,
            updated_at: Some(updated_at),
        }));
    }

    Ok(Json(TelegramConfigResponse {
        enabled: state.config.telegram_chat_id.is_some(),
        chat_id: state.config.telegram_chat_id.clone(),
        threshold_bnb: 0.5,
        alpha_digest_enabled: true,
        updated_at: None,
    }))
}

/// PUT /api/v1/telegram/config
pub async fn update_telegram_config(
    State(state): State<AppState>,
    Json(payload): Json<TelegramConfigUpdateRequest>,
) -> Result<Json<TelegramConfigResponse>, AppError> {
    let threshold_bnb = payload.threshold_bnb.max(0.0);
    let row: (bool, Option<String>, f64, bool, DateTime<Utc>) = sqlx::query_as(
        r#"
        INSERT INTO telegram_runtime_config (id, enabled, chat_id, threshold_bnb, alpha_digest_enabled, updated_at)
        VALUES (1, $1, $2, $3, $4, NOW())
        ON CONFLICT (id)
        DO UPDATE SET
            enabled = EXCLUDED.enabled,
            chat_id = EXCLUDED.chat_id,
            threshold_bnb = EXCLUDED.threshold_bnb,
            alpha_digest_enabled = EXCLUDED.alpha_digest_enabled,
            updated_at = NOW()
        RETURNING enabled, chat_id, threshold_bnb::double precision, alpha_digest_enabled, updated_at
        "#,
    )
    .bind(payload.enabled)
    .bind(payload.chat_id)
    .bind(threshold_bnb)
    .bind(payload.alpha_digest_enabled)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(TelegramConfigResponse {
        enabled: row.0,
        chat_id: row.1,
        threshold_bnb: row.2,
        alpha_digest_enabled: row.3,
        updated_at: Some(row.4),
    }))
}
