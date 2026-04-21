use axum::{extract::State, Json};

use crate::{
    error::AppError,
    research::auto_investigation::{run_auto_investigation_scan, AutoInvestigationSettings},
    AppState,
};

pub async fn post_auto_investigation_scan(
    State(state): State<AppState>,
) -> Result<Json<crate::research::auto_investigation::AutoInvestigationScanResponse>, AppError> {
    let settings = AutoInvestigationSettings::from_config(&state.config);
    let result = run_auto_investigation_scan(&state.db, &settings).await?;
    Ok(Json(result))
}
