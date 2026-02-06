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

    println!("🚀 Starting upload handler");

    let mut uploaded_files = Vec::new();
    let mut ttl_seconds: u64 = 3600; // default 1 hour

    while let Ok(Some(mut field)) = multipart.next_field().await {
        println!("🔹 Processing field: {:?}", field.name());

        match field.name() {
            Some("file") => {
                if let Some(filename) = field.file_name() {
                    println!("📄 Uploading file: {}", filename);
                    let filename = filename.to_string();
                    let temp_path = std::env::temp_dir().join(&filename);

                    // Stream chunks to temp file
                    match File::create(&temp_path).await {
                        Ok(mut file) => {
                            println!("📝 Created temp file: {:?}", temp_path);

                            while let Ok(Some(chunk)) = field.chunk().await {
                                if verbose {
                                    println!("⬇️ Writing chunk: {} bytes", chunk.len());
                                }
                                if let Err(e) = file.write_all(&chunk).await {
                                    eprintln!("❌ Failed writing chunk: {:?}", e);
                                    return Html(format!("Failed to write file: {:?}", e));
                                }
                            }

                            uploaded_files.push(temp_path.clone());
                            println!("✅ File saved successfully: {:?}", temp_path);
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to create temp file: {:?}", e);
                            return Html(format!("Failed to create temp file: {:?}", e));
                        }
                    }
                }
            }
            Some("ttl_value") => {
                if let Ok(val) = u64::from_str(&field.text().await.unwrap_or_default()) {
                    ttl_seconds = val;
                    println!("⏱ TTL value set: {}", ttl_seconds);
                }
            }
            Some("ttl_unit") => {
                let unit = field.text().await.unwrap_or_default();
                ttl_seconds = match unit.as_str() {
                    "minutes" => ttl_seconds.saturating_mul(60),
                    "hours" => ttl_seconds.saturating_mul(3600),
                    _ => ttl_seconds,
                };
                println!("⏱ TTL unit applied: {} seconds", ttl_seconds);
            }
            other => {
                println!("⚠️ Ignored field: {:?}", other);
            }
        }
    }

    if uploaded_files.is_empty() {
        eprintln!("❌ No files uploaded");
        return Html("No file uploaded".to_string());
    }

    // Upload each file to S3 and spawn TTL deletion
    let mut results = Vec::new();
    for path in uploaded_files {
        println!("🚀 Uploading to S3: {:?}", path);

        match crate::commands::upload::upload_file(
            &path.to_string_lossy(),
            &config,
            verbose,
            Some(ttl_seconds),
        )
        .await
        {
            Ok(info) => {
                println!("✅ Upload completed: {}", info.download_url);

                // Spawn background task to delete temp file after TTL
                let path_clone = path.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(ttl_seconds)).await;
                    match tokio::fs::remove_file(&path_clone).await {
                        Ok(_) => println!("🗑️ File {:?} deleted after TTL", path_clone),
                        Err(e) => eprintln!(
                            "❌ Failed to delete file {:?} after TTL: {:?}",
                            path_clone, e
                        ),
                    }
                });

                results.push(format!(
                    "<p>File: {} uploaded successfully! <br>Download: <a href='{}'>{}</a> <br>Expires in: {} seconds</p>",
                    info.file_name, info.download_url, info.download_url, ttl_seconds
                ));
            }
            Err(e) => {
                eprintln!("❌ Upload failed for {:?}: {:?}", path, e);
                results.push(format!(
                    "<p>Upload failed for {:?}: {:?}</p>",
                    path.file_name().unwrap(),
                    e
                ));
            }
        }
    }

    Html(results.join("<hr>"))
}
