//!# Synology Download Station API Client
//!
//! [![test & release](https://img.shields.io/github/actions/workflow/status/artemy/syno-download-station/ci.yml?logo=github)](https://github.com/artemy/syno-download-station)
//! [![Crates.io Version](https://img.shields.io/crates/v/syno-download-station?logo=rust) ](https://crates.io/crates/syno-download-station)
//! [![docs.rs](https://img.shields.io/docsrs/syno-download-station?logo=docs.rs)](https://docs.rs/syno-download-station/latest/syno_download_station/)
//! [![MIT License](https://img.shields.io/github/license/artemy/syno-download-station)](https://github.com/artemy/syno-download-station)
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
//! ```rust,no_run
//! use anyhow::Result;
//! use std::env;
//! use syno_download_station::client::SynoDS;
//!
//! #[tokio::main(flavor = "current_thread")]
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
