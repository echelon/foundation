//! Enqueue: validate + route + price + persist. Commits a `queued` job (with slices when
//! fanned out) and returns — the dispatcher takes it from there. No provider I/O here.

use chrono::{Duration, Utc};
use enums::{ApiShape, JobEventType, JobSliceStatus, JobSource, JobStatus};
use foundation_router::image::build::build_image_route;
use foundation_router::image::plan::RouterImageModel;
use ids::{AccountId, FileId, UserId};

use db_queries::{job_events, job_slices, jobs};

use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::{job_body, JobBody};
use crate::generation::stored_params::StoredImageParams;
use crate::state::AppState;

/// How long an undispatched/unfinished job lives before expiry.
const JOB_TTL_HOURS: i64 = 24;

/// Resolve request image refs (file ids or https URLs) into concrete URLs for the router.
pub async fn resolve_image_refs(
  state: &AppState,
  account_id: &AccountId,
  refs: &[String],
) -> ApiResult<Vec<String>> {
  let mut urls = Vec::with_capacity(refs.len());
  for r in refs {
    urls.push(resolve_one(state, account_id, r).await?);
  }
  Ok(urls)
}

pub async fn resolve_one(state: &AppState, account_id: &AccountId, raw: &str) -> ApiResult<String> {
  if let Ok(file_id) = raw.parse::<FileId>() {
    let file = db_queries::files::find_file_for_account(&state.pool, account_id, &file_id)
      .await?
      .ok_or_else(|| ApiError::BadRequest(format!("unknown file id: {raw}")))?;
    let folder = match file.bucket_folder.as_str() {
      "generations" => bucket_paths::BucketFolder::Generations,
      "samples" => bucket_paths::BucketFolder::Samples,
      "avatars" => bucket_paths::BucketFolder::Avatars,
      _ => bucket_paths::BucketFolder::Uploads,
    };
    let key = bucket_paths::object_key(bucket_paths::ObjectKeyParts {
      folder,
      hash: &file.bucket_hash,
      prefix: None,
      suffix: file.bucket_suffix.as_deref(),
      version: None,
      extension: file.bucket_extension.as_deref(),
    });
    return Ok(bucket_paths::cdn_url(&state.cdn_base_url, &key));
  }
  if raw.starts_with("https://") {
    // NOTE (assets.md deviation, v0): https URLs are passed through to the provider rather
    // than mirrored-first. Mirroring + full SSRF policy is a fast-follow.
    return Ok(raw.to_string());
  }
  Err(ApiError::BadRequest(format!(
    "image inputs must be a `file_…` id or an https:// URL, got: {raw}"
  )))
}

/// Everything enqueue needs to know about the caller/door.
pub struct EnqueueContext<'a> {
  pub account_id: &'a AccountId,
  pub user_id: Option<&'a UserId>,
  pub source: JobSource,
  pub api_shape: ApiShape,
  /// Dry run: admit, price, and log the exact outbound payload(s) — but never send. The
  /// job is born terminal (`canceled` / failure_code `dry_run`).
  pub dry_run: bool,
}

/// Validate, route, price, persist — returns the initial Job resource (201 body).
pub async fn enqueue_image_job(
  state: &AppState,
  ctx: EnqueueContext<'_>,
  params: StoredImageParams,
) -> ApiResult<JobBody> {
  let model = RouterImageModel::from_slug(&params.model)
    .ok_or_else(|| ApiError::BadRequest(format!("unknown or unrouted model: {}", params.model)))?;
  let plan = params.to_plan().ok_or_else(|| ApiError::Internal("plan rebuild failed".into()))?;

  let built = build_image_route(plan).map_err(|e| ApiError::BadRequest(e.to_string()))?;
  let quote = built.route.price();
  let fan_out = built.route.fan_out() as i32;
  let spec = model.spec();
  let modality = if params.image_urls.is_empty() { "text_to_image" } else { "image_edit" };

  // Persist the SANITIZED params (post-clamp n) so the dispatcher rebuilds identically.
  let mut stored = params.clone();
  stored.n = fan_out as u8;
  let request_params = serde_json::to_value(&stored).map_err(|e| ApiError::Internal(e.to_string()))?;
  let adjustments = serde_json::to_value(&built.adjustments).map_err(|e| ApiError::Internal(e.to_string()))?;

  let mut tx = state.pool.begin().await?;
  let job = jobs::create_job(
    &mut *tx,
    jobs::NewJob {
      account_id: ctx.account_id,
      api_key_id: None,
      user_id: ctx.user_id,
      source: ctx.source,
      media_type: "image",
      model: spec.slug,
      modality,
      api_shape: ctx.api_shape,
      api_version: Utc::now().date_naive(),
      request_params: &request_params,
      param_adjustments: &adjustments,
      provider: "openai",
      unit_amount_nanos: spec.price_nanos,
      quantity: fan_out as f64,
      estimated_max_micros: quote.max_micros.or(Some(quote.amount_micros)),
      is_dry_run: ctx.dry_run,
      expires_at: Utc::now() + Duration::hours(JOB_TTL_HOURS),
    },
  )
  .await?;
  // Slices ONLY when the request actually fans out (n > 1).
  let slices = if fan_out > 1 {
    job_slices::create_job_slices(&mut *tx, &job.id, fan_out).await?
  } else {
    Vec::new()
  };
  job_events::create_job_event(&mut *tx, &job.id, JobEventType::Created, None).await?;

  if ctx.dry_run {
    // Log the EXACT outbound payload(s) the dispatcher would have sent — one provider_calls
    // row per would-be call — then land the job terminal without ever touching the provider.
    for index in 0..built.route.fan_out() {
      crate::generation::provider_log::log_outbound_image_request(
        &mut *tx,
        crate::generation::provider_log::LogOutboundImageRequestArgs {
          job_id: &job.id,
          slice: slices.get(index),
          route: &built.route,
          index,
          is_dry_run: true,
        },
      )
      .await?;
    }
    for slice in &slices {
      job_slices::complete_job_slice(
        &mut *tx,
        job_slices::CompleteJobSlice {
          id: &slice.id,
          status: JobSliceStatus::Canceled,
          failure_code: Some("dry_run"),
          failure_message: Some("dry run — request was not sent to the provider"),
        },
      )
      .await?;
    }
    jobs::complete_job(
      &mut *tx,
      jobs::CompleteJob {
        id: &job.id,
        status: JobStatus::Canceled,
        failure_code: Some("dry_run"),
        failure_message: Some("dry run — request was not sent to the provider"),
        canceled_by: Some("user"),
      },
    )
    .await?;
    job_events::create_job_event(&mut *tx, &job.id, JobEventType::Canceled, Some(serde_json::json!({ "dry_run": true })))
      .await?;
    tx.commit().await?;
    return load_job_body(state, ctx.account_id, &job.id).await;
  }

  tx.commit().await?;

  Ok(job_body(state, &job, &slices, &[]))
}

/// Load the full Job resource for polling responses.
pub async fn load_job_body(state: &AppState, account_id: &AccountId, job_id: &ids::JobId) -> ApiResult<JobBody> {
  let job = jobs::find_job_for_account(&state.pool, account_id, job_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("job not found".into()))?;
  let slices = job_slices::list_slices_for_job(&state.pool, &job.id).await?;
  let outputs = db_queries::outputs::list_outputs_with_files_for_job(&state.pool, &job.id).await?;
  Ok(job_body(state, &job, &slices, &outputs))
}
