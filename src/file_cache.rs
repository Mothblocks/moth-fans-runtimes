use std::{fs, future::Future, path::PathBuf};

use serde::{de::DeserializeOwned, Serialize};

static CACHE_PATH: &str = "cache";

// TODO: Replace this with sqlite someday
pub async fn from_cache_or<T, F, Ft>(path: &str, callback: F) -> color_eyre::Result<T>
where
    F: FnOnce() -> Ft,
    Ft: Future<Output = color_eyre::Result<T>>,
    T: DeserializeOwned + Serialize,
{
    fs::create_dir_all(PathBuf::from(CACHE_PATH).join(path).parent().unwrap())
        .expect("couldn't create cache dir");

    let cache_file_path = PathBuf::from(CACHE_PATH).join(path);

    if let Ok(file) = fs::File::open(&cache_file_path) {
        let value = serde_json::from_reader(file)?;

        tracing::debug!("loading cached value from `{}`", cache_file_path.display());

        Ok(value)
    } else {
        let value = callback().await?;

        let file = fs::File::create(&cache_file_path)?;

        serde_json::to_writer(file, &value)?;

        tracing::debug!("writing cached value into `{}`", cache_file_path.display());

        Ok(value)
    }
}
