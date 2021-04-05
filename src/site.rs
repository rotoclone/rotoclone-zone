use chrono::{DateTime, Datelike, Utc};
use ordinal::Ordinal;
use pulldown_cmark::{html, Options, Parser};
use serde::Deserialize;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs::{create_dir_all, read_to_string, DirEntry, File, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Write},
    num::NonZeroUsize,
    path::PathBuf,
};

/// The name of the directory blog entry files are stored under.
const BLOG_ENTRIES_DIR_NAME: &str = "blog";

/// The template to use to render blog entries that have no template defined in their front matter.
const DEFAULT_BLOG_ENTRY_TEMPLATE_NAME: &str = "blog_entry";

/// The string used to delimit the beginning and end of the front matter
const FRONT_MATTER_DELIMITER: &str = "+++";

const RECENT_BLOG_ENTRIES_LIMIT: usize = 5;
const BLOG_INDEX_PAGE_SIZE: usize = 10;

#[derive(Serialize)]
pub struct IndexContext {
    title: String,
    recent_blog_entries: Vec<BlogEntryStub>,
}

#[derive(Serialize)]
pub struct BlogIndexContext {
    title: String,
    entries: Vec<BlogEntryStub>,
    previous_page: Option<usize>,
    next_page: Option<usize>,
}

#[derive(Serialize)]
struct BlogEntryStub {
    title: String,
    url: String,
    created_at: String,
}

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
    source_file: PathBuf,
    html_content_file: PathBuf,
    pub slug: String,
    pub template_name: String,
}

#[derive(Debug)]
pub struct BlogEntry {
    pub title: String,
    pub metadata: PageMetadata,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl BlogEntry {
    fn to_stub(&self) -> BlogEntryStub {
        BlogEntryStub {
            title: self.title.clone(),
            url: format!("/blog/{}", self.metadata.slug),
            created_at: format_time(self.created_at),
        }
    }
}

impl Site {
    /// Builds the site model from the provided source directory, and puts rendered HTML in the provided HTML directory.
    ///
    /// # Errors
    /// Returns any errors that occur while reading from the file system or parsing file contents.
    pub fn from_dir(source_dir: &PathBuf, html_dir: &PathBuf) -> Result<Site, std::io::Error> {
        let blog_entries_source_dir = source_dir.join(BLOG_ENTRIES_DIR_NAME);
        let blog_entries_html_dir = html_dir.join(BLOG_ENTRIES_DIR_NAME);

        //TODO recursively delete html_dir

        let mut blog_entries = Vec::new();
        for blog_file in blog_entries_source_dir.read_dir()? {
            let blog_file = blog_file?;

            let (front_matter, content_markdown) =
                extract_front_matter_and_content(&blog_file.path())?;
            let html_content_file = write_content_as_html(
                &blog_entries_html_dir,
                blog_file.file_name(),
                &content_markdown,
            )?;

            let metadata = PageMetadata {
                source_file: blog_file.path(),
                html_content_file,
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

        blog_entries.sort_by(|a, b| a.created_at.cmp(&b.created_at).reverse());
        Ok(Site { blog_entries })
    }

    pub fn build_index_context(&self) -> IndexContext {
        let recent_blog_entries = self
            .blog_entries
            .iter()
            .take(RECENT_BLOG_ENTRIES_LIMIT)
            .map(BlogEntry::to_stub)
            .collect();

        IndexContext {
            title: "Sup".to_string(),
            recent_blog_entries,
        }
    }

    pub fn build_blog_index_context(&self, page: NonZeroUsize) -> BlogIndexContext {
        let page = page.get();
        let start_index = (page - 1) * BLOG_INDEX_PAGE_SIZE;
        let entries = self
            .blog_entries
            .iter()
            .skip(start_index)
            .take(BLOG_INDEX_PAGE_SIZE)
            .map(BlogEntry::to_stub)
            .collect();

        let previous_page = match page {
            1 => None,
            _ => Some(page - 1),
        };

        let next_page = if self.blog_entries.len() > (start_index + BLOG_INDEX_PAGE_SIZE) {
            Some(page + 1)
        } else {
            None
        };

        BlogIndexContext {
            title: "The Rotoclone Zone Blog".to_string(),
            entries,
            previous_page,
            next_page,
        }
    }
}

fn default_slug_for_file(file: &DirEntry) -> String {
    file.path()
        .file_stem()
        .unwrap_or(&file.file_name())
        .to_string_lossy()
        .to_string()
}

/// Parses the front matter and the content from the file at the provided location.
///
/// # Errors
/// Returns an error if there are any errors reading the file or parsing the front matter from it.
fn extract_front_matter_and_content(
    file_path: &PathBuf,
) -> Result<(FrontMatter, String), std::io::Error> {
    let file = File::open(file_path)?;
    let mut front_matter_string = "".to_string();
    let mut done_with_front_matter = false;
    let mut content_lines = Vec::new();
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

        if done_with_front_matter {
            content_lines.push(line);
        } else if line == FRONT_MATTER_DELIMITER {
            done_with_front_matter = true;
        } else {
            front_matter_string.push_str(&format!("{}\n", line));
        }
    }

    let front_matter = toml::from_str(&front_matter_string)?;
    Ok((front_matter, content_lines.join("\n")))
}

fn write_content_as_html(
    output_dir: &PathBuf,
    mut file_name: OsString,
    markdown: &str,
) -> Result<PathBuf, std::io::Error> {
    file_name.push(".html");

    let mut output_path = output_dir.clone();
    output_path.push(file_name);

    create_dir_all(output_dir)?;

    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_path)?;
    writeln!(output_file, "{}", markdown_to_html(markdown))?;

    Ok(output_path)
}

/// Converts the provided markdown to HTML.
fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(markdown, options);

    let mut html: String = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut html, parser);

    html
}

#[derive(Serialize)]
pub struct BlogEntryContext {
    title: String,
    tags: Vec<String>,
    created_at: String,
    updated_at: Option<String>,
    entry_content: String,
    previous_entry: Option<BlogEntryStub>,
    next_entry: Option<BlogEntryStub>,
}

impl BlogEntryContext {
    pub fn from_blog_entry(entry: &BlogEntry) -> Result<BlogEntryContext, std::io::Error> {
        Ok(BlogEntryContext {
            title: entry.title.clone(),
            tags: entry.tags.clone(),
            created_at: format_time(entry.created_at),
            updated_at: entry.updated_at.map(format_time),
            entry_content: read_to_string(&entry.metadata.html_content_file)?,
            previous_entry: None, //TODO
            next_entry: None,     //TODO
        })
    }
}

fn format_time(time: DateTime<Utc>) -> String {
    let month = time.format("%B");
    let day = Ordinal(time.day()).to_string();
    let year = time.format("%Y");

    format!("{} {} {}", month, day, year)
}
