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

    let app = Router::new()
        .merge(spa)
        .route("/data.json", axum::routing::get(routes::data))
        .layer(Extension(Arc::new(state)));

    tracing::debug!("listening on {address}");

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
