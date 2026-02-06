use crate::{config::StorageConfig, s3_client::create_client};
use anyhow::Result;

pub async fn list_files(
    prefix: Option<&str>,
    limit: i32,
    config: &StorageConfig,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("📄 Listing files in bucket {}", config.bucket);
        if let Some(p) = prefix {
            println!("  Prefix: {}", p);
        }
        println!("  Limit: {}", limit);
    }

    let client = create_client(config, verbose).await?;
    let mut request = client
        .list_objects_v2()
        .bucket(&config.bucket)
        .max_keys(limit);

    if let Some(prefix) = prefix {
        request = request.prefix(prefix);
    }

    let response = request.send().await?;
    let contents = response.contents();

    if contents.is_empty() {
        println!("No files found");
    } else {
        println!("Found {} file(s):", contents.len());
        for (i, object) in contents.iter().enumerate() {
            let size = object.size().unwrap_or(0);
            let last_modified = object
                .last_modified()
                .map(|dt| dt.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            println!(
                "{}. {} ({} bytes, modified: {})",
                i + 1,
                object.key().unwrap_or("unknown"),
                size,
                last_modified
            );
        }
    }

    Ok(())
}
