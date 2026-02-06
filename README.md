

# S3 Storage CLI

A simple CLI tool written in Rust for **uploading, downloading, listing, and deleting files** from S3-compatible storage. It also supports **presigned URLs** for secure temporary access.

---

## Features

* Upload files to an S3 bucket
* Download files from an S3 bucket
* Generate presigned URLs for temporary file access
* List files with optional prefix filtering
* Delete files from the bucket
* Verbose mode for detailed output
* Configurable via environment variables or CLI options
* Supports S3-compatible storage endpoints

---

## Installation

1. Clone the repository:

```bash
git clone <your-repo-url>
cd <your-repo-folder>
```

2. Set up Rust:

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

You can configure the CLI either via **environment variables** or **CLI flags**.

### Environment Variables

| Variable             | Description                | Default              |
| -------------------- | -------------------------- | -------------------- |
| `STORAGE_BUCKET`     | Storage bucket name        | `default-bucket`     |
| `STORAGE_REGION`     | Storage region             | `us-east-1`          |
| `STORAGE_ACCESS_KEY` | Access key for storage     | *required*           |
| `STORAGE_SECRET_KEY` | Secret key for storage     | *required*           |
| `STORAGE_URL`        | S3-compatible endpoint URL | optional             |
| `STORAGE_MAX_SIZE`   | Maximum file size in bytes | `104857600` (100 MB) |

### CLI Options

All environment variables can be overridden with CLI flags:

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

### Upload a file

```bash
cargo run -- upload <FILE_PATH>
```

**Options:**

* `--verbose` – Show detailed output
* `--bucket` – Specify a custom bucket
* `--max-size` – Override maximum file size

**Example:**

```bash
cargo run -- --verbose upload ./example.pdf
```

---

### Download a file

```bash
cargo run -- download <FILE_NAME>
```

**Options:**

* `--output <FILE_PATH>` – Save file to custom path
* `--presign` – Generate presigned URL instead of downloading
* `--expires <SECONDS>` – Expiry time for presigned URL (default 3600)
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

### List files

```bash
cargo run -- list
```

**Options:**

* `--prefix <PREFIX>` – Filter files by prefix
* `--limit <NUMBER>` – Maximum number of files to list
* `--verbose` – Show detailed output

**Examples:**

```bash
# List all files
cargo run -- --verbose list

# List files with a prefix
cargo run -- --verbose list --prefix images/
```

---

### Delete a file

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

## Presigned URLs

* Presigned URLs allow temporary access to files without exposing credentials.
* Default expiration is **1 hour (3600 seconds)**.
* Custom expiration can be set with `--expires`.

---

## Max File Size

* By default, the maximum upload file size is **100 MB**.
* You can override with `--max-size <BYTES>` or `STORAGE_MAX_SIZE` environment variable.

---

## Verbose Mode

* Enable verbose mode with `--verbose` to see detailed steps during upload/download/list/delete operations.

---

## Examples

```bash
# Upload a file with verbose output
cargo run -- --verbose upload ./report.pdf

# Download a file quietly
cargo run -- download report.pdf

# Generate a presigned URL for a file
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