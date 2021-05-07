use std::num::NonZeroUsize;

use rocket::{response::NamedFile, State};
use rocket_contrib::serve::{crate_relative, Options, StaticFiles};
use rocket_contrib::templates::Template;
use std::path::PathBuf;

#[macro_use]
extern crate rocket;

mod site;

mod updating_site;
use updating_site::*;

mod context;
use context::*;

const ADDITIONAL_STATIC_FILES_DIR_CONFIG_KEY: &str = "static_files_dir";

const SITE_CONTENT_BASE_DIR_CONFIG_KEY: &str = "site_content_base_dir";
const DEFAULT_SITE_CONTENT_BASE_DIR: &str = "./site_content";

const RENDERED_HTML_BASE_DIR_CONFIG_KEY: &str = "rendered_html_base_dir";
const DEFAULT_RENDERED_HTML_BASE_DIR: &str = "./rendered_html";

#[get("/")]
fn index(updating_site: State<UpdatingSite>) -> Template {
    let context = updating_site.site.read().unwrap().build_index_context();
    Template::render("index", &context)
}

#[get("/about")]
fn about(updating_site: State<UpdatingSite>) -> Template {
    let context = updating_site.site.read().unwrap().build_about_context();
    Template::render("about", &context)
}

#[get("/blog?<page>")]
fn get_blog_index(page: Option<NonZeroUsize>, updating_site: State<UpdatingSite>) -> Template {
    let context = updating_site
        .site
        .read()
        .unwrap()
        .build_blog_index_context(page.unwrap_or_else(|| NonZeroUsize::new(1).unwrap()));
    Template::render("blog_index", &context)
}

#[get("/blog/posts/<entry_name>")]
fn get_blog_entry(entry_name: String, updating_site: State<UpdatingSite>) -> Option<Template> {
    let site = &updating_site.site.read().unwrap();
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

#[get("/blog/posts/<entry_name>/<path..>")]
fn get_blog_entry_file(
    entry_name: String,
    path: PathBuf,
    updating_site: State<UpdatingSite>,
) -> Option<NamedFile> {
    let site = &updating_site.site.read().unwrap();
    let entry = site
        .blog_entries
        .iter()
        .find(|entry| entry.metadata.slug == entry_name)?;
    let full_path = entry
        .metadata
        .associated_files
        .iter()
        .find(|file| file.relative_path == path)
        .map(|file| &file.full_path)?;

    //TODO NamedFile::open(full_path).await.ok()
    None
}

#[get("/blog/tags")]
fn get_blog_tags(updating_site: State<UpdatingSite>) -> Template {
    let context = updating_site.site.read().unwrap().build_blog_tags_context();
    Template::render("blog_tags", &context)
}

#[get("/blog/tags/<tag>?<page>")]
fn get_blog_tag(
    tag: String,
    page: Option<NonZeroUsize>,
    updating_site: State<UpdatingSite>,
) -> Option<Template> {
    let context = updating_site
        .site
        .read()
        .unwrap()
        .build_blog_tag_context(tag, page.unwrap_or_else(|| NonZeroUsize::new(1).unwrap()));

    context.map(|x| Template::render("blog_tag", &x))
}

#[get("/blog/feed")]
fn get_blog_feed(updating_site: State<UpdatingSite>) -> Template {
    let context = updating_site.site.read().unwrap().build_blog_feed_context();
    Template::render("feed", &context)
}

#[catch(404)]
fn not_found() -> Template {
    let context = ErrorContext {
        base: BaseContext {
            title: "404".to_string(),
            meta_description: "Not a page".to_string(),
        },
        header: "404".to_string(),
        message: "That's not a page".to_string(),
    };
    Template::render("error", &context)
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let mut rocket = rocket::build()
        .mount(
            "/",
            routes![
                index,
                about,
                get_blog_index,
                get_blog_entry,
                get_blog_tags,
                get_blog_tag,
                get_blog_feed,
            ],
        )
        .mount("/", StaticFiles::from(crate_relative!("static")).rank(10))
        .register("/", catchers![not_found])
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

    println!("Building site...");
    match std::fs::remove_dir_all(&html_base_dir) {
        Ok(()) => (),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => (),
            _ => panic!("error deleting {}: {}", html_base_dir, e),
        },
    };
    let updating_site =
        UpdatingSite::from_dir(PathBuf::from(site_base_dir), PathBuf::from(html_base_dir))
            .unwrap_or_else(|e| panic!("error building site: {:?}", e));
    println!("Site built successfully.");
    rocket = rocket.manage(updating_site);

    if let Ok(dir) = additional_static_files_dir {
        println!("Serving static files from {}", dir);
        rocket = rocket.mount(
            "/",
            StaticFiles::new(dir, Options::Index | Options::DotFiles).rank(9),
        );
    }

    rocket
}
