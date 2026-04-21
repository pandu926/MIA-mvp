use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    api::investigation::fetch_token_snapshot,
    error::AppError,
    research::{dossier, heurist::HeuristDossier, linking::LinkedLaunchSummary},
    AppState,
};

use super::{
    planner::{build_research_plan, PlannedStep},
    service::{get_token_symbol_hint, load_cached_report, persist_report, CachedReportRecord},
    tool_registry::{
        get_deployer_memory, get_linked_launch_cluster, get_market_structure,
        get_optional_narrative_context, get_pattern_matches, get_wallet_structure, ToolPayment,
    },
    types::{
        deep_research_provider_label, DeepResearchPaymentLedgerResponse, DeepResearchRunResponse,
        DeepResearchRunStage, DeepResearchRunStatus, DeepResearchRunStepResponse,
        DeepResearchRunTraceResponse, DeepResearchToolCallResponse,
    },
};

const STAGE_PLAN: &str = "plan";
const STAGE_GATHER_INTERNAL: &str = "gather_internal";
const STAGE_GATHER_EXTERNAL: &str = "gather_external";
const STAGE_SYNTHESIZE: &str = "synthesize";
const STAGE_FINALIZE: &str = "finalize";

#[derive(Debug, sqlx::FromRow)]
struct RunRecord {
    id: Uuid,
    token_address: String,
    provider_path: String,
    status: String,
    current_phase: String,
    budget_usage_cents: i32,
    paid_calls_count: i32,
    error_message: Option<String>,
    report_updated_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct RunStepRecord {
    id: i64,
    step_key: String,
    title: String,
    status: String,
    agent_name: Option<String>,
    tool_name: Option<String>,
    summary: Option<String>,
    evidence: Value,
    cost_cents: i32,
    payment_tx: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct ToolCallRecord {
    id: i64,
    step_key: String,
    tool_name: String,
    provider: Option<String>,
    status: String,
    summary: Option<String>,
    evidence: Value,
    latency_ms: Option<i32>,
    cost_cents: i32,
    payment_tx: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct PaymentLedgerRecord {
    id: i64,
    tool_call_id: i64,
    provider: String,
    network: String,
    asset: String,
    amount_units: String,
    amount_display: String,
    tx_hash: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug)]
struct GatheredInputs {
    heurist_dossier: Option<HeuristDossier>,
    dex_context: crate::research::dexscreener::DexScreenerContext,
    wallet_structure: crate::research::launch_intelligence::WalletStructureSummary,
    deployer_memory: Option<crate::research::launch_intelligence::DeployerMemorySummary>,
    linked_launch: Option<LinkedLaunchSummary>,
    pattern_engine: Option<crate::research::pattern_engine::PatternEngineSummary>,
}

fn map_status(raw: &str) -> DeepResearchRunStatus {
    match raw {
        "queued" => DeepResearchRunStatus::Queued,
        "running" => DeepResearchRunStatus::Running,
        "completed" => DeepResearchRunStatus::Completed,
        "failed" => DeepResearchRunStatus::Failed,
        "skipped" => DeepResearchRunStatus::Skipped,
        _ => DeepResearchRunStatus::Failed,
    }
}

fn map_stage(raw: &str) -> DeepResearchRunStage {
    match raw {
        STAGE_PLAN => DeepResearchRunStage::Plan,
        STAGE_GATHER_INTERNAL => DeepResearchRunStage::GatherInternal,
        STAGE_GATHER_EXTERNAL => DeepResearchRunStage::GatherExternal,
        STAGE_SYNTHESIZE => DeepResearchRunStage::Synthesize,
        STAGE_FINALIZE => DeepResearchRunStage::Finalize,
        _ => DeepResearchRunStage::Finalize,
    }
}

fn parse_evidence_list(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn to_step_response(record: RunStepRecord) -> DeepResearchRunStepResponse {
    DeepResearchRunStepResponse {
        id: record.id,
        step_key: record.step_key,
        title: record.title,
        status: map_status(&record.status),
        agent_name: record.agent_name,
        tool_name: record.tool_name,
        summary: record.summary,
        evidence: parse_evidence_list(&record.evidence),
        cost_cents: record.cost_cents.max(0) as u32,
        payment_tx: record.payment_tx,
        started_at: record.started_at,
        completed_at: record.completed_at,
    }
}

fn to_tool_call_response(record: ToolCallRecord) -> DeepResearchToolCallResponse {
    DeepResearchToolCallResponse {
        id: record.id,
        step_key: record.step_key,
        tool_name: record.tool_name,
        provider: record.provider,
        status: map_status(&record.status),
        summary: record.summary,
        evidence: parse_evidence_list(&record.evidence),
        latency_ms: record.latency_ms.map(|value| value.max(0) as u32),
        cost_cents: record.cost_cents.max(0) as u32,
        payment_tx: record.payment_tx,
        created_at: record.created_at,
        completed_at: record.completed_at,
    }
}

fn to_payment_ledger_response(record: PaymentLedgerRecord) -> DeepResearchPaymentLedgerResponse {
    DeepResearchPaymentLedgerResponse {
        id: record.id,
        tool_call_id: record.tool_call_id,
        provider: record.provider,
        network: record.network,
        asset: record.asset,
        amount_units: record.amount_units,
        amount_display: record.amount_display,
        tx_hash: record.tx_hash,
        status: record.status,
        created_at: record.created_at,
    }
}

fn to_run_response(record: RunRecord) -> DeepResearchRunResponse {
    DeepResearchRunResponse {
        run_id: record.id,
        token_address: record.token_address,
        provider_path: record.provider_path,
        status: map_status(&record.status),
        current_phase: map_stage(&record.current_phase),
        budget_usage_cents: record.budget_usage_cents.max(0) as u32,
        paid_calls_count: record.paid_calls_count.max(0) as u32,
        report_ready: record.report_updated_at.is_some() && record.status == "completed",
        error_message: record.error_message,
        created_at: record.created_at,
        started_at: record.started_at,
        completed_at: record.completed_at,
    }
}

async fn insert_run_record(
    db: &PgPool,
    token_address: &str,
    provider_path: &str,
    planner_version: &str,
    execution_mode: &str,
) -> Result<Uuid, AppError> {
    let run_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO deep_research_runs (
            id, token_address, provider_path, status, current_phase, planner_version, execution_mode, started_at
        )
        VALUES ($1, $2, $3, 'running', $4, $5, $6, NOW())
        "#,
    )
    .bind(run_id)
    .bind(token_address)
    .bind(provider_path)
    .bind(STAGE_PLAN)
    .bind(planner_version)
    .bind(execution_mode)
    .execute(db)
    .await?;

    Ok(run_id)
}

async fn insert_step_records(
    db: &PgPool,
    run_id: Uuid,
    steps: &[PlannedStep],
) -> Result<(), AppError> {
    for step in steps {
        let tool_name = if step.tools.is_empty() {
            None
        } else {
            let joined = step
                .tools
                .iter()
                .map(|tool| tool.name)
                .collect::<Vec<_>>()
                .join(", ");
            let compact = if joined.len() <= 64 {
                joined
            } else {
                format!("{} tools scheduled", step.tools.len())
            };

            Some(compact)
        };

        sqlx::query(
            r#"
            INSERT INTO deep_research_run_steps (
                run_id, step_key, title, status, agent_name, tool_name, evidence
            )
            VALUES ($1, $2, $3, 'queued', $4, $5, '[]'::jsonb)
            "#,
        )
        .bind(run_id)
        .bind(step.step_key)
        .bind(step.title)
        .bind(step.agent_name)
        .bind(tool_name)
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn update_run_status(
    db: &PgPool,
    run_id: Uuid,
    status: &str,
    current_phase: &str,
    error_message: Option<&str>,
    completed: bool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE deep_research_runs
        SET status = $2,
            current_phase = $3,
            error_message = $4,
            updated_at = NOW(),
            completed_at = CASE WHEN $5 THEN NOW() ELSE completed_at END
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(status)
    .bind(current_phase)
    .bind(error_message)
    .bind(completed)
    .execute(db)
    .await?;

    Ok(())
}

async fn update_step_status(
    db: &PgPool,
    run_id: Uuid,
    step_key: &str,
    status: &str,
    summary: Option<&str>,
    evidence: &[String],
    completed: bool,
) -> Result<(), AppError> {
    let evidence_json =
        serde_json::to_value(evidence).map_err(|err| AppError::Internal(err.into()))?;

    sqlx::query(
        r#"
        UPDATE deep_research_run_steps
        SET status = $3,
            summary = COALESCE($4, summary),
            evidence = $5,
            started_at = CASE WHEN status = 'queued' AND $3 = 'running' THEN NOW() ELSE started_at END,
            completed_at = CASE WHEN $6 THEN NOW() ELSE completed_at END,
            updated_at = NOW()
        WHERE run_id = $1
          AND step_key = $2
        "#,
    )
    .bind(run_id)
    .bind(step_key)
    .bind(status)
    .bind(summary)
    .bind(evidence_json)
    .bind(completed)
    .execute(db)
    .await?;

    Ok(())
}

async fn insert_tool_call(
    db: &PgPool,
    run_id: Uuid,
    step_key: &str,
    tool_name: &str,
    provider: Option<&str>,
) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO deep_research_tool_calls (
            run_id, step_key, tool_name, provider, status
        )
        VALUES ($1, $2, $3, $4, 'running')
        RETURNING id
        "#,
    )
    .bind(run_id)
    .bind(step_key)
    .bind(tool_name)
    .bind(provider)
    .fetch_one(db)
    .await?;

    Ok(row.0)
}

async fn complete_tool_call(
    db: &PgPool,
    tool_call_id: i64,
    status: &str,
    summary: &str,
    evidence: &[String],
    latency_ms: u32,
) -> Result<(), AppError> {
    let evidence_json =
        serde_json::to_value(evidence).map_err(|err| AppError::Internal(err.into()))?;

    sqlx::query(
        r#"
        UPDATE deep_research_tool_calls
        SET status = $2,
            summary = $3,
            evidence = $4,
            latency_ms = $5,
            completed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(tool_call_id)
    .bind(status)
    .bind(summary)
    .bind(evidence_json)
    .bind(latency_ms as i32)
    .execute(db)
    .await?;

    Ok(())
}

async fn attach_tool_payment(
    db: &PgPool,
    run_id: Uuid,
    step_key: &str,
    tool_call_id: i64,
    payment: &ToolPayment,
) -> Result<(), AppError> {
    let cost_cents = payment.cost_cents as i32;

    sqlx::query(
        r#"
        UPDATE deep_research_tool_calls
        SET cost_cents = $2,
            payment_tx = $3
        WHERE id = $1
        "#,
    )
    .bind(tool_call_id)
    .bind(cost_cents)
    .bind(payment.payment_tx.as_deref())
    .execute(db)
    .await?;

    sqlx::query(
        r#"
        UPDATE deep_research_run_steps
        SET cost_cents = cost_cents + $3,
            payment_tx = COALESCE($4, payment_tx),
            updated_at = NOW()
        WHERE run_id = $1
          AND step_key = $2
        "#,
    )
    .bind(run_id)
    .bind(step_key)
    .bind(cost_cents)
    .bind(payment.payment_tx.as_deref())
    .execute(db)
    .await?;

    sqlx::query(
        r#"
        UPDATE deep_research_runs
        SET budget_usage_cents = budget_usage_cents + $2,
            paid_calls_count = paid_calls_count + 1,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(cost_cents)
    .execute(db)
    .await?;

    Ok(())
}

async fn insert_payment_ledger(
    db: &PgPool,
    run_id: Uuid,
    tool_call_id: i64,
    provider: &str,
    payment: &ToolPayment,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO deep_research_payment_ledger (
            run_id, tool_call_id, provider, amount_units, amount_display, asset, network, tx_hash, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'completed')
        "#,
    )
    .bind(run_id)
    .bind(tool_call_id)
    .bind(provider)
    .bind(&payment.amount_units)
    .bind(&payment.amount_display)
    .bind(&payment.asset)
    .bind(&payment.network)
    .bind(payment.payment_tx.as_deref())
    .execute(db)
    .await?;

    Ok(())
}

async fn fail_tool_call(
    db: &PgPool,
    tool_call_id: i64,
    summary: &str,
    evidence: &[String],
    latency_ms: u32,
) -> Result<(), AppError> {
    complete_tool_call(db, tool_call_id, "failed", summary, evidence, latency_ms).await
}

async fn mark_run_completed(
    db: &PgPool,
    run_id: Uuid,
    report: &CachedReportRecord,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE deep_research_runs
        SET status = 'completed',
            current_phase = $2,
            report_updated_at = $3,
            updated_at = NOW(),
            completed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(STAGE_FINALIZE)
    .bind(report.updated_at)
    .execute(db)
    .await?;

    Ok(())
}

async fn fail_run(
    db: &PgPool,
    run_id: Uuid,
    step_key: &str,
    message: &str,
) -> Result<(), AppError> {
    update_step_status(
        db,
        run_id,
        step_key,
        "failed",
        Some(message),
        &[message.to_string()],
        true,
    )
    .await?;
    update_run_status(db, run_id, "failed", step_key, Some(message), true).await
}

async fn execute_market_structure(
    state: &AppState,
    run_id: Uuid,
    step_key: &str,
    token_address: &str,
) -> Result<crate::research::dexscreener::DexScreenerContext, AppError> {
    let started = Instant::now();
    let tool_call_id = insert_tool_call(
        &state.db,
        run_id,
        step_key,
        "get_market_structure",
        Some("dexscreener-search"),
    )
    .await?;

    match get_market_structure(token_address).await {
        Ok(result) => {
            let mut evidence = result.evidence.clone();
            evidence.push(format!("Tool: {}.", result.tool_name));
            evidence.push(format!("Provider: {}.", result.provider));
            complete_tool_call(
                &state.db,
                tool_call_id,
                "completed",
                &result.summary,
                &evidence,
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Ok(result.payload)
        }
        Err(err) => {
            let message = err.to_string();
            fail_tool_call(
                &state.db,
                tool_call_id,
                "Market structure tool failed.",
                &[message.clone()],
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Err(err)
        }
    }
}

async fn execute_wallet_structure(
    state: &AppState,
    run_id: Uuid,
    step_key: &str,
    token_snapshot: &crate::api::investigation::TokenSnapshot,
) -> Result<crate::research::launch_intelligence::WalletStructureSummary, AppError> {
    let started = Instant::now();
    let tool_call_id = insert_tool_call(
        &state.db,
        run_id,
        step_key,
        "get_wallet_structure",
        Some("mia_internal_wallet_graph"),
    )
    .await?;

    match get_wallet_structure(&state.db, token_snapshot).await {
        Ok(result) => {
            let mut evidence = result.evidence.clone();
            evidence.push(format!("Tool: {}.", result.tool_name));
            evidence.push(format!("Provider: {}.", result.provider));
            complete_tool_call(
                &state.db,
                tool_call_id,
                "completed",
                &result.summary,
                &evidence,
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Ok(result.payload)
        }
        Err(err) => {
            let message = err.to_string();
            fail_tool_call(
                &state.db,
                tool_call_id,
                "Wallet structure tool failed.",
                &[message.clone()],
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Err(err)
        }
    }
}

async fn execute_deployer_memory(
    state: &AppState,
    run_id: Uuid,
    step_key: &str,
    token_snapshot: &crate::api::investigation::TokenSnapshot,
) -> Result<Option<crate::research::launch_intelligence::DeployerMemorySummary>, AppError> {
    let started = Instant::now();
    let tool_call_id = insert_tool_call(
        &state.db,
        run_id,
        step_key,
        "get_deployer_memory",
        Some("mia_internal_history"),
    )
    .await?;

    match get_deployer_memory(&state.db, token_snapshot).await {
        Ok(result) => {
            let mut evidence = result.evidence.clone();
            evidence.push(format!("Tool: {}.", result.tool_name));
            evidence.push(format!("Provider: {}.", result.provider));
            complete_tool_call(
                &state.db,
                tool_call_id,
                "completed",
                &result.summary,
                &evidence,
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Ok(result.payload)
        }
        Err(err) => {
            let message = err.to_string();
            fail_tool_call(
                &state.db,
                tool_call_id,
                "Deployer memory tool failed.",
                &[message.clone()],
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Err(err)
        }
    }
}

async fn execute_linked_cluster(
    state: &AppState,
    run_id: Uuid,
    step_key: &str,
    token_address: &str,
) -> Result<Option<LinkedLaunchSummary>, AppError> {
    let started = Instant::now();
    let tool_call_id = insert_tool_call(
        &state.db,
        run_id,
        step_key,
        "get_linked_launch_cluster",
        Some("mia_internal_linking"),
    )
    .await?;

    match get_linked_launch_cluster(&state.db, token_address).await {
        Ok(result) => {
            let mut evidence = result.evidence.clone();
            evidence.push(format!("Tool: {}.", result.tool_name));
            evidence.push(format!("Provider: {}.", result.provider));
            complete_tool_call(
                &state.db,
                tool_call_id,
                "completed",
                &result.summary,
                &evidence,
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Ok(result.payload)
        }
        Err(err) => {
            let message = err.to_string();
            fail_tool_call(
                &state.db,
                tool_call_id,
                "Linked launch tool failed.",
                &[message.clone()],
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Err(err)
        }
    }
}

async fn execute_pattern_matches(
    state: &AppState,
    run_id: Uuid,
    step_key: &str,
    token_address: &str,
) -> Result<Option<crate::research::pattern_engine::PatternEngineSummary>, AppError> {
    let started = Instant::now();
    let tool_call_id = insert_tool_call(
        &state.db,
        run_id,
        step_key,
        "get_pattern_matches",
        Some("mia_pattern_engine"),
    )
    .await?;

    match get_pattern_matches(&state.db, token_address).await {
        Ok(result) => {
            let mut evidence = result.evidence.clone();
            evidence.push(format!("Tool: {}.", result.tool_name));
            evidence.push(format!("Provider: {}.", result.provider));
            complete_tool_call(
                &state.db,
                tool_call_id,
                if result.payload.is_some() {
                    "completed"
                } else {
                    "skipped"
                },
                &result.summary,
                &evidence,
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Ok(result.payload)
        }
        Err(err) => {
            let message = err.to_string();
            fail_tool_call(
                &state.db,
                tool_call_id,
                "Pattern match tool failed.",
                &[message.clone()],
                started.elapsed().as_millis() as u32,
            )
            .await?;
            Err(err)
        }
    }
}

async fn execute_optional_narrative(
    state: &AppState,
    run_id: Uuid,
    step_key: &str,
    token_address: &str,
    symbol_hint: &str,
) -> Result<Option<HeuristDossier>, AppError> {
    let started = Instant::now();
    let provider = match state.config.deep_research_provider {
        crate::config::DeepResearchProvider::HeuristMeshX402 => "heurist_mesh",
        crate::config::DeepResearchProvider::NativeXApi => "native_x_api_reserved",
    };
    let tool_call_id = insert_tool_call(
        &state.db,
        run_id,
        step_key,
        "get_optional_narrative_context",
        Some(provider),
    )
    .await?;

    let result = get_optional_narrative_context(&state.config, token_address, symbol_hint).await?;
    let mut evidence = result.evidence.clone();
    evidence.push(format!("Tool: {}.", result.tool_name));
    evidence.push(format!("Provider: {}.", result.provider));

    for payment in &result.payments {
        attach_tool_payment(&state.db, run_id, step_key, tool_call_id, payment).await?;
        insert_payment_ledger(&state.db, run_id, tool_call_id, &result.provider, payment).await?;
        evidence.push(format!(
            "Upstream payment settled: {} on {} via {} / {}.",
            payment.amount_display, payment.network, result.provider, payment.asset
        ));
        if let Some(tx) = &payment.payment_tx {
            evidence.push(format!("Payment tx: {tx}."));
        }
    }

    complete_tool_call(
        &state.db,
        tool_call_id,
        if result.payload.is_some() {
            "completed"
        } else {
            "skipped"
        },
        &result.summary,
        &evidence,
        started.elapsed().as_millis() as u32,
    )
    .await?;

    Ok(result.payload)
}

async fn execute_synthesize_step(
    state: &AppState,
    run_id: Uuid,
    token_address: &str,
    gathered: &GatheredInputs,
) -> Result<CachedReportRecord, AppError> {
    let provider_path = deep_research_provider_label(state.config.deep_research_provider);

    let started = Instant::now();
    let synth_call_id = insert_tool_call(
        &state.db,
        run_id,
        STAGE_SYNTHESIZE,
        "build_premium_dossier",
        Some("mia_dossier_builder"),
    )
    .await?;

    let artifacts = dossier::build_premium_dossier_artifacts(
        token_address,
        gathered.heurist_dossier.clone(),
        Some(gathered.dex_context.clone()),
        gathered.wallet_structure.clone(),
        gathered.deployer_memory.clone(),
        gathered.linked_launch.clone(),
        gathered.pattern_engine.clone(),
    );

    complete_tool_call(
        &state.db,
        synth_call_id,
        "completed",
        &artifacts.executive_summary,
        &[
            "Premium dossier artifacts built from internal and optional external evidence."
                .to_string(),
        ],
        started.elapsed().as_millis() as u32,
    )
    .await?;

    let persist_started = Instant::now();
    let persist_call_id = insert_tool_call(
        &state.db,
        run_id,
        STAGE_FINALIZE,
        "persist_deep_research_report",
        Some("postgres_report_cache"),
    )
    .await?;

    persist_report(
        &state.db,
        token_address,
        provider_path,
        &artifacts.executive_summary,
        &artifacts.sections,
        &artifacts.citations,
        &artifacts.source_status,
        &artifacts.raw_payload,
    )
    .await?;

    let cached = load_cached_report(&state.db, token_address, provider_path)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("report was not persisted").into()))?;

    complete_tool_call(
        &state.db,
        persist_call_id,
        "completed",
        "Deep Research report persisted to cache.",
        &["Report cache is ready for fetch.".to_string()],
        persist_started.elapsed().as_millis() as u32,
    )
    .await?;

    Ok(cached)
}

pub(crate) async fn create_research_run(
    state: &AppState,
    token_address: &str,
) -> Result<DeepResearchRunResponse, AppError> {
    let provider_path = deep_research_provider_label(state.config.deep_research_provider);
    let plan = build_research_plan(state.config.deep_research_provider);
    let required_tool_count = plan
        .steps
        .iter()
        .flat_map(|step| step.tools.iter())
        .filter(|tool| matches!(tool.requirement, super::planner::ToolRequirement::Required))
        .count();
    let optional_tool_count = plan
        .steps
        .iter()
        .flat_map(|step| step.tools.iter())
        .filter(|tool| matches!(tool.requirement, super::planner::ToolRequirement::Optional))
        .count();
    let run_id = insert_run_record(
        &state.db,
        token_address,
        provider_path,
        plan.planner_version,
        plan.execution_mode,
    )
    .await?;
    insert_step_records(&state.db, run_id, &plan.steps).await?;

    update_step_status(
        &state.db,
        run_id,
        STAGE_PLAN,
        "completed",
        Some("Planner created a stable internal research plan."),
        &[
            format!("Planner version: {}.", plan.planner_version),
            format!("Execution mode: {}.", plan.execution_mode),
            format!("Planned steps: {}.", plan.steps.len()),
            format!("Required tools: {}.", required_tool_count),
            format!("Optional tools: {}.", optional_tool_count),
        ],
        true,
    )
    .await?;

    let token_snapshot = match fetch_token_snapshot(state, token_address).await {
        Ok(snapshot) => snapshot,
        Err(err) => {
            let message = format!("Unable to resolve token snapshot for Deep Research: {err}");
            fail_run(&state.db, run_id, STAGE_PLAN, &message).await?;
            return Err(err);
        }
    };

    let symbol_hint = get_token_symbol_hint(&state.db, token_address).await?;

    update_run_status(
        &state.db,
        run_id,
        "running",
        STAGE_GATHER_INTERNAL,
        None,
        false,
    )
    .await?;
    update_step_status(
        &state.db,
        run_id,
        STAGE_GATHER_INTERNAL,
        "running",
        Some("Executing internal Deep Research tools."),
        &[
            "Run the market structure tool.".to_string(),
            "Run the wallet structure tool.".to_string(),
            "Run the deployer memory tool.".to_string(),
            "Run the linked launch tool.".to_string(),
            "Run the Pattern Match Engine.".to_string(),
        ],
        false,
    )
    .await?;

    let dex_context =
        match execute_market_structure(state, run_id, STAGE_GATHER_INTERNAL, token_address).await {
            Ok(payload) => payload,
            Err(err) => {
                let message = format!("Deep Research internal market structure failed: {err}");
                fail_run(&state.db, run_id, STAGE_GATHER_INTERNAL, &message).await?;
                return Err(err);
            }
        };
    let wallet_structure =
        match execute_wallet_structure(state, run_id, STAGE_GATHER_INTERNAL, &token_snapshot).await
        {
            Ok(payload) => payload,
            Err(err) => {
                let message = format!("Deep Research internal wallet structure failed: {err}");
                fail_run(&state.db, run_id, STAGE_GATHER_INTERNAL, &message).await?;
                return Err(err);
            }
        };
    let deployer_memory = match execute_deployer_memory(
        state,
        run_id,
        STAGE_GATHER_INTERNAL,
        &token_snapshot,
    )
    .await
    {
        Ok(payload) => payload,
        Err(err) => {
            let message = format!("Deep Research internal deployer memory failed: {err}");
            fail_run(&state.db, run_id, STAGE_GATHER_INTERNAL, &message).await?;
            return Err(err);
        }
    };
    let linked_launch =
        match execute_linked_cluster(state, run_id, STAGE_GATHER_INTERNAL, token_address).await {
            Ok(payload) => payload,
            Err(err) => {
                let message = format!("Deep Research linked launch recovery failed: {err}");
                fail_run(&state.db, run_id, STAGE_GATHER_INTERNAL, &message).await?;
                return Err(err);
            }
        };
    let pattern_engine =
        match execute_pattern_matches(state, run_id, STAGE_GATHER_INTERNAL, token_address).await {
            Ok(payload) => payload,
            Err(err) => {
                let message = format!("Deep Research pattern engine failed: {err}");
                fail_run(&state.db, run_id, STAGE_GATHER_INTERNAL, &message).await?;
                return Err(err);
            }
        };

    update_step_status(
        &state.db,
        run_id,
        STAGE_GATHER_INTERNAL,
        "completed",
        Some("Internal Deep Research tools completed."),
        &[
            "Market structure attached.".to_string(),
            "Wallet structure attached.".to_string(),
            "Deployer memory attached.".to_string(),
            "Linked launch evidence attached.".to_string(),
            "Pattern engine attached.".to_string(),
        ],
        true,
    )
    .await?;

    update_run_status(
        &state.db,
        run_id,
        "running",
        STAGE_GATHER_EXTERNAL,
        None,
        false,
    )
    .await?;
    update_step_status(
        &state.db,
        run_id,
        STAGE_GATHER_EXTERNAL,
        "running",
        Some("Evaluating optional external narrative enrichment."),
        &["External narrative is optional for this run.".to_string()],
        false,
    )
    .await?;

    let heurist_dossier = execute_optional_narrative(
        state,
        run_id,
        STAGE_GATHER_EXTERNAL,
        token_address,
        &symbol_hint,
    )
    .await?;

    update_step_status(
        &state.db,
        run_id,
        STAGE_GATHER_EXTERNAL,
        if heurist_dossier.is_some() {
            "completed"
        } else {
            "skipped"
        },
        Some(if heurist_dossier.is_some() {
            "Optional external narrative was attached."
        } else {
            "Optional external narrative was skipped or degraded safely."
        }),
        &[if heurist_dossier.is_some() {
            "Narrative enrichment is available for the dossier.".to_string()
        } else {
            "Run stayed healthy without narrative enrichment.".to_string()
        }],
        true,
    )
    .await?;

    let gathered = GatheredInputs {
        heurist_dossier,
        dex_context,
        wallet_structure,
        deployer_memory,
        linked_launch,
        pattern_engine,
    };

    update_run_status(&state.db, run_id, "running", STAGE_SYNTHESIZE, None, false).await?;
    update_step_status(
        &state.db,
        run_id,
        STAGE_SYNTHESIZE,
        "running",
        Some("Synthesizing the Deep Research dossier."),
        &["Merge internal evidence into the final premium dossier.".to_string()],
        false,
    )
    .await?;

    let cached = match execute_synthesize_step(state, run_id, token_address, &gathered).await {
        Ok(cached) => cached,
        Err(err) => {
            let message = format!("Deep Research synthesis failed: {err}");
            fail_run(&state.db, run_id, STAGE_SYNTHESIZE, &message).await?;
            return Err(err);
        }
    };

    update_step_status(
        &state.db,
        run_id,
        STAGE_SYNTHESIZE,
        "completed",
        Some("Premium dossier assembled successfully."),
        &[cached.executive_summary.clone()],
        true,
    )
    .await?;

    update_step_status(
        &state.db,
        run_id,
        STAGE_FINALIZE,
        "completed",
        Some("Run ledger and report cache are ready."),
        &["Trace, tool ledger, and report cache are all available.".to_string()],
        true,
    )
    .await?;
    mark_run_completed(&state.db, run_id, &cached).await?;

    get_research_run(&state.db, token_address, run_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("research run was not persisted").into()))
}

pub(crate) async fn has_inflight_research_run(
    db: &PgPool,
    token_address: &str,
) -> Result<bool, AppError> {
    let row: (bool,) = sqlx::query_as(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM deep_research_runs
            WHERE LOWER(token_address) = LOWER($1)
              AND status IN ('queued', 'running')
        )
        "#,
    )
    .bind(token_address)
    .fetch_one(db)
    .await?;

    Ok(row.0)
}

pub(crate) async fn get_research_run(
    db: &PgPool,
    token_address: &str,
    run_id: Uuid,
) -> Result<Option<DeepResearchRunResponse>, AppError> {
    let row: Option<RunRecord> = sqlx::query_as(
        r#"
        SELECT
            id,
            token_address,
            provider_path,
            status,
            current_phase,
            budget_usage_cents,
            paid_calls_count,
            error_message,
            report_updated_at,
            created_at,
            started_at,
            completed_at
        FROM deep_research_runs
        WHERE id = $1
          AND LOWER(token_address) = LOWER($2)
        LIMIT 1
        "#,
    )
    .bind(run_id)
    .bind(token_address)
    .fetch_optional(db)
    .await?;

    Ok(row.map(to_run_response))
}

pub(crate) async fn get_research_run_trace(
    db: &PgPool,
    token_address: &str,
    run_id: Uuid,
) -> Result<Option<DeepResearchRunTraceResponse>, AppError> {
    let Some(run) = get_research_run(db, token_address, run_id).await? else {
        return Ok(None);
    };

    let step_rows: Vec<RunStepRecord> = sqlx::query_as(
        r#"
        SELECT
            id,
            step_key,
            title,
            status,
            agent_name,
            tool_name,
            summary,
            evidence,
            cost_cents,
            payment_tx,
            started_at,
            completed_at
        FROM deep_research_run_steps
        WHERE run_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(run_id)
    .fetch_all(db)
    .await?;

    let tool_rows: Vec<ToolCallRecord> = sqlx::query_as(
        r#"
        SELECT
            id,
            step_key,
            tool_name,
            provider,
            status,
            summary,
            evidence,
            latency_ms,
            cost_cents,
            payment_tx,
            created_at,
            completed_at
        FROM deep_research_tool_calls
        WHERE run_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(run_id)
    .fetch_all(db)
    .await?;

    let payment_rows: Vec<PaymentLedgerRecord> = sqlx::query_as(
        r#"
        SELECT
            id,
            tool_call_id,
            provider,
            network,
            asset,
            amount_units,
            amount_display,
            tx_hash,
            status,
            created_at
        FROM deep_research_payment_ledger
        WHERE run_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(run_id)
    .fetch_all(db)
    .await?;

    Ok(Some(DeepResearchRunTraceResponse {
        run_id: run.run_id,
        token_address: run.token_address,
        provider_path: run.provider_path,
        status: run.status,
        current_phase: run.current_phase,
        budget_usage_cents: run.budget_usage_cents,
        paid_calls_count: run.paid_calls_count,
        error_message: run.error_message,
        created_at: run.created_at,
        started_at: run.started_at,
        completed_at: run.completed_at,
        steps: step_rows.into_iter().map(to_step_response).collect(),
        tool_calls: tool_rows.into_iter().map(to_tool_call_response).collect(),
        payment_ledger: payment_rows
            .into_iter()
            .map(to_payment_ledger_response)
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{map_stage, map_status};
    use crate::api::deep_research::types::{DeepResearchRunStage, DeepResearchRunStatus};

    #[test]
    fn status_mapping_stays_api_safe() {
        assert_eq!(map_status("queued"), DeepResearchRunStatus::Queued);
        assert_eq!(map_status("running"), DeepResearchRunStatus::Running);
        assert_eq!(map_status("completed"), DeepResearchRunStatus::Completed);
        assert_eq!(map_status("failed"), DeepResearchRunStatus::Failed);
        assert_eq!(map_status("skipped"), DeepResearchRunStatus::Skipped);
    }

    #[test]
    fn stage_mapping_stays_api_safe() {
        assert_eq!(map_stage("plan"), DeepResearchRunStage::Plan);
        assert_eq!(
            map_stage("gather_internal"),
            DeepResearchRunStage::GatherInternal
        );
        assert_eq!(
            map_stage("gather_external"),
            DeepResearchRunStage::GatherExternal
        );
        assert_eq!(map_stage("synthesize"), DeepResearchRunStage::Synthesize);
        assert_eq!(map_stage("finalize"), DeepResearchRunStage::Finalize);
    }
}
