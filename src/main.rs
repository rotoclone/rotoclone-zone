use std::num::NonZeroUsize;

use rocket::State;
use rocket_contrib::serve::{crate_relative, Options, StaticFiles};
use rocket_contrib::templates::Template;
use serde::Serialize;

#[macro_use]
extern crate rocket;

mod site;
use site::*;

mod context;
use context::*;

const ADDITIONAL_STATIC_FILES_DIR_CONFIG_KEY: &str = "static_files_dir";

const SITE_CONTENT_BASE_DIR_CONFIG_KEY: &str = "site_content_base_dir";
const DEFAULT_SITE_CONTENT_BASE_DIR: &str = "./site_content";

const RENDERED_HTML_BASE_DIR_CONFIG_KEY: &str = "rendered_html_base_dir";
const DEFAULT_RENDERED_HTML_BASE_DIR: &str = "./rendered_html";

#[derive(Serialize)]
struct ErrorContext {
    title: String,
    header: String,
    message: String,
}

#[get("/")]
fn index(site: State<Site>) -> Template {
    let context = site.build_index_context();
    Template::render("index", &context)
}

#[get("/blog?<page>")]
fn get_blog_index(page: Option<NonZeroUsize>, site: State<Site>) -> Template {
    let context =
        site.build_blog_index_context(page.unwrap_or_else(|| NonZeroUsize::new(1).unwrap()));
    Template::render("blog_index", &context)
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
            site.build_blog_entry_context(&x)
                .unwrap_or_else(|e| panic!("error rendering blog entry {}: {}", entry_name, e)),
        )
    })
}

#[get("/blog/tags")]
fn get_blog_tags(site: State<Site>) -> Template {
    //let context = site.build_blog_tags_context(q);
    //Template::render("blog_tags", &context)
    unimplemented!() //TODO
}

#[get("/blog/tags/<tag>")]
fn get_blog_tag(tag: String, site: State<Site>) -> Template {
    //let context = site.build_blog_tag_context(q);
    //Template::render("blog_tag", &context)
    unimplemented!() //TODO
}

#[get("/blog/search?<q>")]
fn get_blog_search(q: Option<String>, site: State<Site>) -> Template {
    //let context = site.build_blog_search_context(q);
    //Template::render("blog_search", &context)
    unimplemented!() //TODO
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
        .mount(
            "/",
            routes![
                index,
                get_blog_index,
                get_blog_entry,
                get_blog_tags,
                get_blog_tag,
                get_blog_search
            ],
        )
        .mount("/", StaticFiles::from(crate_relative!("static")).rank(10))
        .register(catchers![not_found])
        .attach(Template::fairing());

    let config = rocket.figment();
    let additional_static_files_dir =
        config.extract_inner::<String>(ADDITIONAL_STATIC_FILES_DIR_CONFIG_KEY);
    let site_base_dir = config
        .extract_inner::<String>(SITE_CONTENT_BASE_DIR_CONFIG_KEY)
        .unwrap_or_else(|_| DEFAULT_SITE_CONTENT_BASE_DIR.to_string());
    let html_base_dir = config
        .extract_inner::<String>(RENDERED_HTML_BASE_DIR_CONFIG_KEY)
        .unwrap_or_else(|_| DEFAULT_RENDERED_HTML_BASE_DIR.to_string());

    let site =
        Site::from_dir(&site_base_dir.into(), &html_base_dir.into()).expect("error building site");
    println!("Built site: {:#?}", site); //TODO remove?
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
