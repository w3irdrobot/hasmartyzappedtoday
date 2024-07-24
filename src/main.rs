use anyhow::{bail, Result};
use config::{Case, Environment};
use log::debug;
use serde::Deserialize;

use crate::db::connect_database;
use crate::nostr::{get_client, save_zaps_to_db, subscribe_to_npubs};
use crate::server::{start_server, ServerConfig};

mod db;
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
    debug!("config: {:?}", cfg);
    let client = get_client(&cfg.ndb_path).await?;
    let db = connect_database(&cfg.sqlite_path).await?;

    let processor_handle = tokio::spawn(save_zaps_to_db(client.clone(), db.clone()));
    subscribe_to_npubs(client.clone()).await?;
    let server_handle = tokio::spawn(start_server(cfg.server, db));

    let _ = tokio::join!(processor_handle, server_handle);

    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    ndb_path: String,
    sqlite_path: String,
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
        .set_default("ndb_path", "./marty.db")?
        .set_default("sqlite_path", "sqlite://zaps.db")?
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", "8000")?
        .build()?;

    Ok(cfg.try_deserialize()?)
}
