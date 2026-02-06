use std::{env, path::Path};
use anyhow::{Result, bail};
use aws_sdk_s3::{Client, config::Credentials};
use aws_sdk_s3::types::ByteStream;
use aws_config::SdkConfig;
use dotenvy::dotenv;
use mime_guess::MimeGuess;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let file_path = env::args().nth(1)
        .expect("Usage: cargo run -- <file_path>");
    upload_file(&file_path).await?;
    println!("✅ Upload successful");
    Ok(())
}

async fn upload_file(file_path: &str) -> Result<()> {
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
    let region = env::var("STORAGE_REGION")?;
    let access_key = env::var("STORAGE_ACCESS_KEY")?;
    let secret_key = env::var("STORAGE_SECRET_KEY")?;
    let endpoint = env::var("STORAGE_URL").ok();
    let credentials = Credentials::new(access_key,secret_key,None,None,"custom",);
    let mut config_loader = aws_config::from_env()
        .region(region)
        .credentials_provider(credentials);

    if let Some(endpoint_url) = endpoint {
        config_loader = config_loader.endpoint_url(endpoint_url);
    }
    let config: SdkConfig = config_loader.load().await;
    let client = Client::new(&config);

    let key = path.file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mime: MimeGuess = mime_guess::from_path(path);
    let content_type = mime.first_or_octet_stream().to_string();
    let body = ByteStream::from_path(path).await?;
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type(content_type)
        .body(body)
        .send()
        .await?;

    Ok(())
}
