use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
}

impl TelegramConfig {
    pub fn enabled(&self) -> bool {
        self.bot_token.is_some() && self.chat_id.is_some()
    }
}

pub async fn send_telegram_message(
    http: &Client,
    db: &PgPool,
    cfg: &TelegramConfig,
    message_type: &str,
    text: String,
) -> Result<()> {
    if !cfg.enabled() {
        log_delivery(
            db,
            "telegram",
            message_type,
            "skipped",
            json!({ "text": text }),
            None,
        )
        .await?;
        return Ok(());
    }

    let token = cfg.bot_token.as_deref().unwrap_or_default();
    let chat_id = cfg.chat_id.as_deref().unwrap_or_default();
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let payload = json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "Markdown",
        "disable_web_page_preview": true
    });

    let res = http.post(url).json(&payload).send().await;
    match res {
        Ok(r) if r.status().is_success() => {
            log_delivery(db, "telegram", message_type, "sent", payload, None).await?;
            Ok(())
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            log_delivery(
                db,
                "telegram",
                message_type,
                "failed",
                payload,
                Some(format!("HTTP error: {body}")),
            )
            .await?;
            Ok(())
        }
        Err(e) => {
            log_delivery(
                db,
                "telegram",
                message_type,
                "failed",
                payload,
                Some(e.to_string()),
            )
            .await?;
            Ok(())
        }
    }
}

async fn log_delivery(
    db: &PgPool,
    channel: &str,
    message_type: &str,
    status: &str,
    payload: serde_json::Value,
    error_message: Option<String>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO telegram_delivery_logs (channel, message_type, status, payload, error_message)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(channel)
    .bind(message_type)
    .bind(status)
    .bind(payload)
    .bind(error_message)
    .execute(db)
    .await?;
    Ok(())
}
