use crate::experiments::analyzer::analyze;
use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

pub async fn get_experiment_winner(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> impl IntoResponse {
    let results = match state.experiments_repo.results(experiment_id).await {
        Ok(v) => v,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let analysis = analyze(&results, 100);
    (axum::http::StatusCode::OK, Json(analysis)).into_response()
}
