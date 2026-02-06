use anyhow::{bail, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub endpoint: Option<String>,
    pub max_size: u64,
}

impl StorageConfig {
    pub fn load_from_cli(cli: &crate::cli::Cli) -> Result<Self> {
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

        if access_key.is_empty() || secret_key.is_empty() {
            bail!("Access key and secret key must be provided via parameters or environment variables");
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
}
