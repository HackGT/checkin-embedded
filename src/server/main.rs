#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate mongodb;
#[macro_use] extern crate wither_derive;

use rocket::State;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use std::collections::HashMap;
use mongodb::{ThreadedClient, doc};
use wither::model::Model;

pub type DB = std::sync::Arc<mongodb::db::DatabaseInner>;

mod models;
use models::Device;
mod api;

#[get("/")]
fn index(db: State<DB>) -> Template {
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
	let mongo_url = std::env::var("MONGO_URL").unwrap_or("mongodb://localhost".to_owned());
	let db_name = std::env::var("MONGO_DB").unwrap_or("checkin-embedded".to_owned());
	let db = mongodb::Client::with_uri(&mongo_url).expect("Failed to connect to the MongoDB server").db(&db_name);

	rocket::ignite()
		.attach(Template::fairing())
		.mount("/", routes![index])
		.mount("/api", routes![api::initialize])
		.mount("/css", StaticFiles::from("/ui/css"))
		.manage(db)
		.launch();
}
