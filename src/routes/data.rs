use std::sync::Arc;

use axum::{response::IntoResponse, Extension, Json};

use crate::{rounds::Round, state::AppState};

// TODO: Cache
pub async fn data(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let rounds: &Vec<Round> = &state.rounds().await.expect("can't get rounds");

    Json(rounds).into_response()
}
