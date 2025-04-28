# Synology Download Station API Client

[![test & release](https://img.shields.io/github/actions/workflow/status/artemy/syno-download-station/ci.yml?logo=github)](https://github.com/artemy/syno-download-station)
[![Crates.io Version](https://img.shields.io/crates/v/syno-download-station?logo=rust)](https://crates.io/crates/syno-download-station)
[![docs.rs](https://img.shields.io/docsrs/syno-download-station?logo=docs.rs)](https://docs.rs/syno-download-station/latest/syno_download_station/)
[![MIT License](https://img.shields.io/github/license/artemy/syno-download-station)](https://github.com/artemy/syno-download-station)

A Rust client library for interacting with the Synology Download Station API. Manage your downloads programmatically with a strongly-typed interface.

## Features

- Authentication with Synology API
- List download tasks
- Get detailed task information (status, progress, files, peers)
- Create downloads from URLs/magnet links
- Create downloads from torrent files
- Control tasks (pause, resume, complete, delete)
- Clear completed downloads

## Installation

```bash
cargo add syno-download-station
```

## Example Usage

```rust
use anyhow::Result;
use syno_download_station::client::SynoDS;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new client
    let mut synods = SynoDS::new(
        "https://your-synology-nas.local:5001".to_string(),
        "username".to_string(),
        "password".to_string(),
    )?;
    
    // Authenticate
    synods.authorize().await?;
    
    // List all tasks
    let tasks = synods.get_tasks().await?;
    for task in tasks.task {
        println!(
            "Task: {}, Status: {:?}, Progress: {}%, {}",
            task.title,
            task.status,
            task.calculate_progress(),
            task.calculate_speed()
        );
        
        // Get more detailed information if needed
        if let Some(additional) = &task.additional {
            if let Some(detail) = &additional.detail {
                println!("  Created: {}", detail.created_time);
                println!("  Destination: {}", detail.destination);
            }
        }
    }
    
    // Create a new download task from URL
    synods.create_task(
        "https://example.com/large-file.zip",
        "downloads"
    ).await?;
    
    // Other operations
    synods.pause("task-id").await?;
    synods.resume("task-id").await?;
    synods.complete("task-id").await?;
    synods.clear_completed().await?;
    
    Ok(())
}
```

## CLI Example

An example CLI application is included in the examples directory. To run it:

```bash
SYNOLOGY_URL="https://your-synology-nas.local:1234" \
SYNOLOGY_USERNAME="your-username" \
SYNOLOGY_PASSWORD="your-password" \
cargo run --example cli
```

## License

This project is licensed under the MIT License — see the [LICENSE.md](LICENSE.md) file for details
