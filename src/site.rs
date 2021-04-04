use chrono::{DateTime, Datelike, Utc};
use ordinal::Ordinal;
use pulldown_cmark::{html, Options, Parser};
use serde::Deserialize;
use serde::Serialize;
use std::{
    fs::{DirEntry, File},
    io::{BufRead, BufReader, ErrorKind},
    path::PathBuf,
};

/// The name of the directory blog entry files are stored under.
const BLOG_ENTRIES_DIR_NAME: &str = "blog";

/// The template to use to render blog entries that have no template defined in their front matter.
const DEFAULT_BLOG_ENTRY_TEMPLATE_NAME: &str = "blog_entry";

/// The string used to delimit the beginning and end of the front matter
const FRONT_MATTER_DELIMITER: &str = "+++";

#[derive(Debug)]
pub struct Site {
    pub blog_entries: Vec<BlogEntry>,
}

#[derive(Deserialize)]
pub struct FrontMatter {
    slug: Option<String>,
    title: Option<String>,
    template: Option<String>,
    tags: Option<Vec<String>>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct PageMetadata {
    content_file: PathBuf,
    pub slug: String,
    pub template_name: String,
}

#[derive(Debug)]
pub struct BlogEntry {
    title: String,
    pub metadata: PageMetadata,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Site {
    /// Builds the site model from the provided base directory.
    ///
    /// # Errors
    /// Returns any errors that occur while reading from the file system or parsing file contents.
    pub fn from_dir(base_dir: &PathBuf) -> Result<Site, std::io::Error> {
        let blog_entries_dir = base_dir.join(BLOG_ENTRIES_DIR_NAME);
        let mut blog_entries = Vec::new();
        for blog_file in blog_entries_dir.read_dir()? {
            let blog_file = blog_file?;

            let front_matter = extract_front_matter(&blog_file.path())?;
            let metadata = PageMetadata {
                content_file: blog_file.path(),
                slug: front_matter
                    .slug
                    .unwrap_or_else(|| default_slug_for_file(&blog_file)),
                template_name: front_matter
                    .template
                    .unwrap_or_else(|| DEFAULT_BLOG_ENTRY_TEMPLATE_NAME.to_string()),
            };
            let entry = BlogEntry {
                metadata,
                title: front_matter.title.unwrap_or_else(|| "".to_string()),
                tags: front_matter.tags.unwrap_or_default(),
                created_at: front_matter
                    .created_at
                    .unwrap_or(blog_file.metadata()?.created()?.into()),
                updated_at: front_matter.updated_at,
            };

            blog_entries.push(entry);
        }

        Ok(Site { blog_entries })
    }
}

fn default_slug_for_file(file: &DirEntry) -> String {
    file.path()
        .file_stem()
        .unwrap_or(&file.file_name())
        .to_string_lossy()
        .to_string()
}

/// Parses the front matter from the file at the provided location.
///
/// # Errors
/// Returns an error if there are any errors reading the file or parsing the front matter from it.
fn extract_front_matter(file_path: &PathBuf) -> Result<FrontMatter, std::io::Error> {
    let file = File::open(file_path)?;
    let mut front_matter_string = "".to_string();
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;

        if i == 0 {
            if line != FRONT_MATTER_DELIMITER {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "file at {:?} did not start with {}",
                        file_path, FRONT_MATTER_DELIMITER
                    ),
                ));
            }
            continue;
        }

        if line == FRONT_MATTER_DELIMITER {
            break;
        }

        front_matter_string.push_str(&format!("{}\n", line));
    }

    Ok(toml::from_str(&front_matter_string)?)
}

/// Extracts the contents of the provided file (ignoring the front matter) and converts it from markdown to HTML.
///
/// # Errors
/// Returns an error if there are any issues reading from the file.
fn extract_content_as_html(file_path: &PathBuf) -> Result<String, std::io::Error> {
    let file = File::open(file_path)?;
    let mut done_with_front_matter = false;
    let mut file_lines = Vec::new();
    for line in BufReader::new(file).lines().skip(1) {
        let line = line?;
        if done_with_front_matter {
            file_lines.push(line);
        } else if line == FRONT_MATTER_DELIMITER {
            done_with_front_matter = true;
        }
    }

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TABLES);
    let markdown = file_lines.join("\n");
    let parser = Parser::new_ext(&markdown, options);

    let mut html: String = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut html, parser);

    Ok(html)
}

#[derive(Serialize)]
pub struct BlogEntryContext {
    title: String,
    tags: Vec<String>,
    created_at: String,
    updated_at: Option<String>,
    entry_content: String,
}

impl BlogEntryContext {
    pub fn from_blog_entry(entry: &BlogEntry) -> Result<BlogEntryContext, std::io::Error> {
        Ok(BlogEntryContext {
            title: entry.title.clone(),
            tags: entry.tags.clone(),
            created_at: format_time(entry.created_at),
            updated_at: entry.updated_at.map(format_time),
            entry_content: extract_content_as_html(&entry.metadata.content_file)?,
        })
    }
}

fn format_time(time: DateTime<Utc>) -> String {
    let month = time.format("%B");
    let day = Ordinal(time.day()).to_string();
    let year = time.format("%Y");

    format!("{} {} {}", month, day, year)
}
