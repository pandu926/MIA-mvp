use serde_json::json;

use crate::{
    api::investigation::TokenSnapshot,
    config::{Config, DeepResearchProvider},
    error::AppError,
    research::{
        dexscreener::{self, DexScreenerContext},
        heurist::HeuristDossier,
        heurist_x402::HeuristPaymentTrace,
        launch_intelligence::{self, DeployerMemorySummary, WalletStructureSummary},
        linking::{self, LinkedLaunchSummary},
        pattern_engine::{self, PatternEngineSummary},
    },
};

#[derive(Debug, Clone)]
pub(crate) struct ToolPayment {
    pub amount_units: String,
    pub amount_display: String,
    pub cost_cents: u32,
    pub network: String,
    pub asset: String,
    pub payment_tx: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ToolExecution<T> {
    pub tool_name: &'static str,
    pub provider: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub payments: Vec<ToolPayment>,
    pub payload: T,
}

impl From<HeuristPaymentTrace> for ToolPayment {
    fn from(value: HeuristPaymentTrace) -> Self {
        Self {
            amount_units: value.amount_units,
            amount_display: value.amount_display,
            cost_cents: value.cost_cents,
            network: value.network,
            asset: value.asset,
            payment_tx: value.payment_tx,
        }
    }
}

pub(crate) async fn get_market_structure(
    token_address: &str,
) -> Result<ToolExecution<DexScreenerContext>, AppError> {
    let context = dexscreener::fetch_pair_context(token_address)
        .await
        .map_err(|err| AppError::NotReady(format!("DexScreener unavailable: {err}")))?;

    Ok(ToolExecution {
        tool_name: "get_market_structure",
        provider: context.provider.clone(),
        summary: context.summary.clone(),
        evidence: vec![
            context.market_structure_label.clone(),
            context
                .source_url
                .clone()
                .unwrap_or_else(|| "No source URL returned.".to_string()),
        ],
        payments: Vec::new(),
        payload: context,
    })
}

pub(crate) async fn get_wallet_structure(
    db: &sqlx::PgPool,
    token_snapshot: &TokenSnapshot,
) -> Result<ToolExecution<WalletStructureSummary>, AppError> {
    let summary = launch_intelligence::build_wallet_structure_summary(db, token_snapshot)
        .await
        .map_err(|err| AppError::NotReady(format!("Wallet structure unavailable: {err}")))?;

    Ok(ToolExecution {
        tool_name: "get_wallet_structure",
        provider: "mia_internal_wallet_graph".to_string(),
        summary: summary.summary.clone(),
        evidence: summary.evidence.clone(),
        payments: Vec::new(),
        payload: summary,
    })
}

pub(crate) async fn get_deployer_memory(
    db: &sqlx::PgPool,
    token_snapshot: &TokenSnapshot,
) -> Result<ToolExecution<Option<DeployerMemorySummary>>, AppError> {
    let summary = launch_intelligence::build_deployer_memory_summary(db, token_snapshot)
        .await
        .map_err(|err| AppError::NotReady(format!("Deployer memory unavailable: {err}")))?;

    let (text, evidence) = match &summary {
        Some(payload) => (payload.summary.clone(), payload.evidence.clone()),
        None => (
            "No deployer memory record is attached to this token yet.".to_string(),
            vec!["The deployer did not resolve to a stored profile in MIA.".to_string()],
        ),
    };

    Ok(ToolExecution {
        tool_name: "get_deployer_memory",
        provider: "mia_internal_history".to_string(),
        summary: text,
        evidence,
        payments: Vec::new(),
        payload: summary,
    })
}

pub(crate) async fn get_linked_launch_cluster(
    db: &sqlx::PgPool,
    token_address: &str,
) -> Result<ToolExecution<Option<LinkedLaunchSummary>>, AppError> {
    let summary = linking::build_linked_launch_summary(db, token_address)
        .await
        .map_err(|err| AppError::NotReady(format!("Linked launch cluster unavailable: {err}")))?;

    let (text, evidence) = match &summary {
        Some(payload) => (payload.summary.clone(), payload.evidence.clone()),
        None => (
            "No linked launch cluster was recovered for this token yet.".to_string(),
            vec!["No linked cluster evidence was returned from the internal dataset.".to_string()],
        ),
    };

    Ok(ToolExecution {
        tool_name: "get_linked_launch_cluster",
        provider: "mia_internal_linking".to_string(),
        summary: text,
        evidence,
        payments: Vec::new(),
        payload: summary,
    })
}

pub(crate) async fn get_optional_narrative_context(
    config: &Config,
    token_address: &str,
    symbol_hint: &str,
) -> Result<ToolExecution<Option<HeuristDossier>>, AppError> {
    match config.deep_research_provider {
        DeepResearchProvider::HeuristMeshX402 => {
            match crate::research::heurist_x402::run_paid_mvp_dossier(
                config,
                token_address,
                symbol_hint,
            )
            .await
            {
                Ok((dossier, payment_traces)) => Ok(ToolExecution {
                    tool_name: "get_optional_narrative_context",
                    provider: "heurist_mesh".to_string(),
                    summary: "Optional paid upstream narrative enrichment attached.".to_string(),
                    evidence: vec![
                        format!("Heurist result count: {}.", dossier.results.len()),
                        dossier.executive_summary.clone(),
                        format!("Paid upstream calls: {}.", payment_traces.len()),
                    ],
                    payments: payment_traces.into_iter().map(Into::into).collect(),
                    payload: Some(dossier),
                }),
                Err(err) => Ok(ToolExecution {
                    tool_name: "get_optional_narrative_context",
                    provider: "heurist_mesh".to_string(),
                    summary: "Optional narrative enrichment is unavailable for this run."
                        .to_string(),
                    evidence: vec![format!("Narrative lane degraded safely: {err}")],
                    payments: Vec::new(),
                    payload: None,
                }),
            }
        }
        DeepResearchProvider::NativeXApi => Ok(ToolExecution {
            tool_name: "get_optional_narrative_context",
            provider: "native_x_api_reserved".to_string(),
            summary: "Native X narrative lane is reserved for a later rollout.".to_string(),
            evidence: vec![
                "No external narrative call was required for this run.".to_string(),
                json!({"provider": "native_x_api_reserved"}).to_string(),
            ],
            payments: Vec::new(),
            payload: None,
        }),
    }
}

pub(crate) async fn get_pattern_matches(
    db: &sqlx::PgPool,
    token_address: &str,
) -> Result<ToolExecution<Option<PatternEngineSummary>>, AppError> {
    let summary = pattern_engine::load_latest_pattern_engine_summary(db, token_address)
        .await
        .map_err(|err| AppError::NotReady(format!("Pattern engine unavailable: {err}")))?;

    let (text, evidence) = match &summary {
        Some(payload) => (payload.summary.clone(), payload.evidence.clone()),
        None => (
            "Pattern Match Engine has no active prediction for this token yet.".to_string(),
            vec![
                "No active pattern model or no horizon predictions were stored for this token."
                    .to_string(),
            ],
        ),
    };

    Ok(ToolExecution {
        tool_name: "get_pattern_matches",
        provider: "mia_pattern_engine".to_string(),
        summary: text,
        evidence,
        payments: Vec::new(),
        payload: summary,
    })
}

#[cfg(test)]
mod tests {
    use super::ToolExecution;

    #[test]
    fn tool_execution_preserves_summary_and_provider() {
        let result = ToolExecution {
            tool_name: "get_wallet_structure",
            provider: "mia_internal_wallet_graph".to_string(),
            summary: "Wallet structure attached.".to_string(),
            evidence: vec!["Wallet breadth recovered.".to_string()],
            payments: Vec::new(),
            payload: 7u8,
        };

        assert_eq!(result.tool_name, "get_wallet_structure");
        assert_eq!(result.provider, "mia_internal_wallet_graph");
        assert_eq!(result.payload, 7);
    }
}
