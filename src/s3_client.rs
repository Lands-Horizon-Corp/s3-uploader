use anyhow::Result;
use aws_config::SdkConfig;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::Client;

use crate::config::StorageConfig;

pub async fn create_client(config: &StorageConfig, verbose: bool) -> Result<Client> {
    if verbose {
        println!("🔧 Creating S3 client for bucket {}", config.bucket);
    }

    let credentials = Credentials::new(
        config.access_key.clone(),
        config.secret_key.clone(),
        None,
        None,
        "custom",
    );

    let region = Region::new(config.region.clone());

    let mut loader = aws_config::ConfigLoader::default()
        .region(region)
        .credentials_provider(credentials);

    if let Some(endpoint) = &config.endpoint {
        loader = loader.endpoint_url(endpoint);
    }

    let sdk_config: SdkConfig = loader.load().await;

    let client_config = aws_sdk_s3::config::Builder::from(&sdk_config)
        .behavior_version_latest()
        .build();

    Ok(Client::from_conf(client_config))
}
