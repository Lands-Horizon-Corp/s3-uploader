use std::{env, path::Path, time::Duration};
use anyhow::{Result, bail};
use aws_sdk_s3::{Client, config::{Credentials, Region}};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_config::SdkConfig;
use dotenvy::dotenv;
use mime_guess::MimeGuess;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let file_path = env::args().nth(1)
        .expect("Usage: cargo run -- <file_path>");

    let info = upload_file(&file_path).await?;
    
    println!("✅ Upload successful!");
    println!("File: {}", info.file_name);
    println!("Bucket: {}", info.bucket);
    println!("Region: {}", info.region);
    println!("Size: {} bytes", info.size);
    println!("Download URL (expires in 1 hour): {}", info.download_url);

    Ok(())
}

struct UploadInfo {
    file_name: String,
    bucket: String,
    region: String,
    size: u64,
    download_url: String,
}

async fn upload_file(file_path: &str) -> Result<UploadInfo> {
    let path = Path::new(file_path);
    if !path.exists() {
        bail!("File does not exist");
    }

    let metadata = std::fs::metadata(path)?;
    let max_size: u64 = env::var("STORAGE_MAX_SIZE")?.parse()?;
    if metadata.len() > max_size {
        bail!("File exceeds max size limit");
    }

    let bucket = env::var("STORAGE_BUCKET")?;
    let region_str = env::var("STORAGE_REGION")?;
    let access_key = env::var("STORAGE_ACCESS_KEY")?;
    let secret_key = env::var("STORAGE_SECRET_KEY")?;
    let endpoint = env::var("STORAGE_URL").ok();

    let credentials = Credentials::new(
        access_key.clone(),
        secret_key.clone(),
        None,
        None,
        "custom",
    );

    // Create a Region object, which satisfies the 'static lifetime
    let region = Region::new(region_str.clone());

    let mut config_loader = aws_config::from_env()
        .region(region)
        .credentials_provider(credentials);

    if let Some(endpoint_url) = endpoint.clone() {
        config_loader = config_loader.endpoint_url(endpoint_url);
    }

    let config: SdkConfig = config_loader.load().await;
    let client = Client::new(&config);

    let file_name = path.file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mime: MimeGuess = mime_guess::from_path(path);
    let content_type = mime.first_or_octet_stream().to_string();
    let body = ByteStream::from_path(path).await?;

    // Upload
    client
        .put_object()
        .bucket(&bucket)
        .key(&file_name)
        .content_type(content_type)
        .body(body)
        .send()
        .await?;

    // Presigned URL
    let presign_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;
    let presigned_req = client
        .get_object()
        .bucket(&bucket)
        .key(&file_name)
        .presigned(presign_config)
        .await?;

    Ok(UploadInfo {
        file_name,
        bucket,
        region: region_str,
        size: metadata.len(),
        download_url: presigned_req.uri().to_string(),
    })
}
