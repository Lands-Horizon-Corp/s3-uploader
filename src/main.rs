use anyhow::{bail, Result};
use aws_config::SdkConfig;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{
    config::{Credentials, Region},
    Client,
};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use std::{env, fs, path::Path, time::Duration};
use tokio::io::AsyncWriteExt;

#[derive(Parser)]
#[command(name = "s3-storage")]
#[command(about = "Upload and download files from S3-compatible storage")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a file to storage
    Upload {
        /// Path to the file to upload
        file_path: String,

        /// Storage bucket name
        #[arg(long)]
        bucket: Option<String>,

        /// Storage region
        #[arg(long)]
        region: Option<String>,

        /// Storage access key
        #[arg(long)]
        access_key: Option<String>,

        /// Storage secret key
        #[arg(long)]
        secret_key: Option<String>,

        /// Storage endpoint URL
        #[arg(long)]
        endpoint: Option<String>,

        /// Maximum file size in bytes (default: 100MB)
        #[arg(long)]
        max_size: Option<u64>,
    },
    /// Download a file from storage
    Download {
        /// File name in storage to download
        file_name: String,

        /// Output file path (defaults to current directory with same filename)
        #[arg(long)]
        output: Option<String>,

        /// Storage bucket name
        #[arg(long)]
        bucket: Option<String>,

        /// Storage region
        #[arg(long)]
        region: Option<String>,

        /// Storage access key
        #[arg(long)]
        access_key: Option<String>,

        /// Storage secret key
        #[arg(long)]
        secret_key: Option<String>,

        /// Storage endpoint URL
        #[arg(long)]
        endpoint: Option<String>,

        /// Generate presigned URL instead of downloading
        #[arg(long)]
        presign: bool,

        /// Expiry time for presigned URL in seconds (default: 3600)
        #[arg(long, default_value_t = 3600)]
        expires: u64,
    },
}

#[derive(Debug)]
struct StorageConfig {
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
    endpoint: Option<String>,
}

struct UploadInfo {
    file_name: String,
    bucket: String,
    region: String,
    size: u64,
    download_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Upload {
            file_path,
            bucket,
            region,
            access_key,
            secret_key,
            endpoint,
            max_size,
        } => {
            let config = load_config(bucket, region, access_key, secret_key, endpoint)?;
            let info = upload_file(&file_path, &config, max_size).await?;

            println!("✅ Upload successful!");
            println!("File: {}", info.file_name);
            println!("Bucket: {}", info.bucket);
            println!("Region: {}", info.region);
            println!("Size: {} bytes", info.size);
            println!("Download URL (expires in 1 hour): {}", info.download_url);
        }
        Commands::Download {
            file_name,
            output,
            bucket,
            region,
            access_key,
            secret_key,
            endpoint,
            presign,
            expires,
        } => {
            let config = load_config(bucket, region, access_key, secret_key, endpoint)?;

            if presign {
                let url = generate_presigned_url(&file_name, &config, expires).await?;
                println!("Presigned URL (expires in {} seconds):", expires);
                println!("{}", url);
            } else {
                download_file(&file_name, output.as_deref(), &config).await?;
                println!("✅ Download successful!");
            }
        }
    }

    Ok(())
}

fn load_config(
    bucket: Option<String>,
    region: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
    endpoint: Option<String>,
) -> Result<StorageConfig> {
    // Get values from params first, then env, then defaults
    let bucket = bucket
        .or_else(|| env::var("STORAGE_BUCKET").ok())
        .unwrap_or_else(|| "default-bucket".to_string());

    let region = region
        .or_else(|| env::var("STORAGE_REGION").ok())
        .unwrap_or_else(|| "us-east-1".to_string());

    let access_key = access_key
        .or_else(|| env::var("STORAGE_ACCESS_KEY").ok())
        .unwrap_or_else(|| "".to_string());

    let secret_key = secret_key
        .or_else(|| env::var("STORAGE_SECRET_KEY").ok())
        .unwrap_or_else(|| "".to_string());

    let endpoint = endpoint.or_else(|| env::var("STORAGE_URL").ok());

    // Validate required fields
    if access_key.is_empty() || secret_key.is_empty() {
        bail!("Access key and secret key must be provided either via parameters or environment variables");
    }

    Ok(StorageConfig {
        bucket,
        region,
        access_key,
        secret_key,
        endpoint,
    })
}

async fn create_client(config: &StorageConfig) -> Result<Client> {
    let credentials = Credentials::new(
        config.access_key.clone(),
        config.secret_key.clone(),
        None,
        None,
        "custom",
    );

    let region = Region::new(config.region.clone());

    // Use the builder pattern with behavior version
    let mut config_builder = aws_config::ConfigLoader::default()
        .region(region)
        .credentials_provider(credentials);

    if let Some(endpoint_url) = &config.endpoint {
        config_builder = config_builder.endpoint_url(endpoint_url);
    }

    let sdk_config: SdkConfig = config_builder.load().await;

    // Create client with explicit behavior version
    let client_config = aws_sdk_s3::config::Builder::from(&sdk_config)
        .behavior_version_latest()
        .build();

    Ok(Client::from_conf(client_config))
}

async fn upload_file(
    file_path: &str,
    config: &StorageConfig,
    max_size: Option<u64>,
) -> Result<UploadInfo> {
    let path = Path::new(file_path);

    if !path.exists() {
        bail!("File does not exist: {}", file_path);
    }

    let metadata = fs::metadata(path)?;

    // Check file size
    let max_size = max_size
        .or_else(|| {
            env::var("STORAGE_MAX_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(100 * 1024 * 1024); // Default 100MB

    if metadata.len() > max_size {
        bail!("File exceeds max size limit of {} bytes", max_size);
    }

    let client = create_client(config).await?;
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
        .to_string_lossy()
        .to_string();

    // Determine content type
    let mime = mime_guess::from_path(path);
    let content_type = mime.first_or_octet_stream().to_string();

    // Read file and upload
    let body = ByteStream::from_path(path).await?;

    client
        .put_object()
        .bucket(&config.bucket)
        .key(&file_name)
        .content_type(content_type)
        .body(body)
        .send()
        .await?;

    // Generate presigned URL
    let presign_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;
    let presigned_req = client
        .get_object()
        .bucket(&config.bucket)
        .key(&file_name)
        .presigned(presign_config)
        .await?;

    Ok(UploadInfo {
        file_name,
        bucket: config.bucket.clone(),
        region: config.region.clone(),
        size: metadata.len(),
        download_url: presigned_req.uri().to_string(),
    })
}

async fn download_file(
    file_name: &str,
    output_path: Option<&str>,
    config: &StorageConfig,
) -> Result<()> {
    let client = create_client(config).await?;

    // Determine output path
    let output_path = match output_path {
        Some(path) => Path::new(path).to_path_buf(),
        None => {
            let current_dir = env::current_dir()?;
            current_dir.join(file_name)
        }
    };

    // Check if output directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Download the file
    let mut object = client
        .get_object()
        .bucket(&config.bucket)
        .key(file_name)
        .send()
        .await?;

    // Create output file
    let mut file = tokio::fs::File::create(&output_path).await?;

    // Stream the data to the file
    while let Some(chunk) = object.body.try_next().await? {
        file.write_all(&chunk).await?;
    }

    file.flush().await?;

    println!("Downloaded to: {}", output_path.display());
    Ok(())
}

async fn generate_presigned_url(
    file_name: &str,
    config: &StorageConfig,
    expires_seconds: u64,
) -> Result<String> {
    let client = create_client(config).await?;

    let presign_config = PresigningConfig::expires_in(Duration::from_secs(expires_seconds))?;

    let presigned_req = client
        .get_object()
        .bucket(&config.bucket)
        .key(file_name)
        .presigned(presign_config)
        .await?;

    Ok(presigned_req.uri().to_string())
}
