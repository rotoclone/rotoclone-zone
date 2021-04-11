use chrono::{DateTime, Datelike, Utc};
use ordinal::Ordinal;
use serde::Serialize;
use std::{fs::read_to_string, num::NonZeroUsize};

use crate::site::{BlogEntry, Site};

const RECENT_BLOG_ENTRIES_LIMIT: usize = 5;
const BLOG_INDEX_PAGE_SIZE: usize = 10;

#[derive(Serialize)]
pub struct BlogEntryStub {
    pub title: String,
    pub tags: Vec<String>,
    pub url: String,
    pub created_at: String,
}

impl BlogEntry {
    /// Builds a `BlogEntryStub` that represents this `BlogEntry`.
    fn to_stub(&self) -> BlogEntryStub {
        BlogEntryStub {
            title: self.title.clone(),
            tags: self.tags.clone(),
            url: format!("/blog/{}", self.metadata.slug),
            created_at: format_datetime(self.created_at),
        }
    }
}

#[derive(Serialize)]
pub struct BaseContext {
    pub title: String,
    pub meta_description: String,
}

#[derive(Serialize)]
pub struct IndexContext {
    pub base: BaseContext,
    pub recent_blog_entries: Vec<BlogEntryStub>,
}

impl Site {
    /// Builds the context for the index page.
    pub fn build_index_context(&self) -> IndexContext {
        let recent_blog_entries = self
            .blog_entries
            .iter()
            .take(RECENT_BLOG_ENTRIES_LIMIT)
            .map(BlogEntry::to_stub)
            .collect();

        IndexContext {
            base: BaseContext {
                title: "Sup".to_string(),
                meta_description: "It's The Rotoclone Zone".to_string(),
            },
            recent_blog_entries,
        }
    }
}

#[derive(Serialize)]
pub struct AboutContext {
    base: BaseContext,
}

impl Site {
    /// Builds the context for the about page.
    pub fn build_about_context(&self) -> AboutContext {
        AboutContext {
            base: BaseContext {
                title: "About The Rotoclone Zone".to_string(),
                meta_description: "It's The Rotoclone Zone".to_string(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct BlogIndexContext {
    base: BaseContext,
    entries: Vec<BlogEntryStub>,
    previous_page: Option<usize>,
    next_page: Option<usize>,
}

impl Site {
    /// Builds the context for the blog index page.
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
            base: BaseContext {
                title: "The Rotoclone Zone Blog".to_string(),
                meta_description: "It's The Rotoclone Zone Blog".to_string(),
            },
            entries,
            previous_page,
            next_page,
        }
    }
}

#[derive(Serialize)]
pub struct BlogEntryContext {
    base: BaseContext,
    tags: Vec<String>,
    created_at: String,
    updated_at: Option<String>,
    entry_content: String,
    previous_entry: Option<BlogEntryStub>,
    next_entry: Option<BlogEntryStub>,
}

impl Site {
    /// Builds the context for the blog entry page for the provided blog entry.
    ///
    /// # Errors
    /// Returns any errors encountered while reading the content of the blog entry from the filesystem.
    pub fn build_blog_entry_context(
        &self,
        entry: &BlogEntry,
    ) -> Result<BlogEntryContext, std::io::Error> {
        Ok(BlogEntryContext {
            base: BaseContext {
                title: entry.title.clone(),
                meta_description: entry.title.clone(),
            },
            tags: entry.tags.clone(),
            created_at: format_datetime(entry.created_at),
            updated_at: entry.updated_at.map(format_datetime),
            entry_content: read_to_string(&entry.metadata.html_content_file)?,
            previous_entry: None, //TODO
            next_entry: None,     //TODO
        })
    }
}

#[derive(Serialize)]
pub struct ErrorContext {
    pub base: BaseContext,
    pub header: String,
    pub message: String,
}

/// Converts the provided `DateTime` into a nice human-readable string.
fn format_datetime(datetime: DateTime<Utc>) -> String {
    let month = datetime.format("%B");
    let day = Ordinal(datetime.day()).to_string();
    let year = datetime.format("%Y");

    format!("{} {}, {}", month, day, year)
}
