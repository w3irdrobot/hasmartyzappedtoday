use anyhow::{bail, Result};
use config::{Case, Environment};
use serde::Deserialize;

use crate::server::{start_server, ServerConfig};
use nostr::{get_client, subscribe_to_npubs};

mod nostr;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = dotenvy::dotenv() {
        if !e.not_found() {
            bail!(e)
        }
    }
    env_logger::init();

    let cfg = get_config().await?;
    let client = get_client(&cfg.database_path).await?;

    subscribe_to_npubs(client.clone()).await?;
    start_server(cfg.server, client.clone()).await?;

    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    database_path: String,
    server: ServerConfig,
}

async fn get_config() -> Result<Config> {
    let cfg = config::Config::builder()
        .add_source(
            Environment::default()
                .prefix("marty")
                .prefix_separator("_")
                .convert_case(Case::UpperSnake)
                .separator("__"),
        )
        .set_default("database_path", "./marty.db")?
        .build()?;

    Ok(cfg.try_deserialize()?)
}
