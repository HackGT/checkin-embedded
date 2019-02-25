use std::collections::HashMap;
use pcsc::*;
use hackgt_nfc::api::CheckinAPI;
use chrono::DateTime;

mod badge;
mod ndef;
mod api;
use api::{ ManagerAPI, ManagedStatus };
mod crypto;
mod peripherals;

fn main() {
    let notifier = peripherals::Notifier::start(0x70, 0x71, 18);
    notifier.scroll_text_speed("Logging in...", 10);

    // Bootstrap connection to manager
    let api = CheckinAPI::login("ryan", "test").unwrap();
    // let manager = ManagerAPI::new();
    // let api: CheckinAPI = match manager.initialize() {
    //     Ok(ref status) if *status == ManagedStatus::AuthorizedHasCredentials => {
    //         // Request
    //         CheckinAPI::login("ryan", "test").unwrap()
    //     },
    //     Ok(ref status) if *status == ManagedStatus::AuthorizedNoCredentials => {
    //         // Request credentials from server;
    //         CheckinAPI::login("ryan", "test").unwrap()
    //     },
    //     Ok(ref status) if *status == ManagedStatus::Unauthorized => {
    //         eprintln!("Check-in instance <{}> has been denied access in the manager UI", manager.get_name());
    //         std::process::exit(1)
    //     },
    //     Ok(_) => {
    //         eprintln!("Check-in instance <{}> must be approved in the manager UI before use", manager.get_name());
    //         std::process::exit(1)
    //     },
    //     Err(err) => {
    //         panic!("{:?}", err)
    //     }
    // };

    // Signify that we're logged in and ready to go
    notifier.flash_multiple(false, vec![500, 200, 100, 0]);
    notifier.flash_multiple(true, vec![500, 200, 100, 0]);
    notifier.beep(vec![
        peripherals::Tone::new(261.63, 500),
        peripherals::Tone::new(0.0, 200),
        peripherals::Tone::new(523.25, 100),
    ]);

    let ctx = Context::establish(Scope::User).expect("Failed to establish context");

    let mut readers_buf = [0; 2048];
    let mut reader_states = vec![
        // Listen for reader insertions/removals, if supported
        ReaderState::new(PNP_NOTIFICATION(), State::UNAWARE),
    ];
    // Keeps track of which readers have an active card
    let mut readers = HashMap::new();
    loop {
        // Remove dead readers
        fn is_invalid(rs: &ReaderState) -> bool {
            rs.event_state().intersects(State::UNKNOWN | State::IGNORE)
        }
        reader_states.retain(|rs| !is_invalid(rs));

        // Add new readers
        let names = ctx.list_readers(&mut readers_buf).expect("Failed to list readers");
        for name in names {
            // Ignore the pseudo reader created by Windows Hello
            if !reader_states.iter().any(|rs| rs.name() == name) && !name.to_str().unwrap().contains("Windows Hello") {
                println!("Adding {:?}", name);
                reader_states.push(ReaderState::new(name, State::UNAWARE));
            }
        }

        // Update the view of the state to wait on
        for rs in &mut reader_states {
            rs.sync_current_state();
        }

        // Wait until the state changes
        ctx.get_status_change(None, &mut reader_states).expect("Failed to get status change");
        for rs in &reader_states {
            if rs.name() == PNP_NOTIFICATION() { continue; }

            let name = rs.name().to_owned();
            // Debounce repeated events
            if rs.event_state().intersects(State::PRESENT) {
                if !readers.get(&name).unwrap_or(&false) {
                    card_tapped(&ctx, rs.name(), &api, &notifier);
                }
                readers.insert(name, true);
            }
            else if rs.event_state().intersects(State::EMPTY) {
                readers.insert(name, false);
            }
        }
    }
}

fn card_tapped(ctx: &Context, reader: &std::ffi::CStr, api: &CheckinAPI, notifier: &peripherals::Notifier) {
    // Connect to the card.
    let card = match ctx.connect(reader, ShareMode::Shared, Protocols::ANY) {
        Ok(card) => card,
        Err(Error::NoSmartcard) => {
            eprintln!("A smartcard is not present in the reader");
            return;
        }
        Err(err) => {
            eprintln!("Failed to connect to card: {}", err);
            std::process::exit(1);
        }
    };

    let badge = badge::NFCBadge::new(&card);
    badge.set_buzzer(false).unwrap();

    // THIS IS SLOWWWWW
    // My 3:40am guess is that the notifier is causing some kind of hold on the &card argument
    // fake news
    // my 3:42am knowledges says that this is somehow the cause:
    // badge.get_user_id().unwrap();

    match badge.get_user_id() {
        Ok(id) => {
            match api.check_in(&id, "123") {
                Ok((success, user, tag)) => {
                    if success {
                        notifier.flash(true, 500);
                        notifier.beep(vec![
                            peripherals::Tone::new(1046.50, 100),
                        ]);
                        println!("Checked in {}", &user.name);
                    }
                    else {
                        notifier.flash(false, 500);
                        notifier.beep(vec![
                            peripherals::Tone::new(261.63, 500),
                            peripherals::Tone::new(0.0, 200),
                            peripherals::Tone::new(261.63, 500),
                        ]);
                        if let Some(last_checkin) = tag.last_successful_checkin {
                            let time = get_relative_time(&last_checkin.checked_in_date);
                            notifier.scroll_text(&time);
                        }
                        else {
                            notifier.scroll_text("Already checked in");
                        }
                    }
                },
                Err(hackgt_nfc::api::Error::Message("Invalid user ID on badge")) => {
                    notifier.flash(false, 500);
                    notifier.beep(vec![
                        peripherals::Tone::new(261.63, 500),
                        peripherals::Tone::new(0.0, 200),
                        peripherals::Tone::new(261.63, 500),
                    ]);
                    notifier.scroll_text("Invalid user ID on badge");
                },
                Err(_err) => {
                    notifier.flash(false, 500);
                    notifier.beep(vec![
                        peripherals::Tone::new(261.63, 500),
                        peripherals::Tone::new(0.0, 200),
                        peripherals::Tone::new(261.63, 500),
                    ]);
                    notifier.scroll_text("API error");
                }
            };
        },
        Err(err) => {
            println!("Error getting user ID: {:?}", err);
            notifier.flash_multiple(false, vec![200, 100, 200, 0]);
            notifier.beep(vec![
                peripherals::Tone::new(261.63, 500),
            ]);
            notifier.scroll_text("Try again");
        }
    };
}

fn get_relative_time(iso_time: &str) -> String {
    let time = match DateTime::parse_from_rfc3339(iso_time) {
        Ok(time) => time,
        Err(err) => return String::from("invalid time ago"),
    };
    let now = chrono::Local::now();
    let duration = now.signed_duration_since(time);

    fn pluralizer(num: i64, label: &str) -> String {
        format!("{} {}{} ago", num, label, if num == 1 { "s" } else { "" })
    }

    let weeks = duration.num_weeks();
    if weeks > 0 {
        return pluralizer(weeks, "week");
    }
    let days = duration.num_days();
    if days > 0 {
        return pluralizer(days, "day");
    }
    let hours = duration.num_hours();
    if hours > 0 {
        return pluralizer(hours, "hour");
    }
    let minutes = duration.num_minutes();
    if minutes > 0 {
        return pluralizer(minutes, "minute");
    }
    pluralizer(duration.num_seconds(), "second")
}
