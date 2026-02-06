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
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>S3 File Uploader</title>
        <script src="https://cdn.tailwindcss.com"></script>
    </head>
    <body class="bg-gray-100 flex items-center justify-center min-h-screen">
        <div class="bg-white p-8 rounded-lg shadow-lg w-full max-w-md">
            <h1 class="text-2xl font-bold mb-6 text-center text-gray-800">S3 File Uploader</h1>
            <form action="/upload" method="post" enctype="multipart/form-data" class="space-y-4">
                
                <!-- File input -->
                <div>
                    <label class="block text-gray-700 font-medium mb-1" for="file">Select file</label>
                    <input type="file" name="file" id="file" required
                        class="block w-full text-gray-700 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" />
                </div>

                <!-- Identifier input -->
                <div>
                    <label class="block text-gray-700 font-medium mb-1" for="identifier">Identifier</label>
                    <input type="text" name="identifier" id="identifier" placeholder="File identifier"
                        class="block w-full text-gray-700 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" />
                </div>

                <!-- TTL selector -->
                <div>
                    <label class="block text-gray-700 font-medium mb-1">Expiration (TTL)</label>
                    <div class="flex space-x-2">
                        <input type="number" name="ttl_value" min="1" max="100" value="1"
                            class="w-1/2 text-gray-700 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" />
                        <select name="ttl_unit"
                            class="w-1/2 text-gray-700 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="minutes">Minutes</option>
                            <option value="hours">Hours</option>
                        </select>
                    </div>
                </div>

                <!-- Submit button -->
                <button type="submit"
                    class="w-full bg-blue-600 text-white font-bold py-2 px-4 rounded-lg hover:bg-blue-700 transition">
                    Upload
                </button>
            </form>
        </div>
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
    use std::str::FromStr;

    let mut uploaded_path = None;
    let mut ttl_seconds: u64 = 3600; // default 1 hour

    // Loop through all fields
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name() {
            Some("file") => {
                if let Some(filename) = field.file_name() {
                    let filename = filename.to_string(); // clone to own
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
            Some("ttl_value") => {
                if let Ok(val) = u64::from_str(&field.text().await.unwrap_or_default()) {
                    ttl_seconds = val; // temporarily store, will convert with unit
                }
            }
            Some("ttl_unit") => {
                let unit = field.text().await.unwrap_or_default();
                ttl_seconds = match unit.as_str() {
                    "minutes" => ttl_seconds * 60,
                    "hours" => ttl_seconds * 3600,
                    _ => ttl_seconds,
                };
            }
            _ => {}
        }
    }

    if let Some(path) = uploaded_path {
        match crate::commands::upload::upload_file(
            &path.to_string_lossy(),
            &config,
            verbose,
            Some(ttl_seconds),
        )
        .await
        {
            Ok(info) => Html(format!(
                "<p>File uploaded successfully!</p>
                 <p>Expires in: {} seconds</p>
                 <p>Download URL: <a href='{}'>{}</a></p>",
                ttl_seconds, info.download_url, info.download_url
            )),
            Err(e) => Html(format!("Upload failed: {:?}", e)),
        }
    } else {
        Html("No file uploaded".to_string())
    }
}
