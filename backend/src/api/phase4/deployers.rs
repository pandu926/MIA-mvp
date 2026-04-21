use super::types::{risk_category, DeployerTokenResponse, LimitQuery};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};

/// GET /api/v1/deployer/:address/tokens?limit=20
pub async fn get_deployer_tokens(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<DeployerTokenResponse>>, AppError> {
    let limit = params.limit.clamp(1, 200);
    let rows: Vec<(
        String,
        Option<String>,
        Option<String>,
        DateTime<Utc>,
        i32,
        i32,
        f64,
        Option<i16>,
    )> = sqlx::query_as(
        r#"
            SELECT
                t.contract_address,
                t.name,
                t.symbol,
                t.deployed_at,
                t.buy_count,
                t.sell_count,
                t.volume_bnb::double precision,
                rs.composite_score
            FROM tokens t
            LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
            WHERE t.deployer_address = $1
            ORDER BY t.deployed_at DESC
            LIMIT $2
            "#,
    )
    .bind(&address)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    contract_address,
                    name,
                    symbol,
                    deployed_at,
                    buy_count,
                    sell_count,
                    volume_bnb,
                    composite_score,
                )| DeployerTokenResponse {
                    contract_address,
                    name,
                    symbol,
                    deployed_at,
                    buy_count,
                    sell_count,
                    volume_bnb,
                    risk_category: composite_score.map(|score| risk_category(score).to_string()),
                    composite_score,
                },
            )
            .collect(),
    ))
}
