use std::{net::IpAddr, path::PathBuf};

use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub address: IpAddr,
    pub port: u16,

    pub db_url: String,

    pub mock_runtimes_data: Option<PathBuf>,

    pub rounds_cache_delay_secs: u64,
}

impl Config {
    pub fn read_from_file() -> color_eyre::Result<Self> {
        toml::from_str(&std::fs::read_to_string("config.toml")?).map_err(Into::into)
    }
}
