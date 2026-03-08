pub mod airtable;
pub mod chrome;
pub mod files;
pub mod github;
pub mod gitlab;
pub mod gmail;
pub mod jira;
pub mod linear;
pub mod notion;
pub mod obsidian;
pub mod slack;
pub mod splitter;
pub mod terminal;
pub mod todoist;
pub mod trello;
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

pub use airtable::AirtableHarvester;
pub use chrome::ChromeHarvester;
pub use files::{FileHarvester, SKIP_DIRS, TEXT_EXTENSIONS, harvest_file};
pub use github::GithubHarvester;
pub use gitlab::GitlabHarvester;
pub use gmail::{GmailConfig, GmailHarvester};
pub use jira::JiraHarvester;
pub use linear::LinearHarvester;
pub use notion::NotionHarvester;
pub use obsidian::ObsidianHarvester;
pub use slack::SlackHarvester;
pub use terminal::TerminalHarvester;
pub use todoist::TodoistHarvester;
pub use trello::TrelloHarvester;
pub use watcher::{start as start_watcher, WatchEvent};

#[cfg(target_os = "macos")]
pub use calendar::CalendarHarvester;
#[cfg(target_os = "macos")]
pub use imessage::IMessageHarvester;
#[cfg(target_os = "macos")]
pub use notes::NotesHarvester;
#[cfg(target_os = "macos")]
pub use safari::SafariHarvester;
