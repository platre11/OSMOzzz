pub mod chrome;
pub mod files;
pub mod gmail;
pub mod splitter;
pub mod watcher;
mod checksum;

pub use chrome::ChromeHarvester;
pub use files::FileHarvester;
pub use gmail::{GmailConfig, GmailHarvester};
pub use watcher::{start as start_watcher, WatchEvent};
