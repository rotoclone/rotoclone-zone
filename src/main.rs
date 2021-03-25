use rocket_contrib::serve::{crate_relative, StaticFiles};
use rocket_contrib::templates::Template;
use serde::Serialize;

#[macro_use]
extern crate rocket;

#[derive(Serialize)]
struct IndexContext {
    title: String,
    header: String,
    items: Vec<String>,
}

#[get("/")]
fn index() -> Template {
    let context = IndexContext {
        title: "Sup".to_string(),
        header: "You have entered The Rotoclone Zone".to_string(),
        items: vec!["boop".to_string(), "doop".to_string(), "floop".to_string()],
    };
    Template::render("index", &context)
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index])
        .mount("/", StaticFiles::from(crate_relative!("static")))
        .attach(Template::fairing())
}
