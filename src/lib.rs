//!# Synology Download Station API Client
//!
//! A Rust client library for interacting with the Synology Download Station API. Manage your downloads programmatically with a strongly-typed interface.
//!
//! ## Features
//!
//! - Authentication with Synology API
//! - List and filter download tasks
//! - Get detailed task information (status, progress, files, peers)
//! - Create downloads from URLs/magnet links
//! - Create downloads from torrent files
//! - Control tasks (pause, resume, complete)
//! - Clear completed downloads
//! - Human-readable file sizes, progress calculation and ETA
//!
//! ## Usage example
//!
//!```rust
//! use anyhow::Result;
//! use std::env;
//! use syno_download_station::client::SynoDS;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut synods = {
//!         let host = env::var("SYNOLOGY_HOST")?;
//!         let username = env::var("SYNOLOGY_USERNAME")?;
//!         let password = env::var("SYNOLOGY_PASSWORD")?;
//!         SynoDS::builder()
//!             .host(host)
//!             .username(username)
//!             .password(password)
//!             .build()?
//!     };
//!
//!     synods.authorize().await?;
//!
//!     let tasks = synods.get_tasks().await?;
//!     for task in tasks.task {
//!         println!(
//!             "task: {}, title: {}, status: {:?}",
//!             task.id, task.title, task.status
//!         );
//!     }
//!
//!     let operation = synods.clear_completed().await?;
//!     println!("operation result: {:?}", operation);
//!
//!     Ok(())
//! }
//! ```


pub mod client;
pub mod entities;
pub mod utils;
