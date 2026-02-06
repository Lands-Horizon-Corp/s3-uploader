use anyhow::Result;
use std::{env, fs, path::Path};
use tokio::io::AsyncWriteExt;

use crate::{config::StorageConfig, s3_client::create_client, utils::format_size};

pub async fn download_file(
    file_name: &str,
    output_path: Option<&str>,
    presign: bool,
    expires_seconds: u64,
    config: &StorageConfig,
    verbose: bool,
) -> Result<()> {
    let client = create_client(config, verbose).await?;

    if presign {
        // Generate presigned URL
        if verbose {
            println!("🔗 Generating presigned URL for {}", file_name);
        }
        let presign_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(
            std::time::Duration::from_secs(expires_seconds),
        )?;
        let presigned_req = client
            .get_object()
            .bucket(&config.bucket)
            .key(file_name)
            .presigned(presign_config)
            .await?;
        println!("{}", presigned_req.uri());
        return Ok(());
    }

    // Determine output path
    let output_path = match output_path {
        Some(p) => Path::new(p).to_path_buf(),
        None => env::current_dir()?.join(file_name),
    };

    if verbose {
        println!("📥 Downloading {} -> {}", file_name, output_path.display());
    }

    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
            if verbose {
                println!("  Created directory {}", parent.display());
            }
        }
    }

    let mut object = client
        .get_object()
        .bucket(&config.bucket)
        .key(file_name)
        .send()
        .await?;
    let content_length = object.content_length().unwrap_or(0);

    let mut file = tokio::fs::File::create(&output_path).await?;
    let mut downloaded = 0;

    while let Some(chunk) = object.body.try_next().await? {
        downloaded += chunk.len();
        file.write_all(&chunk).await?;

        if verbose && content_length > 0 {
            let percent = (downloaded as f64 / content_length as f64 * 100.0) as u32;
            print!(
                "\r  Progress: {}% ({}/{})",
                percent,
                format_size(downloaded as u64),
                format_size(content_length as u64)
            );
        }
    }

    if verbose && content_length > 0 {
        println!("\n✅ Download completed");
    }

    file.flush().await?;
    if verbose {
        println!("  Saved to: {}", output_path.display());
    }

    Ok(())
}
