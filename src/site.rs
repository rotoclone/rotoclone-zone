use serde::Deserialize;
use serde::Serialize;
use std::{
    fs::{DirEntry, File},
    io::{BufRead, BufReader, ErrorKind},
    path::PathBuf,
};

use chrono::{DateTime, Utc};

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

#[derive(Serialize)]
pub struct BlogEntryContext {
    title: String,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    entry_content: String,
}

impl BlogEntryContext {
    pub fn from_blog_entry(entry: &BlogEntry) -> BlogEntryContext {
        //TODO parse content
        // let entry_content = entry.metadata.content_file
        let entry_content = "<h1>Yo</h1><p>dis a blog entry</p>".to_string();

        BlogEntryContext {
            title: entry.title.clone(),
            tags: entry.tags.clone(),
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            entry_content,
        }
    }
}
