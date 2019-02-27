#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate mongodb;
#[macro_use] extern crate wither_derive;

use rocket::State;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use mongodb::{ThreadedClient, doc};
use wither::model::Model;
use hackgt_nfc::api::CheckinAPI;

pub type DB = std::sync::Arc<mongodb::db::DatabaseInner>;

mod models;
use models::Device;
mod api;
mod auth;
use auth::AuthenticatedUser;

#[get("/")]
fn index(user: AuthenticatedUser, db: State<DB>) -> Template {
	let devices = match Device::find(db.clone(), None, None) {
		Ok(result) => result,
		// Driver returns an error if no documents are found
		Err(_) => Vec::new(),
	};

	Template::render("index", &json!({
		"devices": devices
	}))
}

fn main() {
	println!("Logging into HackGT Check-In API...");
	let checkin_api = match std::env::var("CHECKIN_TOKEN") {
		Ok(token) => CheckinAPI::from_token(token),
		Err(_) => {
			let username = std::env::var("CHECKIN_USERNAME").expect("Missing or invalid check-in API username");
			let password = std::env::var("CHECKIN_PASSWORD").expect("Missing or invalid check-in API password");
			CheckinAPI::login(&username, &password).unwrap()
		}
	};

	let mongo_url = std::env::var("MONGO_URL").unwrap_or("mongodb://localhost".to_owned());
	let db_name = std::env::var("MONGO_DB").unwrap_or("checkin-embedded".to_owned());
	let db = mongodb::Client::with_uri(&mongo_url).expect("Failed to connect to the MongoDB server").db(&db_name);

	rocket::ignite()
		.attach(Template::fairing())
		.mount("/", routes![index])
		.mount("/auth", routes![
			auth::login,
			auth::process_login
		])
		.mount("/api", routes![
			api::initialize,
			api::create_credentials,
			api::get_tag,
		])
		.mount("/css", StaticFiles::from("/ui/css"))
		.mount("/js", StaticFiles::from("/ui/js"))
		.register(catchers![
			auth::unauthorized_redirect
		])
		.manage(db)
		.manage(checkin_api)
		.launch();
}
