use std::collections::HashMap;
use pcsc::*;

mod badge;
mod ndef;

fn main() {
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
                    card_tapped(&ctx, rs.name());
                }
                readers.insert(name, true);
            }
            else if rs.event_state().intersects(State::EMPTY) {
                readers.insert(name, false);
            }
        }
    }
}

fn card_tapped(ctx: &Context, reader: &std::ffi::CStr) {
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
    match badge.get_user_id() {
        Ok(id) => println!("ID is {}", id),
        Err(err) => println!("Error getting user ID: {:?}", err),
    }
}
