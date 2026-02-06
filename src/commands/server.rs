use crate::config::StorageConfig;
use anyhow::Result;
use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use axum_extra::extract::Multipart;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

pub async fn start_server(config: StorageConfig, verbose: bool, port: u16) -> Result<()> {
    let shared_config = Arc::new(config);

    let app = Router::new().route("/", get(index)).route(
        "/upload",
        post({
            let cfg = shared_config.clone();
            move |multipart: Multipart| handle_upload(multipart, cfg.clone(), verbose)
        }),
    );

    if verbose {
        println!("🚀 Server running at http://127.0.0.1:{}", port);
    }

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    let _ = axum::serve(listener, app).await;

    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
        <html>
        <body>
            <h1>S3 Uploader</h1>
            <form action="/upload" method="post" enctype="multipart/form-data">
                <input type="file" name="file" />
                <button type="submit">Upload</button>
            </form>
        </body>
        </html>
        "#,
    )
}

async fn handle_upload(
    mut multipart: Multipart,
    config: Arc<StorageConfig>,
    verbose: bool,
) -> Html<String> {
    let mut uploaded_path = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if let Some(filename) = field.file_name() {
            let filename = filename.to_string(); // clone to own the string
            let temp_path = std::env::temp_dir().join(&filename);
            let mut file = File::create(&temp_path).await.unwrap();
            let data = field.bytes().await.unwrap();
            file.write_all(&data).await.unwrap();
            uploaded_path = Some(temp_path);

            if verbose {
                println!("Uploaded file temporarily saved: {:?}", filename);
            }
        }
    }

    if let Some(path) = uploaded_path {
        match crate::commands::upload::upload_file(
            &path.to_string_lossy(),
            &config,
            verbose,
            Some(3600),
        )
        .await
        {
            Ok(info) => Html(format!(
                "<p>File uploaded successfully!</p><p>Download URL: <a href='{}'>{}</a></p>",
                info.download_url, info.download_url
            )),
            Err(e) => Html(format!("Upload failed: {:?}", e)),
        }
    } else {
        Html("No file uploaded".to_string())
    }
}
