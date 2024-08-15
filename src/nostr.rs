use std::time::Duration;

use ::time::OffsetDateTime;
use anyhow::Result;
use log::{debug, error};
use nostr_sdk::prelude::*;
use sqlx::SqlitePool;

use crate::db::{add_zap, zap_already_tracked};

const RELAYS: [&str; 9] = [
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
pub const NPUB_TFTC: &str = "npub1sk7mtp67zy7uex2f3dr5vdjynzpwu9dpc7q4f2c8cpjmguee6eeq56jraw";
pub const NPUB_ODELL: &str = "npub1qny3tkh0acurzla8x3zy4nhrjz5zd8l9sy9jys09umwng00manysew95gx";

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
    let odell_pubkey = PublicKey::parse(NPUB_ODELL).unwrap();
    let tftc_pubkey = PublicKey::parse(NPUB_TFTC).unwrap();
    vec![
        marty_pubkey.to_hex(),
        odell_pubkey.to_hex(),
        tftc_pubkey.to_hex(),
    ]
}

pub async fn subscribe_to_npubs(client: Client) -> Result<()> {
    let filters = zaps_filters_since(Timestamp::now() - THIRTY_DAYS);
    client.subscribe(filters, None).await?;

    Ok(())
}

pub async fn save_zaps_to_db(client: Client, db: SqlitePool) -> Result<()> {
    let marty_pubkey = PublicKey::parse(NPUB_MARTY).unwrap();
    let odells_pubkey = PublicKey::parse(NPUB_ODELL).unwrap();
    let tftc_pubkey = PublicKey::parse(NPUB_TFTC).unwrap();
    let mut notifications = client.notifications();

    while let Ok(notification) = notifications.recv().await {
        let RelayPoolNotification::Message { message, .. } = notification else {
            continue;
        };

        let RelayMessage::Event { event, .. } = message else {
            continue;
        };

        if !was_zapped_by_npub(&event, &[marty_pubkey, odells_pubkey, tftc_pubkey]) {
            continue;
        }

        let request = get_zap_request(&event).unwrap();
        let npub = request.author().to_bech32().unwrap();
        let receipt_id = event.id().to_hex();
        match zap_already_tracked(db.clone(), &npub, &receipt_id).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(err) => {
                error!(
                    "error checking if the zap '{}' is already tracked: '{}'. skipping.",
                    event.id().to_hex(),
                    err
                );
                continue;
            }
        }

        let amount = get_zap_request_amount(&event);
        let created_at = OffsetDateTime::from_unix_timestamp(event.created_at().as_u64() as i64)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

        match add_zap(db.clone(), &npub, &receipt_id, created_at, amount).await {
            Ok(_) => debug!("zap '{}' saved", receipt_id),
            Err(err) => error!("error saving zap '{}': {}", receipt_id, err),
        }
    }

    Ok(())
}

pub fn zaps_filters_since(since: Timestamp) -> Vec<Filter> {
    let zap_filter = Filter::new().kind(Kind::ZapReceipt).since(since);
    let zap_p_filter = Filter::new()
        .kind(Kind::ZapReceipt)
        .custom_tag(SingleLetterTag::uppercase(Alphabet::P), npubs_to_check())
        .since(since);

    vec![zap_filter, zap_p_filter]
}

fn was_zapped_by_npub(event: &Event, npubs: &[PublicKey]) -> bool {
    match event.kind() {
        Kind::ZapReceipt => {
            let Some(event) = get_zap_request(event) else {
                return false;
            };
            for npub in npubs {
                if event.author() == *npub {
                    debug!("the zapped event id: {}", event.id);
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn get_zap_request(event: &Event) -> Option<Event> {
    let Some(tag) = event
        .tags()
        .iter()
        .find(|t| t.kind() == TagKind::Description)
    else {
        debug!("no description tag found in event {}", event.id());
        return None;
    };

    let Ok(event) = Event::from_json(tag.content().unwrap()) else {
        debug!("description tag is not a valid event");
        return None;
    };
    if let Err(e) = event.verify() {
        debug!("invalid zap request event: {:?}", e);
        return None;
    }

    Some(event)
}

fn get_zap_request_amount(event: &Event) -> u32 {
    let Some(event) = get_zap_request(event) else {
        return 0;
    };

    let Some(tag) = event.tags().iter().find(|t| t.kind() == TagKind::Amount) else {
        debug!("no amount tag found in event {}", event.id());
        return 0;
    };

    tag.content()
        .unwrap_or_default()
        .parse()
        .unwrap_or_default()
}
