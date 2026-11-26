//! bucket_client — a small, ergonomic client for S3-compatible object storage (Cloudflare R2 in
//! production, minio locally), built on the `rust-s3` crate.
//!
//! Construct one with [`BucketClientBuilder`] (access key, secret, region, bucket, and — for
//! R2/minio — a custom `endpoint`), then move bytes with [`BucketClient::upload_bytes`],
//! [`BucketClient::download_bytes`], and [`BucketClient::delete_object`]. Object keys are
//! normalized via [`ObjectKey`] (leading slashes stripped — S3/R2 would otherwise mint an object
//! literally named `/foo`).
//!
//! Live round-trip tests (upload → download → delete against a real bucket) are `#[ignore]`d and
//! read credentials from `{repo}/external/credentials/r2_*.txt` — see `tests` in
//! `bucket_client.rs`.

pub mod bucket_client;
pub mod bucket_client_builder;
pub mod bucket_client_error;
pub mod object_key;

pub use bucket_client::{BucketClient, SignedRestoreRequest, UploadBytesArgs};
pub use bucket_client_builder::BucketClientBuilder;
pub use bucket_client_error::BucketClientError;
pub use object_key::ObjectKey;
