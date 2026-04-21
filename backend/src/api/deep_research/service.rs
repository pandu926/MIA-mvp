use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    api::investigation::fetch_token_snapshot,
    api::payments::{
        build_payment_required_header_value, build_payment_requirements,
        decode_payment_signature_header, settle_with_facilitator, settlement_success,
        verification_success, verify_with_facilitator,
    },
    config::{Config, DeepResearchProvider, DeepResearchUnlockModel},
    error::AppError,
    research::{dexscreener, dossier, heurist, launch_intelligence, linking, pattern_engine},
    AppState,
};

use super::types::{
    deep_research_provider_label, deep_research_unlock_resource_path, payment_description,
    unlock_resource_url, DeepResearchEntitlementResponse, DeepResearchReportResponse,
    DeepResearchReportSectionResponse,
};

pub(crate) const PAYMENT_REQUIRED_HEADER: &str = "PAYMENT-REQUIRED";
pub(crate) const PAYMENT_SIGNATURE_HEADER: &str = "PAYMENT-SIGNATURE";
pub(crate) const PAYMENT_RESPONSE_HEADER: &str = "PAYMENT-RESPONSE";
pub(crate) const ENTITLEMENT_HEADER: &str = "X-MIA-ENTITLEMENT";

#[derive(Debug)]
pub(crate) struct EntitlementRecord {
    pub entitlement_secret: Uuid,
    pub entitlement_kind: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub(crate) struct CachedReportRecord {
    pub executive_summary: String,
    pub sections: Value,
    pub citations: Value,
    pub source_status: Value,
    pub updated_at: DateTime<Utc>,
}

pub(crate) fn parse_entitlement_from_headers(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get(ENTITLEMENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

pub(crate) async fn load_cached_report(
    db: &PgPool,
    token_address: &str,
    provider_path: &str,
) -> Result<Option<CachedReportRecord>, AppError> {
    let row: Option<(String, Value, Value, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT executive_summary, sections, citations, source_status, updated_at
        FROM deep_research_reports
        WHERE LOWER(token_address) = LOWER($1)
          AND provider_path = $2
        LIMIT 1
        "#,
    )
    .bind(token_address)
    .bind(provider_path)
    .fetch_optional(db)
    .await?;

    Ok(row.map(
        |(executive_summary, sections, citations, source_status, updated_at)| CachedReportRecord {
            executive_summary,
            sections,
            citations,
            source_status,
            updated_at,
        },
    ))
}

pub(crate) async fn persist_report(
    db: &PgPool,
    token_address: &str,
    provider_path: &str,
    executive_summary: &str,
    sections: &Value,
    citations: &Value,
    source_status: &Value,
    raw_payload: &Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO deep_research_reports (
            token_address, provider_path, executive_summary, sections, citations, source_status, raw_payload, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        ON CONFLICT (token_address, provider_path)
        DO UPDATE SET
            executive_summary = EXCLUDED.executive_summary,
            sections = EXCLUDED.sections,
            citations = EXCLUDED.citations,
            source_status = EXCLUDED.source_status,
            raw_payload = EXCLUDED.raw_payload,
            updated_at = NOW()
        "#,
    )
    .bind(token_address)
    .bind(provider_path)
    .bind(executive_summary)
    .bind(sections)
    .bind(citations)
    .bind(source_status)
    .bind(raw_payload)
    .execute(db)
    .await?;

    Ok(())
}

pub(crate) async fn load_active_entitlement(
    db: &PgPool,
    token_address: &str,
    secret: Uuid,
) -> Result<Option<EntitlementRecord>, AppError> {
    let row: Option<(Uuid, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        r#"
        SELECT entitlement_secret, entitlement_kind, expires_at
        FROM entitlements
        WHERE LOWER(token_address) = LOWER($1)
          AND entitlement_secret = $2
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(token_address)
    .bind(secret)
    .fetch_optional(db)
    .await?;

    Ok(
        row.and_then(|(entitlement_secret, entitlement_kind, expires_at)| {
            if expires_at.is_some_and(|expiry| expiry <= Utc::now()) {
                None
            } else {
                Some(EntitlementRecord {
                    entitlement_secret,
                    entitlement_kind,
                    expires_at,
                })
            }
        }),
    )
}

pub(crate) async fn load_entitlement_by_attempt(
    db: &PgPool,
    payment_attempt_id: i64,
) -> Result<Option<EntitlementRecord>, AppError> {
    let row: Option<(Uuid, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        r#"
        SELECT entitlement_secret, entitlement_kind, expires_at
        FROM entitlements
        WHERE payment_attempt_id = $1
          AND status = 'active'
        ORDER BY unlocked_at DESC
        LIMIT 1
        "#,
    )
    .bind(payment_attempt_id)
    .fetch_optional(db)
    .await?;

    Ok(row.map(
        |(entitlement_secret, entitlement_kind, expires_at)| EntitlementRecord {
            entitlement_secret,
            entitlement_kind,
            expires_at,
        },
    ))
}

fn extract_candidate_address(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in ["payer", "from", "walletAddress", "address"] {
                if let Some(Value::String(text)) = map.get(key) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
            for nested in map.values() {
                if let Some(found) = extract_candidate_address(nested) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(extract_candidate_address),
        _ => None,
    }
}

pub(crate) async fn get_token_symbol_hint(
    db: &PgPool,
    token_address: &str,
) -> Result<String, AppError> {
    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        r#"
        SELECT symbol, name
        FROM tokens
        WHERE LOWER(contract_address) = LOWER($1)
        LIMIT 1
        "#,
    )
    .bind(token_address)
    .fetch_optional(db)
    .await?;

    Ok(row
        .and_then(|(symbol, name)| symbol.or(name))
        .unwrap_or_else(|| token_address.to_string()))
}

async fn upsert_payment_attempt(
    db: &PgPool,
    token_address: &str,
    resource_path: &str,
    config: &Config,
    payment_signature_b64: &str,
    payment_payload: &Value,
    payment_requirements: &Value,
) -> Result<i64, AppError> {
    let existing: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT id
        FROM payment_attempts
        WHERE payment_signature_b64 = $1
        LIMIT 1
        "#,
    )
    .bind(payment_signature_b64)
    .fetch_optional(db)
    .await?;

    if let Some((id,)) = existing {
        return Ok(id);
    }

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO payment_attempts (
            token_address, resource_path, unlock_model, provider_path, network,
            price_usdc_cents, payment_signature_b64, payment_payload, payment_requirements, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'required')
        RETURNING id
        "#,
    )
    .bind(token_address)
    .bind(resource_path)
    .bind(config.deep_research_unlock_model.as_str())
    .bind(deep_research_provider_label(config.deep_research_provider))
    .bind(&config.x402_network)
    .bind(config.x402_price_usdc_cents as i32)
    .bind(payment_signature_b64)
    .bind(payment_payload)
    .bind(payment_requirements)
    .fetch_one(db)
    .await?;

    Ok(row.0)
}

async fn mark_payment_attempt_verified(
    db: &PgPool,
    id: i64,
    verify_response: &Value,
    payer_address: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE payment_attempts
        SET status = 'verified',
            verify_response = $2,
            error_message = NULL,
            payer_address = COALESCE($3, payer_address),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(verify_response)
    .bind(payer_address)
    .execute(db)
    .await?;

    Ok(())
}

async fn mark_payment_attempt_settled(
    db: &PgPool,
    id: i64,
    settle_response: &Value,
    payer_address: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE payment_attempts
        SET status = 'settled',
            settle_response = $2,
            error_message = NULL,
            payer_address = COALESCE($3, payer_address),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(settle_response)
    .bind(payer_address)
    .execute(db)
    .await?;

    Ok(())
}

async fn mark_payment_attempt_failed(
    db: &PgPool,
    id: i64,
    error_message: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE payment_attempts
        SET status = 'failed',
            error_message = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(error_message)
    .execute(db)
    .await?;

    Ok(())
}

async fn create_entitlement_for_attempt(
    db: &PgPool,
    token_address: &str,
    config: &Config,
    payment_attempt_id: i64,
    payer_address: Option<&str>,
) -> Result<EntitlementRecord, AppError> {
    if let Some(existing) = load_entitlement_by_attempt(db, payment_attempt_id).await? {
        return Ok(existing);
    }

    let id = Uuid::new_v4();
    let secret = Uuid::new_v4();
    let expires_at = match config.deep_research_unlock_model {
        DeepResearchUnlockModel::UnlockThisReport => None,
        DeepResearchUnlockModel::DayPass => Some(Utc::now() + Duration::hours(24)),
    };
    let kind = match config.deep_research_unlock_model {
        DeepResearchUnlockModel::UnlockThisReport => "report",
        DeepResearchUnlockModel::DayPass => "day_pass",
    };

    sqlx::query(
        r#"
        INSERT INTO entitlements (
            id, token_address, entitlement_kind, entitlement_secret,
            payer_address, payment_attempt_id, status, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'active', $7)
        "#,
    )
    .bind(id)
    .bind(token_address)
    .bind(kind)
    .bind(secret)
    .bind(payer_address)
    .bind(payment_attempt_id)
    .bind(expires_at)
    .execute(db)
    .await?;

    Ok(EntitlementRecord {
        entitlement_secret: secret,
        entitlement_kind: kind.to_string(),
        expires_at,
    })
}

pub(crate) fn build_payment_required_response(
    config: &Config,
    resource_url: &str,
) -> Result<Response, AppError> {
    let pay_to = config.x402_pay_to.as_deref().ok_or_else(|| {
        AppError::FeatureDisabled("X402_PAY_TO is not configured on this deployment.".to_string())
    })?;
    let asset = config.x402_asset_address.as_deref().ok_or_else(|| {
        AppError::FeatureDisabled(
            "X402_ASSET_ADDRESS is not configured on this deployment.".to_string(),
        )
    })?;
    let encoded = build_payment_required_header_value(
        resource_url,
        payment_description(),
        config,
        pay_to,
        asset,
    )
    .map_err(|err| AppError::Internal(err.into()))?;
    let requirements =
        build_payment_requirements(resource_url, payment_description(), config, pay_to, asset);

    let mut response = (StatusCode::PAYMENT_REQUIRED, Json(requirements)).into_response();
    response.headers_mut().insert(
        PAYMENT_REQUIRED_HEADER,
        HeaderValue::from_str(&encoded).map_err(|err| AppError::Internal(err.into()))?,
    );
    Ok(response)
}

pub(crate) fn build_report_response(
    token_address: &str,
    config: &Config,
    cached: CachedReportRecord,
    entitlement: Option<&EntitlementRecord>,
    payment_response: Option<&Value>,
) -> Result<Response, AppError> {
    let sections_value = cached.sections.as_array().cloned().unwrap_or_default();
    let sections = sections_value
        .into_iter()
        .map(|item| DeepResearchReportSectionResponse {
            id: item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("section")
                .to_string(),
            title: item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Section")
                .to_string(),
            summary: item
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            stage: item
                .get("stage")
                .and_then(Value::as_str)
                .unwrap_or("mvp")
                .to_string(),
            source_agent: item
                .get("source_agent")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            confidence: item
                .get("confidence")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            provider: item
                .get("provider")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            source_url: item
                .get("source_url")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            observed_at: item
                .get("observed_at")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            fallback_note: item
                .get("fallback_note")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            evidence: item.get("evidence").and_then(Value::as_array).map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            }),
            related_tokens: item
                .get("related_tokens")
                .and_then(Value::as_array)
                .cloned(),
            repeated_wallets: item
                .get("repeated_wallets")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                }),
            details: item.get("details").cloned(),
        })
        .collect::<Vec<_>>();

    let citations = cached.citations.as_array().cloned().unwrap_or_default();
    let entitlement_payload = entitlement.map(|record| DeepResearchEntitlementResponse {
        access_token: record.entitlement_secret.to_string(),
        kind: record.entitlement_kind.clone(),
        expires_at: record.expires_at,
    });
    let body = Json(DeepResearchReportResponse {
        token_address: token_address.to_string(),
        provider_path: deep_research_provider_label(config.deep_research_provider).to_string(),
        status: "ready".to_string(),
        executive_summary: cached.executive_summary,
        sections,
        citations,
        source_status: cached.source_status,
        generated_at: cached.updated_at,
        entitlement: entitlement_payload,
    });

    let mut response = (StatusCode::OK, body).into_response();
    if let Some(record) = entitlement {
        response.headers_mut().insert(
            ENTITLEMENT_HEADER,
            HeaderValue::from_str(&record.entitlement_secret.to_string())
                .map_err(|err| AppError::Internal(err.into()))?,
        );
    }
    if let Some(settlement) = payment_response {
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_vec(settlement).map_err(|err| AppError::Internal(err.into()))?);
        response.headers_mut().insert(
            PAYMENT_RESPONSE_HEADER,
            HeaderValue::from_str(&encoded).map_err(|err| AppError::Internal(err.into()))?,
        );
    }
    Ok(response)
}

pub(crate) async fn generate_or_load_report(
    state: &AppState,
    token_address: &str,
) -> Result<CachedReportRecord, AppError> {
    let provider = deep_research_provider_label(state.config.deep_research_provider);
    if let Some(cached) = load_cached_report(&state.db, token_address, provider).await? {
        return Ok(cached);
    }

    let token_snapshot = fetch_token_snapshot(state, token_address).await?;
    let symbol_hint = get_token_symbol_hint(&state.db, token_address).await?;
    let heurist_dossier = match state.config.deep_research_provider {
        DeepResearchProvider::HeuristMeshX402 => {
            heurist::run_mvp_dossier(&state.config, token_address, &symbol_hint)
                .await
                .ok()
        }
        DeepResearchProvider::NativeXApi => None,
    };

    let (dex_context, wallet_structure, deployer_memory) = tokio::join!(
        async {
            dexscreener::fetch_pair_context(token_address)
                .await
                .map_err(|err| AppError::NotReady(format!("DexScreener unavailable: {err}")))
        },
        async {
            launch_intelligence::build_wallet_structure_summary(&state.db, &token_snapshot)
                .await
                .map_err(|err| AppError::NotReady(format!("Wallet structure unavailable: {err}")))
        },
        async {
            launch_intelligence::build_deployer_memory_summary(&state.db, &token_snapshot)
                .await
                .map_err(|err| AppError::NotReady(format!("Deployer memory unavailable: {err}")))
        }
    );
    let dex_context = dex_context.ok();
    let wallet_structure = wallet_structure?;
    let deployer_memory = deployer_memory?;

    let linked_launch = linking::build_linked_launch_summary(&state.db, token_address)
        .await
        .map_err(|err| AppError::NotReady(format!("Deep Research linking unavailable: {err}")))?;
    let pattern_summary =
        pattern_engine::load_latest_pattern_engine_summary(&state.db, token_address)
            .await
            .map_err(|err| AppError::NotReady(format!("Pattern engine unavailable: {err}")))?;

    let dossier_artifacts = dossier::build_premium_dossier_artifacts(
        token_address,
        heurist_dossier,
        dex_context,
        wallet_structure,
        deployer_memory,
        linked_launch,
        pattern_summary,
    );

    persist_report(
        &state.db,
        token_address,
        provider,
        &dossier_artifacts.executive_summary,
        &dossier_artifacts.sections,
        &dossier_artifacts.citations,
        &dossier_artifacts.source_status,
        &dossier_artifacts.raw_payload,
    )
    .await?;

    load_cached_report(&state.db, token_address, provider)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("report was not persisted").into()))
}

pub(crate) async fn verify_payment_and_issue_entitlement(
    state: &AppState,
    token_address: &str,
    headers: &HeaderMap,
) -> Result<(EntitlementRecord, Value), AppError> {
    let payment_header = headers
        .get(PAYMENT_SIGNATURE_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            AppError::PaymentRequired("PAYMENT-SIGNATURE header is required.".to_string())
        })?;

    let payment_payload = decode_payment_signature_header(payment_header).map_err(|err| {
        AppError::PaymentRequired(format!("invalid PAYMENT-SIGNATURE header: {err}"))
    })?;
    let pay_to = state.config.x402_pay_to.as_deref().ok_or_else(|| {
        AppError::FeatureDisabled("X402_PAY_TO is not configured on this deployment.".to_string())
    })?;
    let asset = state.config.x402_asset_address.as_deref().ok_or_else(|| {
        AppError::FeatureDisabled(
            "X402_ASSET_ADDRESS is not configured on this deployment.".to_string(),
        )
    })?;
    let resource_url = payment_payload
        .get("resource")
        .and_then(|value| value.get("url"))
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .unwrap_or_else(|| unlock_resource_url(&state.config, token_address));
    let payment_requirements = build_payment_requirements(
        &resource_url,
        payment_description(),
        &state.config,
        pay_to,
        asset,
    );
    let attempt_id = upsert_payment_attempt(
        &state.db,
        token_address,
        &deep_research_unlock_resource_path(token_address),
        &state.config,
        payment_header,
        &payment_payload,
        &payment_requirements,
    )
    .await?;

    if let Some(existing) = load_entitlement_by_attempt(&state.db, attempt_id).await? {
        return Ok((existing, json!({"reused": true})));
    }

    let verify_response =
        verify_with_facilitator(&state.config, &payment_payload, &payment_requirements)
            .await
            .map_err(|err| AppError::PaymentRequired(format!("x402 verify failed: {err}")))?;
    if !verification_success(&verify_response) {
        mark_payment_attempt_failed(
            &state.db,
            attempt_id,
            "Facilitator rejected the payment payload during verification.",
        )
        .await?;
        return Err(AppError::PaymentRequired(
            "Payment verification did not succeed.".to_string(),
        ));
    }

    let payer_address = extract_candidate_address(&verify_response)
        .or_else(|| extract_candidate_address(&payment_payload));
    mark_payment_attempt_verified(
        &state.db,
        attempt_id,
        &verify_response,
        payer_address.as_deref(),
    )
    .await?;

    let settle_response =
        settle_with_facilitator(&state.config, &payment_payload, &payment_requirements)
            .await
            .map_err(|err| AppError::PaymentRequired(format!("x402 settle failed: {err}")))?;
    if !settlement_success(&settle_response) {
        mark_payment_attempt_failed(
            &state.db,
            attempt_id,
            "Facilitator did not confirm settlement.",
        )
        .await?;
        return Err(AppError::PaymentRequired(
            "Payment settlement did not succeed.".to_string(),
        ));
    }

    mark_payment_attempt_settled(
        &state.db,
        attempt_id,
        &settle_response,
        payer_address.as_deref(),
    )
    .await?;

    let entitlement = create_entitlement_for_attempt(
        &state.db,
        token_address,
        &state.config,
        attempt_id,
        payer_address.as_deref(),
    )
    .await?;

    Ok((entitlement, settle_response))
}
