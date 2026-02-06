use anyhow::Result;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use std::{fs, path::Path, time::Duration};

use crate::{config::StorageConfig, s3_client::create_client, utils::format_size};

pub struct UploadInfo {
    pub file_name: String,
    pub download_url: String,
}

pub async fn upload_file(
    file_path: &str,
    config: &StorageConfig,
    verbose: bool,
    expires_seconds: Option<u64>,
) -> Result<UploadInfo> {
    if verbose {
        println!("📤 Uploading file: {}", file_path);
        println!("  Max size allowed: {}", format_size(config.max_size));
    }

    let path = Path::new(file_path);
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", file_path);
    }

    let metadata = fs::metadata(path)?;
    if metadata.len() > config.max_size {
        anyhow::bail!(
            "File exceeds max size {} (file size: {})",
            format_size(config.max_size),
            format_size(metadata.len())
        );
    }

    if verbose {
        println!("  File size: {}", format_size(metadata.len()));
    }

    let client = create_client(config, verbose).await?;
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();
    let body = ByteStream::from_path(path).await?;

    client
        .put_object()
        .bucket(&config.bucket)
        .key(&file_name)
        .content_type(content_type)
        .body(body)
        .send()
        .await?;

    if verbose {
        println!("  ✅ Upload completed");
    }

    let expires = Duration::from_secs(expires_seconds.unwrap_or(3600));
    let presign_config = PresigningConfig::expires_in(expires)?;
    let presigned_req = client
        .get_object()
        .bucket(&config.bucket)
        .key(&file_name)
        .presigned(presign_config)
        .await?;
    Ok(UploadInfo {
        file_name,
        download_url: presigned_req.uri().to_string(),
    })
}
