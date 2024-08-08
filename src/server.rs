use std::{sync::Arc, time::Duration};

use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};
use time::format_description::well_known::Rfc3339;
// use anyhow::Result;
use ::time::OffsetDateTime;
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Result},
    routing::get,
    Router,
};
use log::{error, info};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_http::{cors::CorsLayer, normalize_path::NormalizePathLayer, services::ServeDir};

use crate::nostr::NPUB_MARTY;
use crate::{
    db::{get_most_recent_zap, get_most_recent_zaps},
    nostr::NPUB_ODELL,
};

const TWENTY_FOUR_HOURS: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    host: String,
    port: u16,
    domain: Option<String>,
}

#[derive(Debug, Clone)]
struct ServerContext {
    db: SqlitePool,
}

pub async fn start_server(config: ServerConfig, db: SqlitePool) -> anyhow::Result<()> {
    let state = Arc::new(ServerContext { db });

    let mut app = Router::new()
        .route("/", get(check_martys_zaps))
        .route("/odell", get(check_odells_zaps))
        .route("/rss.xml", get(martys_zaps_rss))
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

async fn check_martys_zaps(State(state): State<Arc<ServerContext>>) -> Result<Markup, StatusCode> {
    check_zaps_handler(state, "Marty Bent", NPUB_MARTY).await
}

async fn check_odells_zaps(State(state): State<Arc<ServerContext>>) -> Result<Markup, StatusCode> {
    check_zaps_handler(state, "ODELL", NPUB_ODELL).await
}

async fn check_zaps_handler(
    state: Arc<ServerContext>,
    name: &str,
    npub: &str,
) -> Result<Markup, StatusCode> {
    let db = state.db.clone();
    let has_zapped = match get_most_recent_zap(db, npub).await {
        Ok(zap) => {
            let beginning_of_day = OffsetDateTime::now_utc()
                .replace_hour(0)
                .unwrap()
                .replace_minute(0)
                .unwrap()
                .replace_second(0)
                .unwrap();
            zap.zapped_at >= beginning_of_day - TWENTY_FOUR_HOURS
        }
        Err(err) => {
            error!("error getting most recent zap: {}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(html! {
        (header(has_zapped))
        body {
            main.container.is-max-tablet.my-4 {
                .content.has-text-centered {
                    .block {
                        h1 { (name) }
                        @if has_zapped {
                            p {"has zapped today!"}
                            img src="/assets/thumbsup.webp";
                        } @else {
                            p {"has not zapped today!"}
                            img src="/assets/angrypanda.webp";
                        }
                    }
                    .block {
                        p {
                            @if name == "ODELL" {
                                a href="/" { strong { "But has Marty zapped today?" } }
                            } @else {
                                a href="/odell" { strong { "But has ODELL zapped today?" } }
                            }
                        }
                    }
                    .block {
                        p {
                            "Wanna know when Marty zaps in your feed reader?"
                            br;
                            a href="/rss.xml" { strong { "Subscribe to our RSS feed!" } }
                        }
                        p {
                        }
                    }
                }
            }
            (footer())
        }
    })
}

async fn martys_zaps_rss(State(state): State<Arc<ServerContext>>) -> Result<impl IntoResponse> {
    let db = state.db.clone();

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_str("application/xml").unwrap(),
    );

    let mut channel = ChannelBuilder::default();
    channel
        .title("Has Marty Zapped Today?".to_string())
        .link("https://hasmartyzapped.today")
        .description("Determine if Marty Bent has zapped today.");

    let zaps = get_most_recent_zaps(db, NPUB_MARTY, 20)
        .await
        .map_err(|e| {
            error!("error getting the most recent 20 zaps: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    for zap in zaps {
        let guid = GuidBuilder::default()
            .value(zap.id.simple().to_string())
            .build();
        let amount_sats = zap.amount / 1000;
        channel.item(
            ItemBuilder::default()
                .guid(guid)
                .title(format!("Marty Bent zapped {} sats!", amount_sats))
                .pub_date(zap.zapped_at.format(&Rfc3339).unwrap())
                .content(format!(r#"
            Marty Bent has zapped {} sats!
            It was in <a href="https://njump.me/{}">event {}</a>.

            Go <a href="https://njump.me/npub1guh5grefa7vkay4ps6udxg8lrqxg2kgr3qh9n4gduxut64nfxq0q9y6hjy">let him know</a> you're proud of him!
        "#, amount_sats, zap.receipt_id, zap.receipt_id))
                .build(),
        );
    }

    let channel = channel.build();

    Ok((headers, channel.to_string()))
}

fn header(has_zapped: bool) -> Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset="UTF-8";
            meta content="text/html;charset=utf-8" http-equiv="Content-Type";
            meta name="viewport" content="width=device-width, initial-scale=1";

            meta property="og:title" content="Has Marty Zapped Today?";
            meta property="og:type" content="website";
            meta property="og:description" content="Check to make sure Marty Bent has zapped today.";
            meta property="og:url" content="https://hasmartyzapped.today";
            @if has_zapped {
                meta property="og:image" content="/assets/yes.jpeg";
            } @else {
                meta property="og:image" content="/assets/no.jpeg";
            }

            title { "Has Marty Zapped Today?" }
            link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@1.0.1/css/bulma.min.css";
        }
    }
}

fn footer() -> Markup {
    html! {
        footer {
            .has-text-centered {
                p {
                    "Built by "
                    a href="https://njump.me/rob@w3ird.tech" target="_blank" { "w3irdrobot" }
                }
            }
        }
    }
}
