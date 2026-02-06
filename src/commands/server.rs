use crate::config::StorageConfig;
use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    response::Html,
    routing::{get, post},
    Router,
};
use axum_extra::extract::Multipart;
use bytes::Bytes;
use futures::StreamExt;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

pub async fn start_server(config: StorageConfig, verbose: bool, port: u16) -> Result<()> {
    println!("Starting server on 0.0.0.0:{}", port);
    let shared_config = Arc::new(config);

    let app = Router::new()
        .route("/", get(index))
        .route(
            "/upload",
            post({
                let cfg = shared_config.clone();
                move |multipart: Multipart| handle_upload(multipart, cfg.clone(), verbose)
            }),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024));

    if verbose {
        println!("🚀 Server running at http://0.0.0.0:{}", port);
    }

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

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

            <!-- Password input -->
            <div>
                <label class="block text-gray-700 font-medium mb-1" for="password">Password</label>
                <input type="password" name="password" id="password" required
                    class="block w-full text-gray-700 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" />
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
    use std::env;
    use std::str::FromStr;

    if verbose {
        println!("🚀 Starting upload handler");
    }

    let mut uploaded_files: Vec<std::path::PathBuf> = Vec::new();
    let mut identifier = String::new();
    let mut ttl_value: u64 = 1;
    let mut ttl_unit = "hours".to_string();
    let mut password = String::new();

    while let Some(mut field) = multipart.next_field().await.ok().flatten() {
        let name = field.name().map(|s| s.to_string());

        if verbose {
            println!("🔹 Processing field: {:?}", name);
        }

        if let Some(n) = name {
            match n.as_str() {
                "file" => {
                    let filename = field
                        .file_name()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unnamed".to_string());

                    if verbose {
                        println!("📄 Uploading file: {}", filename);
                    }

                    let temp_path = std::env::temp_dir().join(&filename);

                    let mut file = match File::create(&temp_path).await {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("❌ Failed to create temp file: {:?}", e);
                            return Html(format!("Failed to create temp file: {:?}", e));
                        }
                    };

                    while let Some(chunk_res) = field.next().await {
                        let chunk: Bytes = match chunk_res {
                            Ok(c) => c,
                            Err(e) => {
                                eprintln!("❌ Error in stream: {:?}", e);
                                return Html(format!("Error reading file: {:?}", e));
                            }
                        };

                        if verbose {
                            println!("⬇️ Writing chunk: {} bytes", chunk.len());
                        }
                        if let Err(e) = file.write_all(&chunk).await {
                            eprintln!("❌ Failed writing chunk: {:?}", e);
                            return Html(format!("Failed to write file: {:?}", e));
                        }
                    }

                    uploaded_files.push(temp_path.clone());

                    if verbose {
                        println!("✅ File saved successfully: {:?}", temp_path);
                    }
                }
                "identifier" => match field.text().await {
                    Ok(text) => {
                        identifier = text;
                        if verbose {
                            println!("🆔 Identifier set: {}", identifier);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to read identifier: {:?}", e);
                    }
                },
                "ttl_value" => match field.text().await {
                    Ok(text) => {
                        if let Ok(val) = u64::from_str(&text) {
                            ttl_value = val;
                            if verbose {
                                println!("⏱ TTL value set: {}", ttl_value);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to read ttl_value: {:?}", e);
                    }
                },
                "ttl_unit" => match field.text().await {
                    Ok(text) => {
                        ttl_unit = text;
                        if verbose {
                            println!("⏱ TTL unit set: {}", ttl_unit);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to read ttl_unit: {:?}", e);
                    }
                },
                "password" => match field.text().await {
                    Ok(text) => {
                        password = text;
                        if verbose {
                            println!("🔑 Password received");
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to read password: {:?}", e);
                    }
                },
                other => {
                    if verbose {
                        println!("⚠️ Ignored field: {:?}", other);
                    }
                }
            }
        }
    }

    let expected_password = match env::var("PASSWORD") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to read PASSWORD env var: {:?}", e);
            return Html("Server configuration error".to_string());
        }
    };

    if password != expected_password {
        if verbose {
            println!("❌ Invalid password");
        }
        return Html("Invalid password".to_string());
    }

    if verbose {
        println!("✅ Password validated");
    }

    let mut ttl_seconds: u64 = ttl_value;
    ttl_seconds = match ttl_unit.as_str() {
        "minutes" => ttl_seconds.saturating_mul(60),
        "hours" => ttl_seconds.saturating_mul(3600),
        _ => ttl_seconds,
    };

    if verbose {
        println!("⏱ TTL calculated: {} seconds", ttl_seconds);
    }

    // Handle identifier for single file (rename temp file if provided)
    if !identifier.is_empty() && uploaded_files.len() == 1 {
        let old_path = uploaded_files[0].clone();
        let ext = old_path
            .extension()
            .map(|e| e.to_string_lossy().to_string());
        let new_filename = match ext {
            Some(e) => format!("{}.{}", identifier, e),
            None => identifier.clone(),
        };
        let new_path = old_path.with_file_name(new_filename);

        if let Err(e) = tokio::fs::rename(&old_path, &new_path).await {
            eprintln!("❌ Failed to rename file: {:?}", e);
            return Html(format!("Failed to rename file: {:?}", e));
        }

        uploaded_files[0] = new_path.clone();

        if verbose {
            println!("🔄 File renamed to: {:?}", new_path);
        }
    }

    if uploaded_files.is_empty() {
        eprintln!("❌ No files uploaded");
        return Html("No file uploaded".to_string());
    }

    // Upload each file to S3
    let mut results = Vec::new();
    for path in uploaded_files {
        if verbose {
            println!("🚀 Uploading to S3: {:?}", path);
        }

        let upload_result = crate::commands::upload::upload_file(
            &path.to_string_lossy().to_string(),
            &config,
            verbose,
            Some(ttl_seconds),
        )
        .await;

        match upload_result {
            Ok(info) => {
                if verbose {
                    println!("✅ Upload completed: {}", info.download_url);
                }
                results.push(format!(
                "<p>File: {} uploaded successfully! <br>Download: <a href='{}'>{}</a> <br>Expires in: {} seconds</p>",
                info.file_name, info.download_url, info.download_url, ttl_seconds
            ));
            }
            Err(e) => {
                eprintln!("❌ Upload failed for {:?}: {:?}", path, e);
                results.push(format!(
                    "<p>Upload failed for {}: {:?}</p>",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    e
                ));
            }
        }

        // Always delete the temp file immediately after upload attempt
        if let Err(e) = tokio::fs::remove_file(&path).await {
            eprintln!("❌ Failed to delete temp file {:?}: {:?}", path, e);
        } else if verbose {
            println!("🗑️ Temp file deleted: {:?}", path);
        }
    }

    Html(results.join("<hr>"))
}
