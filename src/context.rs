use chrono::{DateTime, Datelike, Utc};
use ordinal::Ordinal;
use serde::Serialize;
use std::{fs::read_to_string, num::NonZeroUsize};

use crate::site::{BlogEntry, Site};

const RECENT_BLOG_ENTRIES_LIMIT: usize = 5;
const PAGE_SIZE: usize = 10;

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
                title: "The Rotoclone Zone".to_string(),
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
        let start_index = (page.get() - 1) * PAGE_SIZE;
        let entries = self
            .blog_entries
            .iter()
            .skip(start_index)
            .take(PAGE_SIZE)
            .map(BlogEntry::to_stub)
            .collect();

        let (previous_page, next_page) =
            calculate_pages(page, start_index, self.blog_entries.len(), PAGE_SIZE);

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
        //TODO this looks up the entry again, refactor this method to take in a slug so the entries list only has to be searched once
        // the list of blog entries is sorted by creation date descending, so the previous entry in the list is the next entry chronologically
        let (next_entry, previous_entry) = stubs_for_surrounding_entries(&self.blog_entries, entry);

        Ok(BlogEntryContext {
            base: BaseContext {
                title: entry.title.clone(),
                meta_description: entry.title.clone(),
            },
            tags: entry.tags.clone(),
            created_at: format_datetime(entry.created_at),
            updated_at: entry.updated_at.map(format_datetime),
            entry_content: read_to_string(&entry.metadata.html_content_file)?,
            previous_entry,
            next_entry,
        })
    }
}

/// Builds `BlogEntryStub`s for the blog entries from the provided list positioned immediately before and after the provided entry, if they exist.
fn stubs_for_surrounding_entries(
    entries: &[BlogEntry],
    entry: &BlogEntry,
) -> (Option<BlogEntryStub>, Option<BlogEntryStub>) {
    let entry_index = entries.iter().position(|x| x == entry);

    entry_index.map_or((None, None), |index| {
        let previous = if index == 0 {
            None
        } else {
            Some(entries[index - 1].to_stub())
        };

        let next = if index == entries.len() - 1 {
            None
        } else {
            Some(entries[index + 1].to_stub())
        };

        (previous, next)
    })
}

#[derive(Serialize)]
pub struct BlogTagsContext {
    base: BaseContext,
    tags: Vec<String>,
}

impl Site {
    /// Builds the context for the page of all the blog tags.
    pub fn build_blog_tags_context(&self) -> BlogTagsContext {
        let mut tags = self
            .blog_entries
            .iter()
            .flat_map(|entry| entry.tags.clone())
            .collect::<Vec<String>>();
        tags.sort_unstable();
        tags.dedup();

        BlogTagsContext {
            base: BaseContext {
                title: "The Rotoclone Zone Blog - All Tags".to_string(),
                meta_description: "All the tags".to_string(),
            },
            tags,
        }
    }
}

#[derive(Serialize)]
pub struct BlogTagContext {
    base: BaseContext,
    tag: String,
    entries: Vec<BlogEntryStub>,
    previous_page: Option<usize>,
    next_page: Option<usize>,
}

impl Site {
    /// Builds the context for a blog tag page.
    /// Returns `None` if there are no entries with the provided tag.
    pub fn build_blog_tag_context(
        &self,
        tag: String,
        page: NonZeroUsize,
    ) -> Option<BlogTagContext> {
        let start_index = (page.get() - 1) * PAGE_SIZE;
        let all_matching_entries = self
            .blog_entries
            .iter()
            .filter(|entry| entry.tags.contains(&tag))
            .collect::<Vec<&BlogEntry>>();

        if all_matching_entries.is_empty() {
            return None;
        }

        let total_matching_entries = all_matching_entries.len();
        let entries = all_matching_entries
            .into_iter()
            .skip(start_index)
            .take(PAGE_SIZE)
            .map(BlogEntry::to_stub)
            .collect::<Vec<BlogEntryStub>>();

        let (previous_page, next_page) =
            calculate_pages(page, start_index, total_matching_entries, PAGE_SIZE);

        Some(BlogTagContext {
            base: BaseContext {
                title: format!("The Rotoclone Zone Blog - Posts Tagged {}", tag),
                meta_description: format!("All the posts tagged {}", tag),
            },
            tag,
            entries,
            previous_page,
            next_page,
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

fn calculate_pages(
    current_page: NonZeroUsize,
    start_index: usize,
    total_size: usize,
    page_size: usize,
) -> (Option<usize>, Option<usize>) {
    let previous_page = match current_page.get() {
        1 => None,
        _ => Some(current_page.get() - 1),
    };

    let next_page = if total_size > (start_index + page_size) {
        Some(current_page.get() + 1)
    } else {
        None
    };

    (previous_page, next_page)
}
