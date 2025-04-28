use anyhow::Result;
use std::env;
use syno_download_station::client::SynoDS;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut synods = {
        let url = env::var("SYNOLOGY_URL")?;
        let username = env::var("SYNOLOGY_USERNAME")?;
        let password = env::var("SYNOLOGY_PASSWORD")?;
        SynoDS::builder()
            .url(url)
            .username(username)
            .password(password)
            .build()?
    };

    synods.authorize().await?;

    let tasks = synods.get_tasks().await?;
    for task in tasks.task {
        println!(
            "task: {}, title: {}, status: {:?}",
            task.id, task.title, task.status
        );
    }

    let operation = synods.clear_completed().await?;
    println!("operation result: {:?}", operation);

    Ok(())
}
