pub mod protocol;
pub mod memory;
pub mod router;
pub mod config;
pub mod cli;
pub mod cognition;
pub mod taskmaster;
pub mod sequential;
pub mod logger;
pub mod vector;
pub mod mcp;
pub mod metrics;
pub mod backup;
pub mod cognitive;
pub mod autonomy;
pub mod db;
pub mod cognitive_db;

// Re-export the exact functions users requested
pub use taskmaster::{add_task, update_task, next_task, link_tasks};
pub use cognitive_db::{store_step, recent_steps};
pub use vector::{insert_text, search};
