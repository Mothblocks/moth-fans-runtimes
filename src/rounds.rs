use std::{collections::HashMap, path::PathBuf, time::Duration};

use chrono::{Datelike, NaiveDateTime};
use color_eyre::eyre::Context;
use serde::{Deserialize, Serialize};
use sqlx::{mysql::MySqlRow, MySqlConnection, Row};
use tokio::io::AsyncReadExt;

use crate::{
    file_cache::from_cache_or,
    request::request,
    runtimes::{BestGuessFilenames, RuntimeBatch},
};

pub type RoundId = i32;

pub async fn load_rounds_over_cloud(
    connection: &mut MySqlConnection,
) -> color_eyre::Result<Vec<Round>> {
    let mut context = RoundCollectionContext::reload().await;

    let mut rounds = Vec::new();

    for row in sqlx::query(
        r#"
        SELECT 
            round.id,
            round.initialize_datetime,
            round.server_port,
            round.commit_hash,
            JSON_EXTRACT(feedback.json, '$.data.*') AS test_merges
        FROM
            round
                LEFT JOIN
            feedback ON feedback.round_id = round.id
                AND feedback.key_name = 'testmerged_prs'
        WHERE round.initialize_datetime >= NOW() - INTERVAL 7 DAY
        ORDER BY round.id DESC
    "#,
    )
    .fetch_all(connection)
    .await?
    .into_iter()
    {
        let round_id = match row.try_get::<RoundId, _>("id") {
            Ok(id) => id,
            Err(_) => {
                tracing::warn!("round with no id found, skipping");
                continue;
            }
        };

        match load_round_from_row(&mut context, row).await {
            Ok(round) => rounds.push(round),
            Err(error) => {
                tracing::warn!("failed to load round {round_id}\n{error}");
            }
        }
    }

    Ok(rounds)
}

async fn load_round_from_row(
    context: &mut RoundCollectionContext,
    row: MySqlRow,
) -> color_eyre::Result<Round> {
    let round_id = row.try_get("id")?;

    from_cache_or(&format!("rounds/{round_id}.json"), || async {
        let mut test_merge_details: Vec<TestMergeDetails> =
            match row.get::<Option<String>, _>("test_merges") {
                Some(test_merge_details) => serde_json::from_str(&test_merge_details)?,
                None => Vec::new(),
            };

        // https://github.com/tgstation/tgstation/issues/70292
        test_merge_details.dedup_by_key(|test_merge| test_merge.number);

        let mut test_merges = Vec::new();
        for details in test_merge_details {
            test_merges.push(context.test_merge_from_details(details).await);
        }

        let port = row.try_get("server_port")?;

        let timestamp = row.get("initialize_datetime");

        Ok(Round {
            round_id,
            server: match crate::servers::server_by_port(port) {
                Some(server) => server.name.to_owned(),
                None => format!("unknown server: {port}"),
            },
            revision: row.try_get("commit_hash")?,

            runtimes: match load_runtimes_from(context, round_id, port, &timestamp).await {
                Ok(runtimes) => Some(runtimes),

                Err(error) => {
                    tracing::warn!("error loading runtimes for round {round_id}: {error}");

                    None
                }
            },

            timestamp,

            test_merges,
        })
    })
    .await
}

#[tracing::instrument]
async fn load_runtimes_from(
    context: &mut RoundCollectionContext,
    round_id: RoundId,
    port: u16,
    timestamp: &NaiveDateTime,
) -> color_eyre::Result<Vec<RuntimeBatch>> {
    let request_url = format!(
        "https://tgstation13.org/parsed-logs/{}/data/logs/{}/{:02}/{:02}/round-{round_id}/runtime.condensed.txt",
        match crate::servers::server_by_port(port) {
            Some(server) => server.name,
            None => {
                color_eyre::eyre::bail!("unknown server: {port}");
            }
        },
        timestamp.year(),
        timestamp.month(),
        timestamp.day(),
    );

    tracing::debug!("loading runtimes from {request_url}");

    let runtime_condensed_txt = request(&request_url)
        .await
        .and_then(reqwest::Response::error_for_status)
        .context("couldn't get runtime.condensed.txt")?
        .text()
        .await?;

    let mut runtimes = crate::runtimes::get_runtimes_for_round(&runtime_condensed_txt)?;

    for runtime in runtimes.iter_mut() {
        if matches!(
            runtime.best_guess_filenames,
            Some(BestGuessFilenames::Definitely(_))
        ) {
            continue;
        }

        if let Some(filenames) = context.git_tree.get(&runtime.source_file) {
            runtime.best_guess_filenames = Some(BestGuessFilenames::Possible(filenames.clone()));
        }
    }

    Ok(runtimes)
}

struct RoundCollectionContext {
    git_tree: HashMap<String, Vec<PathBuf>>,

    test_merges: HashMap<String, TestMerge>,
}

impl std::fmt::Debug for RoundCollectionContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "RoundCollectionContext")
    }
}

impl RoundCollectionContext {
    async fn reload() -> Self {
        let mut test_merges = HashMap::new();

        tokio::fs::create_dir_all("cache/test_merges")
            .await
            .expect("failed to create test_merges cache dir");

        if let Ok(mut read_dir) = tokio::fs::read_dir("cache/test_merges").await {
            // for entry in read_dir {
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let test_merge: TestMerge = match tokio::fs::File::open(entry.path()).await {
                    Ok(mut file) => {
                        let mut contents = String::new();
                        match file.read_to_string(&mut contents).await {
                            Ok(_) => match serde_json::from_str(&contents) {
                                Ok(test_merge) => test_merge,

                                Err(error) => {
                                    tracing::warn!(
                                        "couldn't deserialize test merge from file\n{error}"
                                    );

                                    continue;
                                }
                            },

                            Err(error) => {
                                tracing::warn!("couldn't read test merge file\n{error}");

                                continue;
                            }
                        }
                    }

                    Err(error) => {
                        tracing::warn!(
                            "couldn't read file `{}` when loading test merges\n{error}",
                            entry.path().display()
                        );

                        continue;
                    }
                };

                let commit = test_merge.details.commit.clone();

                tracing::debug!(
                    "loaded cached test merge {} ({})",
                    test_merge.details.number,
                    commit
                );

                test_merges.insert(commit, test_merge);
            }
        }

        Self {
            test_merges,
            git_tree: Self::get_git_tree().await,
        }
    }

    #[tracing::instrument]
    async fn get_git_tree() -> HashMap<String, Vec<PathBuf>> {
        const GIT_TREE_CACHE_FILE: &str = "cache/git_tree.json";

        #[derive(Deserialize)]
        struct GitTreeResponse {
            tree: Vec<GitTreeEntry>,
        }

        #[derive(Deserialize)]
        struct GitTreeEntry {
            path: PathBuf,
        }

        match request(
            "https://api.github.com/repos/tgstation/tgstation/git/trees/master?recursive=1",
        )
        .await
        .and_then(reqwest::Response::error_for_status)
        {
            Ok(response) => match response.json::<GitTreeResponse>().await {
                Ok(git_tree) => {
                    tracing::debug!("loaded git tree from cloud, saving in cache");

                    let mut names: HashMap<String, Vec<PathBuf>> = HashMap::new();

                    for entry in git_tree.tree {
                        let name = entry
                            .path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .into_owned();

                        names.entry(name).or_default().push(entry.path);
                    }

                    if let Err(error) = tokio::fs::write(
                        GIT_TREE_CACHE_FILE,
                        serde_json::to_string_pretty(&names).unwrap(),
                    )
                    .await
                    {
                        tracing::warn!("failed to save git tree to cache\n{error}");
                    }

                    return names;
                }

                Err(error) => {
                    tracing::warn!("failed to deserialize git tree from request\n{error}");
                }
            },

            Err(error) => {
                tracing::warn!("failed to get git tree from request\n{error}");
            }
        }

        tracing::debug!("attempting to load git tree from cache");

        if let Ok(mut file) = tokio::fs::File::open(GIT_TREE_CACHE_FILE).await {
            let mut contents = String::new();

            match file.read_to_string(&mut contents).await {
                Ok(_) => match serde_json::from_str(&contents) {
                    Ok(git_tree) => {
                        tracing::debug!("loaded git tree from cache");

                        return git_tree;
                    }

                    Err(error) => {
                        tracing::warn!("failed to deserialize git tree from cache\n{error}");
                    }
                },

                Err(error) => {
                    tracing::warn!("failed to read git tree from cache\n{error}");
                }
            }
        } else {
            tracing::debug!("couldn't load git tree cache, going empty");
        }

        HashMap::new()
    }

    async fn test_merge_from_details(&mut self, details: TestMergeDetails) -> TestMerge {
        let commit = details.commit.clone();
        let number = details.number;

        if let Some(test_merge) = self.test_merges.get(&commit) {
            return test_merge.clone();
        }

        tracing::debug!("collecting information for test merge {number} ({commit})");

        let files_changed = match tokio::time::timeout(
            Duration::from_secs(8),
            request(format!(
                "https://api.github.com/repos/tgstation/tgstation/pulls/{}/files",
                number
            )),
        )
        .await
        .map(|response| response.and_then(reqwest::Response::error_for_status))
        {
            Ok(Ok(result)) => {
                #[derive(Deserialize)]
                struct File {
                    // False positive: https://github.com/serde-rs/serde/issues/2298
                    #[allow(dead_code)]
                    filename: PathBuf,
                }

                let files: Vec<File> = match result.json().await {
                    Ok(files) => files,
                    Err(error) => {
                        tracing::warn!("error parsing files changed: {}", error);
                        Vec::new()
                    }
                };

                Some(files.into_iter().map(|file| file.filename).collect())
            }

            Ok(Err(error)) => {
                tracing::warn!("couldn't find files changed for {}\n{error}", number);

                None
            }

            Err(_) => {
                tracing::warn!("timed out finding files changed for {}", number);

                None
            }
        };

        let test_merge = TestMerge {
            details,
            files_changed,
        };

        self.test_merges.insert(commit.clone(), test_merge.clone());
        let cache_file_path = test_merge.cache_file_path();

        match tokio::fs::File::create(&cache_file_path).await {
            Ok(file) => {
                if let Err(error) = serde_json::to_writer(file.into_std().await, &test_merge) {
                    tracing::warn!("couldn't serialize test merge to file\n{error}");
                }
            }

            Err(error) => {
                tracing::warn!(
                    "couldn't open file `{}` to save test merge\n{error}",
                    cache_file_path.display()
                );
            }
        }

        test_merge
    }
}

#[derive(Deserialize, Serialize)]
pub struct Round {
    // Details
    pub round_id: RoundId,
    pub timestamp: NaiveDateTime,
    pub revision: String,
    pub server: String,

    // Requires cloud data to collect
    pub runtimes: Option<Vec<RuntimeBatch>>,
    pub test_merges: Vec<TestMerge>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct TestMerge {
    pub details: TestMergeDetails,
    pub files_changed: Option<Vec<PathBuf>>,
}

impl TestMerge {
    pub fn cache_file_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "cache/test_merges/{}_{}.json",
            self.details.number, self.details.commit
        ))
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct TestMergeDetails {
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub number: u64,
    pub title: String,
    pub author: String,
    pub commit: String,
}

fn deserialize_string_to_u64<'de, D: serde::de::Deserializer<'de>>(
    deserializer: D,
) -> Result<u64, D::Error> {
    struct StringOrNumber;

    impl<'de> serde::de::Visitor<'de> for StringOrNumber {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
            Ok(value)
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
            value.parse().map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

#[derive(Deserialize, Serialize)]
pub struct Tree {
    pub files: Vec<String>,
}
