use rocket_contrib::serve::{crate_relative, Options, StaticFiles};
use rocket_contrib::templates::Template;
use serde::Serialize;

#[macro_use]
extern crate rocket;

const ADDITIONAL_STATIC_FILES_DIR_CONFIG_KEY: &str = "static_files_dir";

#[derive(Serialize)]
struct IndexContext {
    title: String,
    header: String,
    items: Vec<String>,
}

#[derive(Serialize)]
struct ErrorContext {
    title: String,
    header: String,
    message: String,
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

#[catch(404)]
fn not_found() -> Template {
    let context = ErrorContext {
        title: "404".to_string(),
        header: "404".to_string(),
        message: "That's not a page".to_string(),
    };
    Template::render("error", &context)
}

#[launch]
fn rocket() -> rocket::Rocket {
    let mut rocket = rocket::ignite()
        .mount("/", routes![index])
        .mount("/", StaticFiles::from(crate_relative!("static")).rank(10))
        .register(catchers![not_found])
        .attach(Template::fairing());

    let config = rocket.figment();

    if let Ok(dir) = config.extract_inner::<String>(ADDITIONAL_STATIC_FILES_DIR_CONFIG_KEY) {
        println!("Serving static files from {}", dir);
        rocket = rocket.mount(
            "/",
            StaticFiles::new(dir, Options::Index | Options::DotFiles).rank(9),
        );
    }

    rocket
}
