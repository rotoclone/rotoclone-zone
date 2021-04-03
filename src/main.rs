use rocket::State;
use rocket_contrib::serve::{crate_relative, Options, StaticFiles};
use rocket_contrib::templates::Template;
use serde::Serialize;

#[macro_use]
extern crate rocket;

mod site;
use site::*;

const ADDITIONAL_STATIC_FILES_DIR_CONFIG_KEY: &str = "static_files_dir";
const SITE_CONTENT_BASE_DIR_CONFIG_KEY: &str = "site_content_base_dir";
const DEFAULT_SITE_CONTENT_BASE_DIR: &str = "./site_content";

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

#[get("/blog/<entry_name>")]
fn get_blog_entry(entry_name: String, site: State<Site>) -> Option<Template> {
    let entry = site
        .blog_entries
        .iter()
        .find(|entry| entry.metadata.slug == entry_name);

    entry.map(|x| {
        Template::render(
            x.metadata.template_name.clone(),
            BlogEntryContext::from_blog_entry(&x),
        )
    })
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
        .mount("/", routes![index, get_blog_entry])
        .mount("/", StaticFiles::from(crate_relative!("static")).rank(10))
        .register(catchers![not_found])
        .attach(Template::fairing());

    let config = rocket.figment();
    let additional_static_files_dir =
        config.extract_inner::<String>(ADDITIONAL_STATIC_FILES_DIR_CONFIG_KEY);
    let site_base_dir = config
        .extract_inner::<String>(SITE_CONTENT_BASE_DIR_CONFIG_KEY)
        .unwrap_or_else(|_| DEFAULT_SITE_CONTENT_BASE_DIR.to_string());

    let site = Site::from_dir(&site_base_dir.into()).expect("error building site");
    println!("Built site: {:?}", site); //TODO remove
    rocket = rocket.manage(site);

    if let Ok(dir) = additional_static_files_dir {
        println!("Serving static files from {}", dir);
        rocket = rocket.mount(
            "/",
            StaticFiles::new(dir, Options::Index | Options::DotFiles).rank(9),
        );
    }

    rocket
}
