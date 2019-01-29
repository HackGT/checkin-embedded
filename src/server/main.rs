#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;

use std::collections::HashMap;

use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

#[get("/")]
fn index() -> Template {
	let context: HashMap<&str, &str> = HashMap::new();
	Template::render("index", &context)
}

fn main() {
	rocket::ignite()
		.attach(Template::fairing())
		.mount("/", routes![index])
		.mount("/css", StaticFiles::from("/ui/css"))
		.launch();
}
