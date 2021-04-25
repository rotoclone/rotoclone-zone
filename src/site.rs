use anyhow::Context;
use chrono::{DateTime, Utc};
use pulldown_cmark::{html, Options, Parser};
use serde::Deserialize;
use std::{
    ffi::OsString,
    fs::{create_dir_all, DirEntry, File, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Write},
    path::{Path, PathBuf},
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

#[derive(Debug, PartialEq)]
pub struct PageMetadata {
    source_file: PathBuf,
    pub html_content_file: PathBuf,
    pub slug: String,
    pub template_name: String,
}

#[derive(Debug, PartialEq)]
pub struct BlogEntry {
    pub title: String,
    pub metadata: PageMetadata,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Site {
    /// Builds the site model from the provided source directory, and puts rendered HTML in the provided HTML directory.
    ///
    /// # Errors
    /// Returns any errors that occur while reading from the file system or parsing file contents.
    pub fn from_dir(source_dir: &Path, html_dir: &Path) -> anyhow::Result<Site> {
        let blog_entries_source_dir = source_dir.join(BLOG_ENTRIES_DIR_NAME);
        let blog_entries_html_dir = html_dir.join(BLOG_ENTRIES_DIR_NAME);

        let mut blog_entries = Vec::new();
        for blog_file in blog_entries_source_dir.read_dir().with_context(|| {
            format!(
                "error reading from {}",
                blog_entries_source_dir.to_string_lossy()
            )
        })? {
            let blog_file = blog_file.with_context(|| {
                format!(
                    "error reading from {}",
                    blog_entries_source_dir.to_string_lossy()
                )
            })?;

            let (front_matter, content_markdown) =
                extract_front_matter_and_content(&blog_file.path()).with_context(|| {
                    format!(
                        "error extracting front matter from {}",
                        blog_file.path().to_string_lossy()
                    )
                })?;
            let html_content_file = write_content_as_html(
                &blog_entries_html_dir,
                blog_file.file_name(),
                &content_markdown,
            )
            .with_context(|| {
                format!(
                    "error writing content of {} as HTML",
                    blog_file.path().to_string_lossy()
                )
            })?;

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
                created_at: front_matter.created_at.unwrap_or(
                    blog_file
                        .metadata()
                        .with_context(|| {
                            format!(
                                "error getting metadata for {}",
                                blog_file.path().to_string_lossy()
                            )
                        })?
                        .created()
                        .with_context(|| {
                            format!(
                                "error getting created at for {}",
                                blog_file.path().to_string_lossy()
                            )
                        })?
                        .into(),
                ),
                updated_at: front_matter.updated_at,
            };

            blog_entries.push(entry);
        }

        blog_entries.sort_by(|a, b| a.created_at.cmp(&b.created_at).reverse());
        Ok(Site { blog_entries })
    }
}

/// Determines the default slug for the provided file.
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
    file_path: &Path,
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

/// Converts the provided markdown to HTML and writes it to a file.
/// Returns the path to the written file.
///
/// # Arguments
/// * `output_dir` - The directory to write the HTML file to.
/// * `file_name` - The name of the source file the markdown is from.
/// * `markdown` - The markdown to convert to HTML.
///
/// # Errors
/// Returns any errors encountered while writing the file.
fn write_content_as_html(
    output_dir: &Path,
    mut file_name: OsString,
    markdown: &str,
) -> Result<PathBuf, std::io::Error> {
    file_name.push(".html");

    let mut output_path = output_dir.to_owned();
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
