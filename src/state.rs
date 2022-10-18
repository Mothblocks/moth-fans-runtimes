use std::{collections::HashSet, fmt::Debug, ops::Deref, path::PathBuf};

use color_eyre::eyre::Context;
use sqlx::{Connection, Row};
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::{config::Config, rounds::Round};

pub struct AppState {
    config: Config,
    rounds: RwLock<Option<Vec<Round>>>,
}

impl Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "State")
    }
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            rounds: RwLock::new(None),
        }
    }

    async fn connect(&self) -> color_eyre::Result<sqlx::MySqlConnection> {
        sqlx::MySqlConnection::connect(&self.config.db_url)
            .await
            .context("couldn't connect to database")
    }

    pub async fn current_db_revision(&self) -> color_eyre::Result<(u32, u32)> {
        let mut connection = self.connect().await?;

        sqlx::query("SELECT major, minor FROM schema_revision ORDER BY date DESC")
            .fetch_all(&mut connection)
            .await
            .map(|rows| {
                let row = rows.first().expect("couldn't find revision");
                (row.get(0), row.get(1))
            })
            .map_err(|error| error.into())
    }

    // TODO: Update when expired in the BACKGROUND.
    // (That's why this isn't a OnceCell).
    pub async fn rounds(&self) -> color_eyre::Result<impl Deref<Target = Vec<Round>> + '_> {
        {
            let rounds_lock = self.rounds.read().await;
            if rounds_lock.is_some() {
                return Ok(RwLockReadGuard::map(rounds_lock, |rounds| {
                    rounds.as_ref().expect("rounds is None")
                }));
            }
        }

        {
            // Hold onto the lock for this entire time so nothing else tries to run DB queries
            let mut rounds_write = self.rounds.write().await;

            *rounds_write = Some(self.load_rounds().await?);
        }

        Ok(RwLockReadGuard::map(self.rounds.read().await, |rounds| {
            rounds.as_ref().expect("rounds is None")
        }))
    }

    #[tracing::instrument]
    async fn load_rounds(&self) -> color_eyre::Result<Vec<Round>> {
        if let Some(mock_runtimes_data_filename) = &self.config.mock_runtimes_data {
            match std::fs::File::open(&mock_runtimes_data_filename) {
                Ok(file) => {
                    tracing::debug!(
                        "loading mock runtimes data from `{}`",
                        mock_runtimes_data_filename.display()
                    );

                    return serde_json::from_reader(file).context("couldn't parse mock data");
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).context("error opening mock data file");
                }
            }

            tracing::debug!("couldn't load from mock data, loading from cloud")
        }

        let rounds = crate::rounds::load_rounds_over_cloud(&mut self.connect().await?)
            .await
            .context("couldn't load rounds over cloud")?;

        if let Some(mock_runtimes_data_filename) = &self.config.mock_runtimes_data {
            tracing::debug!(
                "saving mock data to {}",
                mock_runtimes_data_filename.display()
            );

            if let Err(error) =
                std::fs::write(mock_runtimes_data_filename, serde_json::to_string(&rounds)?)
            {
                tracing::warn!("couldn't write mock data: {error}");
            }
        }

        self.trash_old_cache(&rounds).await;

        Ok(rounds)
    }

    async fn trash_old_cache(&self, rounds: &[Round]) {
        let mut used_files = HashSet::new();

        for round in rounds {
            used_files.insert(PathBuf::from(format!(
                "cache/rounds/{}.json",
                round.round_id
            )));

            for test_merge in &round.test_merges {
                used_files.insert(test_merge.cache_file_path());
            }
        }

        for parent in ["cache/rounds", "cache/test_merges"] {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    if !used_files.contains(entry.path().as_path()) {
                        tracing::debug!("trashing old cache file {}", entry.path().display());

                        if let Err(error) = std::fs::remove_file(entry.path()) {
                            tracing::warn!("couldn't trash old cache file: {error}");
                        }
                    }
                }
            }
        }
    }
}
