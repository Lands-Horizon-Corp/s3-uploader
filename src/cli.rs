use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "s3-storage")]
#[command(about = "Upload and download files from S3-compatible storage")]
#[command(version = "1.0")]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Storage bucket name (overrides env STORAGE_BUCKET)
    #[arg(long, global = true)]
    pub bucket: Option<String>,

    /// Storage region (overrides env STORAGE_REGION)
    #[arg(long, global = true)]
    pub region: Option<String>,

    /// Storage access key (overrides env STORAGE_ACCESS_KEY)
    #[arg(long, global = true)]
    pub access_key: Option<String>,

    /// Storage secret key (overrides env STORAGE_SECRET_KEY)
    #[arg(long, global = true)]
    pub secret_key: Option<String>,

    /// Storage endpoint URL (overrides env STORAGE_URL)
    #[arg(long, global = true)]
    pub endpoint: Option<String>,

    /// Maximum file size in bytes (overrides env STORAGE_MAX_SIZE)
    #[arg(long, global = true, default_value_t = 100 * 1024 * 1024)]
    pub max_size: u64,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Upload a file to storage
    Upload {
        file_path: String,
        #[arg(long, default_value_t = 3600)]
        expires: u64,
    },

    /// Download a file from storage
    Download {
        file_name: String,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        presign: bool,
        #[arg(long, default_value_t = 3600)]
        expires: u64,
    },

    /// List files in storage bucket
    List {
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: i32,
    },

    /// Delete a file from storage
    Delete { file_name: String },

    /// Start web UI server
    Server {
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}
