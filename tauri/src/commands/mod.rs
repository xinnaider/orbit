pub mod diff;
pub mod files;
pub mod plugins;
pub mod stats;
pub mod tasks;

pub use diff::{get_diff, get_file_versions};
pub use files::{get_subagent_journal, list_project_files};
pub use plugins::get_slash_commands;
pub use stats::{get_changelog, get_claude_usage_stats};
pub use tasks::get_tasks;
