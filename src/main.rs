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
#[command(version = "1.0")]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Storage bucket name (overrides env STORAGE_BUCKET)
    #[arg(long, global = true)]
    bucket: Option<String>,

    /// Storage region (overrides env STORAGE_REGION)
    #[arg(long, global = true)]
    region: Option<String>,

    /// Storage access key (overrides env STORAGE_ACCESS_KEY)
    #[arg(long, global = true)]
    access_key: Option<String>,

    /// Storage secret key (overrides env STORAGE_SECRET_KEY)
    #[arg(long, global = true)]
    secret_key: Option<String>,

    /// Storage endpoint URL (overrides env STORAGE_URL)
    #[arg(long, global = true)]
    endpoint: Option<String>,

    /// Maximum file size in bytes (overrides env STORAGE_MAX_SIZE)
    #[arg(long, global = true, default_value_t = 100 * 1024 * 1024)]
    max_size: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a file to storage
    Upload {
        /// Path to the file to upload
        file_path: String,
    },
    /// Download a file from storage
    Download {
        /// File name in storage to download
        file_name: String,

        /// Output file path (defaults to current directory with same filename)
        #[arg(long)]
        output: Option<String>,

        /// Generate presigned URL instead of downloading
        #[arg(long)]
        presign: bool,

        /// Expiry time for presigned URL in seconds
        #[arg(long, default_value_t = 3600)]
        expires: u64,
    },
    /// List files in storage bucket
    List {
        /// List files with this prefix
        #[arg(long)]
        prefix: Option<String>,

        /// Maximum number of files to list
        #[arg(long, default_value_t = 100)]
        limit: i32,
    },
    /// Delete a file from storage
    Delete {
        /// File name in storage to delete
        file_name: String,
    },
}

#[derive(Debug, Clone)]
struct StorageConfig {
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
    endpoint: Option<String>,
    max_size: u64,
}

struct UploadInfo {
    file_name: String,
    bucket: String,
    region: String,
    size: u64,
    download_url: String,
}

struct AppContext {
    config: StorageConfig,
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let cli = Cli::parse();

    // Initialize context
    let ctx = AppContext {
        config: load_config(&cli)?,
        verbose: cli.verbose,
    };

    match cli.command {
        Commands::Upload { file_path } => {
            let info = upload_file(&file_path, &ctx).await?;

            if ctx.verbose {
                println!("✅ Upload successful!");
                println!("File: {}", info.file_name);
                println!("Bucket: {}", info.bucket);
                println!("Region: {}", info.region);
                println!("Size: {} bytes", info.size);
                println!("Download URL (expires in 1 hour): {}", info.download_url);
            } else {
                println!("Uploaded: {} ({})", info.file_name, format_size(info.size));
            }
        }
        Commands::Download {
            file_name,
            output,
            presign,
            expires,
        } => {
            if presign {
                let url = generate_presigned_url(&file_name, expires, &ctx).await?;
                if ctx.verbose {
                    println!("Presigned URL (expires in {} seconds):", expires);
                    println!("{}", url);
                } else {
                    println!("{}", url);
                }
            } else {
                download_file(&file_name, output.as_deref(), &ctx).await?;
                if ctx.verbose {
                    println!("✅ Download successful!");
                } else {
                    println!("Downloaded: {}", file_name);
                }
            }
        }
        Commands::List { prefix, limit } => {
            list_files(prefix.as_deref(), limit, &ctx).await?;
        }
        Commands::Delete { file_name } => {
            delete_file(&file_name, &ctx).await?;
            if ctx.verbose {
                println!("✅ Deleted file: {}", file_name);
            } else {
                println!("Deleted: {}", file_name);
            }
        }
    }

    Ok(())
}

fn load_config(cli: &Cli) -> Result<StorageConfig> {
    // Helper function to get value from CLI, then env, then default
    fn get_value(cli_value: &Option<String>, env_var: &str, default: &str) -> String {
        cli_value
            .clone()
            .or_else(|| env::var(env_var).ok())
            .unwrap_or_else(|| default.to_string())
    }

    let bucket = get_value(&cli.bucket, "STORAGE_BUCKET", "default-bucket");
    let region = get_value(&cli.region, "STORAGE_REGION", "us-east-1");
    let access_key = get_value(&cli.access_key, "STORAGE_ACCESS_KEY", "");
    let secret_key = get_value(&cli.secret_key, "STORAGE_SECRET_KEY", "");

    let endpoint = cli
        .endpoint
        .clone()
        .or_else(|| env::var("STORAGE_URL").ok());

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
        max_size: cli.max_size,
    })
}

async fn create_client(config: &StorageConfig, verbose: bool) -> Result<Client> {
    if verbose {
        println!("🔧 Creating S3 client...");
        println!("  Region: {}", config.region);
        if let Some(endpoint) = &config.endpoint {
            println!("  Endpoint: {}", endpoint);
        }
        println!("  Bucket: {}", config.bucket);
    }

    let credentials = Credentials::new(
        config.access_key.clone(),
        config.secret_key.clone(),
        None,
        None,
        "custom",
    );

    let region = Region::new(config.region.clone());

    let mut config_builder = aws_config::ConfigLoader::default()
        .region(region)
        .credentials_provider(credentials);

    if let Some(endpoint_url) = &config.endpoint {
        config_builder = config_builder.endpoint_url(endpoint_url);
    }

    let sdk_config: SdkConfig = config_builder.load().await;

    let client_config = aws_sdk_s3::config::Builder::from(&sdk_config)
        .behavior_version_latest()
        .build();

    Ok(Client::from_conf(client_config))
}

async fn upload_file(file_path: &str, ctx: &AppContext) -> Result<UploadInfo> {
    if ctx.verbose {
        println!("📤 Uploading file: {}", file_path);
        println!("  Max size allowed: {}", format_size(ctx.config.max_size));
    }

    let path = Path::new(file_path);

    if !path.exists() {
        bail!("File does not exist: {}", file_path);
    }

    let metadata = fs::metadata(path)?;

    // Check file size
    if metadata.len() > ctx.config.max_size {
        bail!(
            "File exceeds max size limit of {} (file size: {})",
            format_size(ctx.config.max_size),
            format_size(metadata.len())
        );
    }

    if ctx.verbose {
        println!("  File size: {}", format_size(metadata.len()));
    }

    let client = create_client(&ctx.config, ctx.verbose).await?;
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
        .to_string_lossy()
        .to_string();

    // Determine content type
    let mime = mime_guess::from_path(path);
    let content_type = mime.first_or_octet_stream().to_string();

    if ctx.verbose {
        println!("  Content type: {}", content_type);
        println!("  Uploading to bucket: {}", ctx.config.bucket);
    }

    // Read file and upload
    let body = ByteStream::from_path(path).await?;

    client
        .put_object()
        .bucket(&ctx.config.bucket)
        .key(&file_name)
        .content_type(content_type)
        .body(body)
        .send()
        .await?;

    if ctx.verbose {
        println!("  ✅ Upload completed");
    }

    // Generate presigned URL
    let presign_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;
    let presigned_req = client
        .get_object()
        .bucket(&ctx.config.bucket)
        .key(&file_name)
        .presigned(presign_config)
        .await?;

    Ok(UploadInfo {
        file_name,
        bucket: ctx.config.bucket.clone(),
        region: ctx.config.region.clone(),
        size: metadata.len(),
        download_url: presigned_req.uri().to_string(),
    })
}

async fn download_file(file_name: &str, output_path: Option<&str>, ctx: &AppContext) -> Result<()> {
    if ctx.verbose {
        println!("📥 Downloading file: {}", file_name);
    }

    let client = create_client(&ctx.config, ctx.verbose).await?;

    // Determine output path
    let output_path = match output_path {
        Some(path) => Path::new(path).to_path_buf(),
        None => {
            let current_dir = env::current_dir()?;
            current_dir.join(file_name)
        }
    };

    if ctx.verbose {
        println!("  Output path: {}", output_path.display());
    }

    // Check if output directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            if ctx.verbose {
                println!("  Creating directory: {}", parent.display());
            }
            fs::create_dir_all(parent)?;
        }
    }

    // Download the file
    let mut object = client
        .get_object()
        .bucket(&ctx.config.bucket)
        .key(file_name)
        .send()
        .await?;

    // Get file size from headers if available
    let content_length = object.content_length().unwrap_or(0);
    if ctx.verbose {
        println!("  File size: {} bytes", content_length);
    }

    // Create output file
    let mut file = tokio::fs::File::create(&output_path).await?;

    // Stream the data to the file
    let mut downloaded = 0;
    while let Some(chunk) = object.body.try_next().await? {
        downloaded += chunk.len();
        file.write_all(&chunk).await?;

        if ctx.verbose && content_length > 0 {
            let percent = (downloaded as f64 / content_length as f64 * 100.0) as u32;
            print!(
                "\r  Progress: {}% ({}/{})",
                percent,
                format_size(downloaded as u64),
                format_size(content_length as u64)
            );
        }
    }

    if ctx.verbose && content_length > 0 {
        println!("\n  ✅ Download completed");
    }

    file.flush().await?;

    if ctx.verbose {
        println!("  Saved to: {}", output_path.display());
    }
    Ok(())
}

async fn generate_presigned_url(
    file_name: &str,
    expires_seconds: u64,
    ctx: &AppContext,
) -> Result<String> {
    if ctx.verbose {
        println!("🔗 Generating presigned URL for: {}", file_name);
        println!("  Expires in: {} seconds", expires_seconds);
    }

    let client = create_client(&ctx.config, ctx.verbose).await?;

    let presign_config = PresigningConfig::expires_in(Duration::from_secs(expires_seconds))?;

    let presigned_req = client
        .get_object()
        .bucket(&ctx.config.bucket)
        .key(file_name)
        .presigned(presign_config)
        .await?;

    Ok(presigned_req.uri().to_string())
}

async fn list_files(prefix: Option<&str>, limit: i32, ctx: &AppContext) -> Result<()> {
    if ctx.verbose {
        println!("📄 Listing files in bucket: {}", ctx.config.bucket);
        if let Some(p) = prefix {
            println!("  Prefix: {}", p);
        }
        println!("  Limit: {}", limit);
    }

    let client = create_client(&ctx.config, ctx.verbose).await?;

    let mut request = client
        .list_objects_v2()
        .bucket(&ctx.config.bucket)
        .max_keys(limit);

    if let Some(prefix) = prefix {
        request = request.prefix(prefix);
    }

    let response = request.send().await?;

    // Get the contents - it returns &[Object] directly
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
async fn delete_file(file_name: &str, ctx: &AppContext) -> Result<()> {
    if ctx.verbose {
        println!("🗑️  Deleting file: {}", file_name);
    }

    let client = create_client(&ctx.config, ctx.verbose).await?;

    client
        .delete_object()
        .bucket(&ctx.config.bucket)
        .key(file_name)
        .send()
        .await?;

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let base = 1024_f64;
    let exponent = (bytes as f64).log(base).floor() as i32;
    let exponent = exponent.min(UNITS.len() as i32 - 1);

    let value = bytes as f64 / base.powi(exponent);

    format!("{:.2} {}", value, UNITS[exponent as usize])
}

// Sample usage examples:
//
// Basic upload (with verbose):
// cargo run -- --verbose upload ./test.pdf
//
// Upload with custom bucket:
// cargo run -- --bucket my-bucket --verbose upload ./test.pdf
//
// Download quietly:
// cargo run download test.pdf
//
// Download with custom output:
// cargo run -- --verbose download test.pdf --output ./downloads/test.pdf
//
// Generate presigned URL:
// cargo run -- --verbose download test.pdf --presign --expires 1800
//
// List files:
// cargo run -- --verbose list
//
// List with prefix:
// cargo run -- --verbose list --prefix images/
//
// Delete file:
// cargo run -- --verbose delete test.pdf
