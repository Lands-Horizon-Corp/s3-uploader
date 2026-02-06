

# S3 Storage CLI

A **Rust CLI tool** for managing files on **S3-compatible storage**. You can upload, download, list, delete files, and generate **presigned URLs** for secure temporary access.

---

## Features

* Upload files to an S3 bucket
* Download files from an S3 bucket
* Generate presigned URLs for temporary access
* List files with optional prefix filtering
* Delete files from the bucket
* Verbose mode for detailed output
* Fully configurable via **environment variables** or **CLI flags**
* Supports any **S3-compatible endpoint**

---

## Installation

1. Clone the repository:

```bash
git clone <your-repo-url>
cd <your-repo-folder>
```

2. Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

3. Build the project:

```bash
cargo build --release
```

4. Run the CLI:

```bash
cargo run -- <COMMAND>
```

---

## Configuration

### Environment Variables

| Variable             | Description                | Default              |
| -------------------- | -------------------------- | -------------------- |
| `STORAGE_BUCKET`     | Bucket name                | `default-bucket`     |
| `STORAGE_REGION`     | Storage region             | `us-east-1`          |
| `STORAGE_ACCESS_KEY` | Access key                 | *required*           |
| `STORAGE_SECRET_KEY` | Secret key                 | *required*           |
| `STORAGE_URL`        | S3-compatible endpoint URL | optional             |
| `STORAGE_MAX_SIZE`   | Max file size in bytes     | `104857600` (100 MB) |

### CLI Flags

All environment variables can be overridden:

```text
--bucket <BUCKET_NAME>
--region <REGION>
--access-key <ACCESS_KEY>
--secret-key <SECRET_KEY>
--endpoint <ENDPOINT_URL>
--max-size <BYTES>
--verbose
```

---

## Commands

### Upload

Upload a file to the S3 bucket:

```bash
cargo run -- upload <FILE_PATH>
```

**Options:**

* `--verbose` – Show detailed output
* `--bucket` – Specify a different bucket
* `--max-size` – Override max file size

**Example:**

```bash
cargo run -- --verbose upload ./example.pdf
```

---

### Download

Download a file or generate a presigned URL:

```bash
cargo run -- download <FILE_NAME>
```

**Options:**

* `--output <FILE_PATH>` – Save to custom location
* `--presign` – Generate presigned URL instead of downloading
* `--expires <SECONDS>` – Expiry for presigned URL (default: 3600)
* `--verbose` – Show detailed output

**Examples:**

```bash
# Download quietly
cargo run -- download example.pdf

# Download to a custom path
cargo run -- --verbose download example.pdf --output ./downloads/example.pdf

# Generate presigned URL
cargo run -- --verbose download example.pdf --presign --expires 1800
```

---

### List Files

List files in the bucket:

```bash
cargo run -- list
```

**Options:**

* `--prefix <PREFIX>` – Filter files by prefix
* `--limit <NUMBER>` – Max files to list (default: 100)
* `--verbose` – Show detailed output

**Examples:**

```bash
# List all files
cargo run -- --verbose list

# List files with prefix
cargo run -- --verbose list --prefix images/
```

---

### Delete File

Delete a file from the bucket:

```bash
cargo run -- delete <FILE_NAME>
```

**Options:**

* `--verbose` – Show detailed output

**Example:**

```bash
cargo run -- --verbose delete example.pdf
```

---

### Server

Start a web UI server:

```bash
cargo run -- server
```

**Options:**

* `--port <PORT>` – Port to run the server (default: 8080)

---

## Presigned URLs

* Allow temporary access to files without exposing credentials
* Default expiration: 1 hour (3600 seconds)
* Custom expiration: `--expires <SECONDS>`

---

## Max File Size

* Default: 100 MB
* Override with `--max-size <BYTES>` or `STORAGE_MAX_SIZE`

---

## Verbose Mode

Enable `--verbose` to see detailed steps during any operation.

---

## Examples

```bash
# Upload a file with verbose output
cargo run -- --verbose upload ./report.pdf

# Download a file quietly
cargo run -- download report.pdf

# Generate a presigned URL
cargo run -- download report.pdf --presign --expires 1800

# List files with prefix
cargo run -- list --prefix documents/

# Delete a file
cargo run -- delete report.pdf
```

---

## Dependencies

* [aws-sdk-s3](https://docs.rs/aws-sdk-s3)
* [tokio](https://docs.rs/tokio)
* [clap](https://docs.rs/clap)
* [dotenvy](https://docs.rs/dotenvy)
* [mime_guess](https://docs.rs/mime_guess)
* [anyhow](https://docs.rs/anyhow)

---

## License

MIT License © Your Name

---
