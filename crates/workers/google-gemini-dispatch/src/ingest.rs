//! Output ingestion: provider bytes → R2 object → `files` row → `outputs` row.
//! Upload happens OUTSIDE any tx (an orphaned object behind an unguessable key
//! is harmless; a dangling row is not).

use sha2::{Digest, Sha256};

use bucket_client::UploadBytesArgs;
use bucket_paths::{mint_bucket_hash, object_key, BucketFolder, ObjectKeyParts};
use db_queries::{files, outputs};

use crate::context::DispatchContext;

pub struct IngestOutputArgs<'a> {
  pub account_id: &'a ids::AccountId,
  pub job_id: &'a ids::JobId,
  /// Position in the job's output list (= slice_index for fanned jobs).
  pub output_index: i32,
  /// `image` or `video` (outputs.kind).
  pub kind: &'a str,
  pub bytes: &'a [u8],
  pub mime: &'a str,
  /// Bucket-path extension (with the dot), e.g. `.png`, `.mp4`.
  pub extension: &'a str,
}

/// Store one output: R2 upload, `files` row, `outputs` row.
pub async fn ingest_output(ctx: &DispatchContext, args: IngestOutputArgs<'_>) -> Result<(), String> {
  let bucket = ctx.bucket.as_ref().ok_or("output storage (R2) is not configured")?;
  let hash = mint_bucket_hash();
  let key = object_key(ObjectKeyParts {
    folder: BucketFolder::Generations,
    hash: &hash,
    prefix: None,
    suffix: None,
    version: None,
    extension: Some(args.extension),
  });
  bucket
    .upload_bytes(UploadBytesArgs { object_key: &key, bytes: args.bytes, content_type: Some(args.mime) })
    .await
    .map_err(|e| format!("R2 upload failed: {e}"))?;
  let file = files::create_file(
    &ctx.pool,
    files::NewFile {
      account_id: args.account_id,
      purpose: "output",
      origin: "generated",
      bucket_folder: BucketFolder::Generations.as_str(),
      bucket_hash: &hash,
      bucket_extension: Some(args.extension),
      mime: args.mime,
      bytes: args.bytes.len() as i64,
      checksum_sha256: &hex::encode(Sha256::digest(args.bytes)),
    },
  )
  .await
  .map_err(|e| format!("files insert failed: {e}"))?;
  outputs::create_output(
    &ctx.pool,
    outputs::NewOutput {
      job_id: args.job_id,
      file_id: &file.id,
      output_index: args.output_index,
      kind: args.kind,
      width: None,
      height: None,
    },
  )
  .await
  .map_err(|e| format!("outputs insert failed: {e}"))?;
  Ok(())
}
