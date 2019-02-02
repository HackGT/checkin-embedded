#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use std::collections::HashMap;
use mongodb::{ThreadedClient, doc};

mod api;

#[get("/")]
fn index() -> Template {
	let context: HashMap<&str, &str> = HashMap::new();
	Template::render("index", &context)
}

fn main() {
	// let mongo_url = std::env::var("MONGO_URL").unwrap_or("mongodb://localhost".to_owned());
	// let db_name = std::env::var("MONGO_DB").unwrap_or("checkin-embedded".to_owned());
	// let db = mongodb::Client::with_uri(&mongo_url).expect("Failed to connect to the MongoDB server").db(&db_name);

	rocket::ignite()
		.attach(Template::fairing())
		.mount("/", routes![index])
		.mount("/api", routes![api::initialize])
		.mount("/css", StaticFiles::from("/ui/css"))
		.launch();
}
