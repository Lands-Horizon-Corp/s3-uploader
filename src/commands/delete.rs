use crate::{config::StorageConfig, s3_client::create_client};
use anyhow::Result;

pub async fn delete_file(file_name: &str, config: &StorageConfig, verbose: bool) -> Result<()> {
    let client = create_client(config, verbose).await?;
    if verbose {
        println!("🗑️ Deleting file: {}", file_name);
    }

    client
        .delete_object()
        .bucket(&config.bucket)
        .key(file_name)
        .send()
        .await?;

    if verbose {
        println!("✅ Deleted file: {}", file_name);
    }

    Ok(())
}
