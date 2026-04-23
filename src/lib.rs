pub mod config;
pub mod market;
pub mod indicators;
pub mod snapshot;
pub mod llm;
pub mod logic;
pub mod watcher;

pub type AnyError = Box<dyn std::error::Error + Send + Sync>;
