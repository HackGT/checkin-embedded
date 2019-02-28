use hackgt_nfc::api::CheckinAPI;
use hackgt_nfc::nfc::{ handle_cards, NFCBadge };
use chrono::DateTime;
use std::sync::Arc;

mod api;
use api::{ ManagerAPI, ManagedStatus };
mod crypto;
mod peripherals;

fn main() {
    // We'll be using this notifer on the main thread + the tag update thread so it needs to be behind an Arc
    let notifier_arc = Arc::new(peripherals::Notifier::start(0x70, 0x71, 18));
    let notifier = notifier_arc.clone();
    notifier.scroll_text_speed("Logging in...", 10);

    // Bootstrap connection to manager
    let manager = ManagerAPI::new();
    let signer = crypto::Signer::load();
    let api: CheckinAPI = match manager.initialize() {
        Ok(ManagedStatus::AuthorizedHasCredentials) => {
            // Use existing credentials
            let credentials = signer.get_api_credentials();
            match CheckinAPI::login(&credentials.username, &credentials.password) {
                Ok(api) => api,
                // This can happen if someone accidentally deletes our account in the checkin2 admin page
                Err(hackgt_nfc::api::Error::Message("Invalid username or password")) => {
                    let response = manager.create_credentials().unwrap();
                    if !response.success {
                        eprintln!("Invalid credentials even though server thinks we already have an account: {:?} ({:?})", response.error, response.details);
                        std::process::exit(1);
                    }
                    CheckinAPI::login(&credentials.username, &credentials.password).expect("Invalid credentials after server apparently created our account again")
                },
                Err(err) => panic!(err),
            }
        },
        Ok(ManagedStatus::AuthorizedNoCredentials) => {
            // Request server create an account with our credentials
            let response = manager.create_credentials().unwrap();
            if !response.success {
                eprintln!("Failed to create credentials: {:?} ({:?})", response.error, response.details);
                std::process::exit(1);
            }
            let credentials = signer.get_api_credentials();
            CheckinAPI::login(&credentials.username, &credentials.password).expect("Invalid credentials after server apparently created our account")
        },
        Ok(ManagedStatus::Unauthorized) => {
            eprintln!("Check-in instance <{}> has been denied access in the manager UI", manager.get_name());
            notifier.scroll_text("Denied access in manager UI");
            notifier.scroll_text_speed("Exiting...", 30);
            std::thread::sleep(std::time::Duration::from_secs(30));
            std::process::exit(1)
        },
        Ok(ManagedStatus::Pending) => {
            eprintln!("Check-in instance <{}> must be approved in the manager UI before use", manager.get_name());
            notifier.scroll_text("Must approve device in manager UI before use");
            notifier.scroll_text_speed("Exiting...", 30);
            std::thread::sleep(std::time::Duration::from_secs(30));
            std::process::exit(1)
        },
        Err(err) => {
            panic!(err)
        }
    };
    // Spawns a thread to check for tag updates
    manager.start_polling_for_tag(30, notifier_arc.clone());

    // Signify that we're logged in and ready to go
    notifier.flash_multiple(false, vec![500, 200, 100, 0]);
    notifier.flash_multiple(true, vec![500, 200, 100, 0]);
    notifier.beep(vec![
        peripherals::Tone::new(261.63, 500),
        peripherals::Tone::new(0.0, 200),
        peripherals::Tone::new(523.25, 100),
    ]);

    // Set up card polling
    let handler_thread = handle_cards(move |card, _reader, _reader_index| {
        let badge = NFCBadge::new(&card);
        badge.set_buzzer(false).unwrap();

        let current_tag = Arc::clone(&manager.current_tag);
        let current_tag = current_tag.read().unwrap();

        // THIS IS SLOWWWWW
        // My 3:40am guess is that the notifier is causing some kind of hold on the &card argument
        // fake news
        // my 3:42am knowledges says that this is somehow the cause:
        // badge.get_user_id().unwrap();
        // Only seems to be a problem on Linux (pcsclite)
        // I ran the same code on Windows and it was significantly faster

        match badge.get_user_id() {
            Ok(_) if current_tag.is_none() => {
                notifier.flash_multiple(false, vec![200, 100, 200, 0]);
                notifier.beep(vec![
                    peripherals::Tone::new(261.63, 500),
                ]);
                notifier.scroll_text("No check-in tag defined by manager");
            },
            Ok(id) => {
                match api.check_in(&id, current_tag.as_ref().unwrap()) {
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
    });
    handler_thread.join().unwrap();
}

fn get_relative_time(iso_time: &str) -> String {
    let time = match DateTime::parse_from_rfc3339(iso_time) {
        Ok(time) => time,
        Err(_) => return String::from("invalid time ago"),
    };
    let now = chrono::Local::now();
    let duration = now.signed_duration_since(time);

    fn pluralizer(num: i64, label: &str) -> String {
        format!("{} {}{} ago", num, label, if num == 1 { "" } else { "s" })
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
