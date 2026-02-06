use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;

mod cli;
mod commands;
mod config;
mod s3_client;
mod utils;

use cli::{Cli, Commands};
use commands::{delete, download, list, upload};
use config::StorageConfig;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let cli = Cli::parse();
    let config = StorageConfig::load_from_cli(&cli)?;

    match &cli.command {
        Commands::Upload { file_path, expires } => {
            let info =
                upload::upload_file(&file_path, &config, cli.verbose, Some(*expires)).await?;
            println!("Uploaded: {} -> {}", info.file_name, info.download_url);
        }
        Commands::Download {
            file_name,
            output,
            presign,
            expires,
        } => {
            download::download_file(
                &file_name,
                output.as_deref(),
                *presign,
                *expires, // just pass u64, no Option
                &config,
                cli.verbose,
            )
            .await?;
        }
        Commands::List { prefix, limit } => {
            list::list_files(prefix.as_deref(), *limit, &config, cli.verbose).await?;
        }
        Commands::Delete { file_name } => {
            delete::delete_file(&file_name, &config, cli.verbose).await?;
        }
    }
    Ok(())
}
