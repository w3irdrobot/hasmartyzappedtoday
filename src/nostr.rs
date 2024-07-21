use std::time::Duration;

use anyhow::Result;
use log::{debug, info};
use nostr_sdk::prelude::*;

const RELAYS: [&'static str; 9] = [
    "wss://relay.damus.io",
    "wss://nostr.plebchain.org/",
    "wss://bitcoiner.social/",
    "wss://relay.snort.social",
    "wss://relayable.org",
    "wss://nos.lol",
    "wss://nostr.mom",
    "wss://e.nos.lol",
    "wss://nostr.bitcoiner.social",
];

pub const NPUB_MARTY: &str = "npub1guh5grefa7vkay4ps6udxg8lrqxg2kgr3qh9n4gduxut64nfxq0q9y6hjy";

const THIRTY_DAYS: Duration = Duration::from_secs(60 * 60 * 24 * 30);

pub async fn get_client(db_path: &str) -> Result<Client> {
    let database = NdbDatabase::open(db_path)?;
    let client = Client::builder().database(database).build();
    // add reader relays
    for relay in RELAYS {
        client
            .add_relay_with_opts(relay, RelayOptions::default().write(false))
            .await?;
    }

    client.connect().await;

    Ok(client)
}

fn npubs_to_check() -> Vec<String> {
    let marty_pubkey = PublicKey::parse(NPUB_MARTY).unwrap();
    vec![marty_pubkey.to_hex()]
}

pub async fn subscribe_to_npubs(client: Client) -> Result<()> {
    let filters = zaps_filters_since(Timestamp::now() - THIRTY_DAYS);
    client.subscribe(filters, None).await?;

    Ok(())
}

pub fn zaps_filters_since(since: Timestamp) -> Vec<Filter> {
    let zap_filter = Filter::new().kind(Kind::ZapReceipt).since(since);
    let zap_p_filter = Filter::new()
        .kind(Kind::ZapReceipt)
        .custom_tag(SingleLetterTag::uppercase(Alphabet::P), &npubs_to_check())
        .since(since);

    vec![zap_filter, zap_p_filter]
}

pub fn check_for_zap_event(events: Vec<Event>) -> bool {
    let marty_pubkey = PublicKey::parse(NPUB_MARTY).unwrap();
    for event in events {
        match event.kind() {
            Kind::ZapReceipt => {
                let Some(tag) = event
                    .tags()
                    .iter()
                    .find(|t| t.kind() == TagKind::Description)
                else {
                    debug!("no description tag found in event {}", event.id());
                    continue;
                };

                let Ok(event) = Event::from_json(tag.content().unwrap()) else {
                    debug!("description tag is not a valid event");
                    continue;
                };
                if let Err(e) = event.verify() {
                    debug!("invalid zap request event: {:?}", e);
                    continue;
                }
                if event.author() != marty_pubkey {
                    continue;
                }
                info!("the zapped event id: {}", event.id);
                return true;
            }
            _ => {}
        }
    }

    false
}
