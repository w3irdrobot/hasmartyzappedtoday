use std::{sync::Arc, time::Duration};

// use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::Result,
    routing::get,
    Router,
};
use log::info;
use maud::{html, Markup, DOCTYPE};
use nostr_sdk::prelude::*;
use serde::Deserialize;
use tower_http::{cors::CorsLayer, normalize_path::NormalizePathLayer, services::ServeDir};

use crate::nostr::{check_for_zap_event, zaps_filters_since};

const TWENTY_FOUR_HOURS: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    host: String,
    port: u16,
    domain: Option<String>,
}

#[derive(Debug, Clone)]
struct ServerContext {
    client: Client,
}

pub async fn start_server(config: ServerConfig, client: Client) -> anyhow::Result<()> {
    let state = Arc::new(ServerContext { client });

    let mut app = Router::new()
        .route("/", get(check_martys_zaps))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state)
        .layer(NormalizePathLayer::trim_trailing_slash());

    if let Some(domain) = config.domain {
        let cors = CorsLayer::new()
            .allow_methods([Method::GET])
            .allow_origin(domain.parse::<HeaderValue>().unwrap());

        app = app.layer(cors);
    }

    let host = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&host).await.unwrap();

    info!("server is running on {}", host);
    Ok(axum::serve(listener, app).await?)
}

async fn check_martys_zaps(State(state): State<Arc<ServerContext>>) -> Result<Markup> {
    let client = state.client.clone();
    let database = client.database();
    let filters = zaps_filters_since(Timestamp::now() - TWENTY_FOUR_HOURS);
    let results = database
        .query(filters, Order::Desc)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let has_zapped = check_for_zap_event(results);

    Ok(html! {
        (header())
        main.grid.min-h-full.place-items-center.bg-white."px-6"."py-24"."sm:py-32"."lg:px-8" {
            h1."mb-3"."text-3xl".font-bold.tracking-tight."text-gray-900"."sm:text-5xl" {"Marty Bent"}
            @if has_zapped {
                p.text-base.font-semibold.text-indigo-600 {"has zapped today!"}
            } @else {
                p.text-base.font-semibold.text-indigo-600 {"has not zapped today!"}
            }
        }
        (footer())
    })
}

fn header() -> Markup {
    html! {
        (DOCTYPE)
        meta charset="UTF-8";
        meta content="text/html;charset=utf-8" http-equiv="Content-Type";
        meta name="viewport" content="width=device-width, initial-scale=1";
        title { "Has Marty Zapped Today?" }
        link rel="stylesheet" href="https://rsms.me/inter/inter.css";
        link rel="stylesheet" href="/assets/main.css";
    }
}

fn footer() -> Markup {
    html! {
        footer {}
    }
}
