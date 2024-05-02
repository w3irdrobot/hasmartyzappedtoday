#![allow(non_snake_case)]

use std::time::Duration;

use dioxus::prelude::*;
use log::{debug, info, LevelFilter};
use nostr_sdk::prelude::{Event, *};

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

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
}

fn main() {
    // Init debug
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();

    launch(App);
}

fn App() -> Element {
    rsx! { Router::<Route> {} }
}

#[component]
fn Home() -> Element {
    let mut hasZapped = use_signal(|| None);
    // marty
    let npub = "npub1guh5grefa7vkay4ps6udxg8lrqxg2kgr3qh9n4gduxut64nfxq0q9y6hjy";
    let npubToCheck = use_signal(|| PublicKey::parse(npub).unwrap());
    let mut name = use_signal(|| "Marty Bent".to_string());
    let mut pfp: Signal<Option<String>> = use_signal(|| None);
    let keys = use_signal(|| Keys::generate());
    let client = use_resource(move || async move {
        debug!("setting up the client");
        let client = Client::new(keys());
        for relay in RELAYS {
            client.add_relay(relay).await.unwrap();
        }
        client.connect().await;
        debug!("client setup complete");
        client
    });
    let _ = use_resource(move || async move {
        debug!("waiting for a client");
        let Some(client) = client() else {
            return;
        };

        let npub: PublicKey = npubToCheck.peek().clone();
        debug!("looking up user {} zaps", npub);
        let mut notifications = client.notifications();
        let zap_filter = Filter::new()
            .kind(Kind::ZapReceipt)
            .since(Timestamp::now() - Duration::from_secs(60 * 60 * 24));
        let zap_p_filter = Filter::new()
            .kind(Kind::ZapReceipt)
            .custom_tag(SingleLetterTag::uppercase(Alphabet::P), vec![npub.to_hex()])
            .since(Timestamp::now() - Duration::from_secs(60 * 60 * 24));
        let zap_sub_id = client.subscribe(vec![zap_filter, zap_p_filter], None).await;
        let profile_filter = Filter::new().kind(Kind::Metadata).author(npub);
        let profile_sub_id = client.subscribe(vec![profile_filter], None).await;

        hasZapped.set(None);

        let mut zapped = false;
        let mut sub_left_count = RELAYS.len();
        loop {
            if sub_left_count == 0 {
                debug!("all connections complete");
                break;
            }
            debug!("waiting for message");
            let RelayPoolNotification::Message { message, .. } =
                notifications.recv().await.unwrap()
            else {
                continue;
            };

            debug!("message received: {:?}", message);
            match message {
                RelayMessage::Event { event, .. } => match event.kind() {
                    Kind::Metadata => {
                        let Ok(metadata) = Metadata::from_json(&event.content) else {
                            debug!("invalid metadata event: {:?}", event.content);
                            continue;
                        };
                        if let Some(display_name) = metadata.display_name {
                            name.set(display_name);
                        }
                        if let Some(picture) = metadata.picture {
                            pfp.set(Some(picture));
                        }
                        client.unsubscribe(profile_sub_id.clone()).await;
                        debug!("metadata set");
                    }
                    Kind::ZapReceipt => {
                        let Some(Tag::Description(description)) = event
                            .tags()
                            .iter()
                            .find(|t| matches!(t, Tag::Description(_)))
                        else {
                            debug!("no description tag found in event {}", event.id());
                            continue;
                        };

                        let Ok(event) = Event::from_json(description) else {
                            debug!("description tag is not a valid event");
                            continue;
                        };
                        if let Err(e) = event.verify() {
                            debug!("invalid zap request event: {:?}", e);
                            continue;
                        }
                        if event.author() != npub {
                            continue;
                        }
                        info!("the zapped event id: {}", event.id);
                        zapped = true;
                        client.unsubscribe(zap_sub_id.clone()).await;
                    }
                    _ => {}
                },
                RelayMessage::EndOfStoredEvents(_) => {
                    sub_left_count -= 1;
                    debug!("connection complete. {} more left", sub_left_count)
                }
                _ => {}
            }
        }

        hasZapped.set(Some(zapped));
        debug!("unsubscribing from all relays");
        client.unsubscribe_all().await;
        debug!("nostr client exiting");
    });

    rsx! {
        main { class: "grid min-h-full place-items-center bg-white px-6 py-24 sm:py-32 lg:px-8",
            div { class: "text-center",
                match hasZapped() {
                    Some(true) => rsx! {
                        h1 {
                            class: "mb-3 text-3xl font-bold tracking-tight text-gray-900 sm:text-5xl",
                            "{name}"
                        }
                        p {
                            class: "text-base font-semibold text-indigo-600",
                            "has zapped today!"
                        }
                    },
                    Some(false) => rsx! {
                        h1 {
                            class: "mb-3 text-3xl font-bold tracking-tight text-gray-900 sm:text-5xl",
                            "{name}"
                        }
                        p {
                            class: "text-base font-semibold text-indigo-600",
                            "has not zapped today."
                        }
                    },
                    None => rsx! {
                        p {
                            class: "text-base font-semibold text-indigo-600 uppercase",
                            "Please wait. We are checking if"
                        }
                        h1 {
                            class: "mt-3 mb-3 text-3xl font-bold tracking-tight text-gray-900 sm:text-5xl",
                            "{name}"
                        }
                        p {
                            class: "text-base font-semibold text-indigo-600",
                            "has zapped today."
                        }
                    },
                }
            }
        }
    }
}
