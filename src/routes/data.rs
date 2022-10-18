use std::sync::{Arc, Mutex};

use axum::{response::IntoResponse, Extension};
use once_cell::sync::OnceCell;

use crate::{rounds::Round, state::AppState};

pub static CACHED_RESPONSE: OnceCell<Mutex<(std::time::Instant, String)>> = OnceCell::new();

#[tracing::instrument]
pub async fn data(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let now = std::time::Instant::now();

    if let Some(response_lock) = CACHED_RESPONSE.get() {
        let lock = response_lock.lock().unwrap();
        let last_updated = lock.0;
        let response = &lock.1;

        if now.duration_since(last_updated).as_secs() < 60 {
            tracing::trace!("returning cached response");
            return ([("content-type", "application/json")], response.clone()).into_response();
        }
    }

    let rounds: &Vec<Round> = &state.rounds().await.expect("can't get rounds");
    let response = serde_json::to_string(&rounds).expect("can't serialize rounds");

    if let Some(response_lock) = CACHED_RESPONSE.get() {
        let mut lock = response_lock.lock().unwrap();
        lock.0 = now;
        lock.1 = response.clone();
    } else {
        CACHED_RESPONSE
            .set(Mutex::new((now, response.clone())))
            .expect("can't set cached response");
    }

    tracing::trace!("returning fresh response");

    ([("content-type", "application/json")], response).into_response()
}
