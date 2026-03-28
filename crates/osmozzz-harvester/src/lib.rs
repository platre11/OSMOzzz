pub mod arc;
pub mod chrome;
pub mod contacts;
pub mod files;
pub mod gmail;
pub mod splitter;
pub mod terminal;
pub mod watcher;
mod checksum;

// Sources macOS uniquement
#[cfg(target_os = "macos")]
pub mod calendar;
#[cfg(target_os = "macos")]
pub mod imessage;
#[cfg(target_os = "macos")]
pub mod notes;
#[cfg(target_os = "macos")]
pub mod safari;

pub use arc::ArcHarvester;
pub use chrome::ChromeHarvester;
pub use contacts::ContactsHarvester;
pub use files::{FileHarvester, SKIP_DIRS, TEXT_EXTENSIONS, harvest_file};
pub use gmail::{GmailConfig, GmailHarvester};
pub use terminal::TerminalHarvester;
pub use watcher::{start as start_watcher, WatchEvent};

#[cfg(target_os = "macos")]
pub use calendar::CalendarHarvester;
#[cfg(target_os = "macos")]
pub use imessage::IMessageHarvester;
#[cfg(target_os = "macos")]
pub use notes::NotesHarvester;
#[cfg(target_os = "macos")]
pub use safari::SafariHarvester;
