use std::{net::SocketAddr, sync::Arc};

use axum::{Extension, Router};
use axum_extra::routing::SpaRouter;
use color_eyre::eyre::Context;

mod config;
mod file_cache;
mod request;
mod rounds;
mod routes;
mod runtimes;
mod servers;
mod state;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    tracing::info!("starting moth-fans-runtimes");

    let config = config::Config::read_from_file().context("error reading config")?;
    let address = SocketAddr::from((config.address, config.port));

    let state = state::AppState::new(config);

    let (major, minor) = state.current_db_revision().await?;
    tracing::debug!("current db revision: {major}.{minor}");

    {
        tracing::debug!("loading rounds");
        let rounds = state.rounds().await?;
        tracing::debug!("loaded {} rounds", rounds.len());
    }

    let spa = SpaRouter::new("/static", "dist");

    let state_arc = Arc::new(state);

    if state_arc.config.mock_runtimes_data.is_none() {
        tokio::task::spawn(track_rounds(state_arc.clone()));
    }

    let app = Router::new()
        .merge(spa)
        .route("/data.json", axum::routing::get(routes::data))
        .layer(Extension(state_arc));

    tracing::debug!("listening on {address}");

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn track_rounds(state: Arc<state::AppState>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(
            state.config.rounds_cache_delay_secs,
        ))
        .await;

        tracing::trace!("updating rounds cache");

        match state.save_new_rounds().await {
            Ok(()) => {
                tracing::trace!("updated rounds cache");
            }

            Err(error) => {
                tracing::error!("error loading rounds: {error}");
            }
        }
    }
}
