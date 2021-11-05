use anyhow::{bail, Context};
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

/// The name of the file a blog entry's content is in.
const BLOG_CONTENT_FILE_NAME: &str = "content.md";

/// The template to use to render blog entries that have no template defined in their front matter.
const DEFAULT_BLOG_ENTRY_TEMPLATE_NAME: &str = "blog_entry";

/// Whether comments should be enabled on blog entries by default.
const DEFAULT_COMMENTS_ENABLED: bool = true;

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
    description: Option<String>,
    template: Option<String>,
    tags: Option<Vec<String>>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    comments_enabled: Option<bool>,
    external_discussions: Option<Vec<ExternalDiscussion>>,
}

#[derive(Debug, PartialEq)]
pub struct PageMetadata {
    source_file: PathBuf,
    pub associated_files: Vec<AssociatedFile>,
    pub html_content_file: PathBuf,
    pub slug: String,
    pub template_name: String,
}

#[derive(Debug, PartialEq)]
pub struct AssociatedFile {
    pub relative_path: PathBuf,
    pub full_path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub struct BlogEntry {
    pub title: String,
    pub description: String,
    pub metadata: PageMetadata,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub comments_enabled: bool,
    pub external_discussions: Vec<ExternalDiscussion>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct ExternalDiscussion {
    pub name: String,
    pub url: String,
}

impl Site {
    /// Builds the site model from the provided source directory, and puts rendered HTML in the provided HTML directory.
    ///
    /// # Errors
    /// Returns any errors that occur while reading from the file system or parsing file contents.
    pub fn from_dir(source_dir: &Path, html_dir: &Path) -> anyhow::Result<Site> {
        let blog_entries_source_dir = source_dir.join(BLOG_ENTRIES_DIR_NAME);
        let blog_entries_html_dir = html_dir.join(BLOG_ENTRIES_DIR_NAME);

        let mut blog_entries: Vec<BlogEntry> = Vec::new();
        for file in blog_entries_source_dir.read_dir().with_context(|| {
            format!(
                "error reading from {}",
                blog_entries_source_dir.to_string_lossy()
            )
        })? {
            let file = file.with_context(|| {
                format!(
                    "error reading from {}",
                    blog_entries_source_dir.to_string_lossy()
                )
            })?;

            if is_dir(&file)? {
                let entry = parse_entry_dir(&file, &blog_entries_html_dir)?;
                if blog_entries
                    .iter()
                    .any(|existing_entry| entry.metadata.slug == existing_entry.metadata.slug)
                {
                    bail!(
                        "Blog entry in {} has non-unique slug: {}",
                        file.path().to_string_lossy(),
                        entry.metadata.slug
                    );
                }
                blog_entries.push(entry);
            }
        }

        blog_entries.sort_by(|a, b| a.created_at.cmp(&b.created_at).reverse());
        Ok(Site { blog_entries })
    }
}

/// Determines whether the provided `DirEntry` is a directory.
fn is_dir(file: &DirEntry) -> anyhow::Result<bool> {
    Ok(file
        .file_type()
        .with_context(|| {
            format!(
                "error determining file type of {}",
                file.path().to_string_lossy()
            )
        })?
        .is_dir())
}

/// Parses a directory into a `BlogEntry`.
///
/// # Arguments
/// * `dir` - The directory to parse.
/// * `html_dir` - The directory to store the rendered HTML in.
fn parse_entry_dir(dir: &DirEntry, html_dir: &Path) -> anyhow::Result<BlogEntry> {
    let content_file_path = dir.path().join(BLOG_CONTENT_FILE_NAME);

    let (front_matter, content_markdown) = extract_front_matter_and_content(&content_file_path)
        .with_context(|| {
            format!(
                "error extracting front matter from {}",
                content_file_path.to_string_lossy()
            )
        })?;

    let html_content_file = write_content_as_html(html_dir, dir.file_name(), &content_markdown)
        .with_context(|| {
            format!(
                "error writing content of {} as HTML",
                content_file_path.to_string_lossy()
            )
        })?;

    let associated_files = find_associated_files(dir, &dir.path(), &content_file_path)?;

    let created_at = front_matter.created_at.unwrap_or(
        content_file_path
            .metadata()
            .with_context(|| {
                format!(
                    "error getting metadata for {}",
                    content_file_path.to_string_lossy()
                )
            })?
            .created()
            .with_context(|| {
                format!(
                    "error getting created at for {}",
                    content_file_path.to_string_lossy()
                )
            })?
            .into(),
    );

    let metadata = PageMetadata {
        source_file: content_file_path,
        associated_files,
        html_content_file,
        slug: front_matter
            .slug
            .unwrap_or_else(|| default_slug_for_file(dir)),
        template_name: front_matter
            .template
            .unwrap_or_else(|| DEFAULT_BLOG_ENTRY_TEMPLATE_NAME.to_string()),
    };
    Ok(BlogEntry {
        metadata,
        title: front_matter.title.unwrap_or_else(|| "".to_string()),
        description: front_matter.description.unwrap_or_else(|| "".to_string()),
        tags: front_matter.tags.unwrap_or_default(),
        created_at,
        updated_at: front_matter.updated_at,
        comments_enabled: front_matter
            .comments_enabled
            .unwrap_or(DEFAULT_COMMENTS_ENABLED),
        external_discussions: front_matter.external_discussions.unwrap_or_else(Vec::new),
    })
}

/// Recursively finds all the files associated with a blog entry, starting in `dir`.
/// Relative paths in the returned `AssociatedFile`s will be relative to `base_path`.
/// Any file with a path matching `content_file_path` will be ignored.
fn find_associated_files(
    dir: &DirEntry,
    base_path: &Path,
    content_file_path: &Path,
) -> anyhow::Result<Vec<AssociatedFile>> {
    let mut associated_files = Vec::new();
    for file in dir
        .path()
        .read_dir()
        .with_context(|| format!("error reading from {}", dir.path().to_string_lossy()))?
    {
        let file =
            file.with_context(|| format!("error reading from {}", dir.path().to_string_lossy()))?;

        if file
            .file_type()
            .with_context(|| format!("error getting type of {}", file.path().to_string_lossy()))?
            .is_dir()
        {
            associated_files.extend(find_associated_files(&file, base_path, content_file_path)?);
        } else {
            let path = file.path();
            if path != *content_file_path {
                associated_files.push(AssociatedFile {
                    relative_path: path.strip_prefix(base_path)?.to_path_buf(),
                    full_path: path,
                });
            }
        }
    }

    Ok(associated_files)
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
    //TODO add width and height attributes to img tags to reduce reflow

    let mut html: String = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut html, parser);

    html
}
