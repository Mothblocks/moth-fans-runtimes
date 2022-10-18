use std::{future::Future, path::PathBuf};

use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

static CACHE_PATH: &str = "cache";

// TODO: Replace this with sqlite someday
pub async fn from_cache_or<T, F, Ft>(path: &str, callback: F) -> color_eyre::Result<T>
where
    F: FnOnce() -> Ft,
    Ft: Future<Output = color_eyre::Result<T>>,
    T: DeserializeOwned + Serialize,
{
    fs::create_dir_all(PathBuf::from(CACHE_PATH).join(path).parent().unwrap())
        .await
        .expect("couldn't create cache dir");

    let cache_file_path = PathBuf::from(CACHE_PATH).join(path);

    if let Ok(mut file) = fs::File::open(&cache_file_path).await {
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        let value = serde_json::from_str(&contents)?;

        tracing::trace!("loading cached value from `{}`", cache_file_path.display());

        Ok(value)
    } else {
        let value = callback().await?;

        let mut file = fs::File::create(&cache_file_path).await?;

        match file
            .write_all(serde_json::to_string(&value)?.as_bytes())
            .await
        {
            Ok(_) => {
                tracing::debug!("writing cached value into `{}`", cache_file_path.display());
            }
            Err(error) => {
                tracing::warn!("couldn't write cache file: {error}");
            }
        }

        Ok(value)
    }
}
