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
    greeting("there".to_string())
}

#[get("/<name>")]
fn greeting(name: String) -> Template {
    let context = IndexContext {
        title: "Sup".to_string(),
        header: format!("Hello {}", name),
        items: vec!["boop".to_string(), "doop".to_string(), "floop".to_string()],
    };
    Template::render("index", &context)
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, greeting])
        .attach(Template::fairing())
}
