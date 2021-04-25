use std::{
    error::Error,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use hotwatch::{Event, Hotwatch};

use crate::site::Site;

/// Site that updates itself when changes to its source directory are detected.
pub struct UpdatingSite {
    /// The `Hotwatch` instance that handles updating the site.
    _hotwatch: Hotwatch,
    /// The site.
    pub site: Arc<RwLock<Site>>,
}

impl UpdatingSite {
    /// Builds an updating site from the provided source directory, and puts rendered HTML in the provided HTML directory.
    ///
    /// # Errors
    /// Returns any errors that occur while reading from the file system or parsing file contents.
    pub fn from_dir(
        source_dir: PathBuf,
        html_dir: PathBuf,
    ) -> Result<UpdatingSite, Box<dyn Error>> {
        let site = Site::from_dir(&source_dir, &html_dir)?;

        let shared_site = Arc::new(RwLock::new(site));
        let hotwatch_site = Arc::clone(&shared_site);

        let mut hotwatch = Hotwatch::new()?;
        hotwatch.watch(source_dir.clone(), move |event: Event| {
            match event {
                Event::NoticeRemove(_) | Event::NoticeWrite(_) | Event::Error(_, _) => return,
                _ => (),
            };

            println!("Changes detected, rebuilding site... ({:?})", event);
            match Site::from_dir(&source_dir, &html_dir) {
                Ok(site) => {
                    println!("Site rebuilt successfully.");
                    *hotwatch_site.write().unwrap() = site;
                }
                Err(e) => println!("Error rebuilding site: {:?}", e),
            };
        })?;

        Ok(UpdatingSite {
            _hotwatch: hotwatch,
            site: shared_site,
        })
    }
}
