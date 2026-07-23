//! Router build + pricing tests: fan-out counts, param pruning/coercion with recorded
//! adjustments, pricing delegation to the provider client, and validation rejections.

use client_core::CostCalculator;
use foundation_router::error::RouterError;
use foundation_router::image::build::build_image_route;
use foundation_router::image::plan::{
  ImageGenerationPlan, RouterImageModel, RouterImageQuality, RouterInputFidelity,
};
use foundation_router::image::route::ImageRoute;
use foundation_router::input_strategy::{input_strategy_for, InputStrategy};
use foundation_router::pricing::PriceCertainty;

fn plan(model: RouterImageModel) -> ImageGenerationPlan {
  ImageGenerationPlan::new(model, "a red fox")
}

#[test]
fn every_model_routes_and_fans_out_n() {
  for model in [
    RouterImageModel::OpenaiGptImageV1,
    RouterImageModel::OpenaiGptImageV1Mini,
    RouterImageModel::OpenaiGptImageV1_5,
    RouterImageModel::OpenaiGptImageV2,
  ] {
    let mut p = plan(model);
    p.n = 4;
    let built = build_image_route(p).unwrap();
    assert_eq!(built.route.fan_out(), 4, "{model:?}");
    assert_eq!(built.route.model(), model);
    assert!(built.adjustments.is_empty(), "{model:?}: {:?}", built.adjustments);
  }
}

#[test]
fn slugs_route_and_match_the_catalog() {
  for slug in [
    "openai_gpt_image_1",
    "openai_gpt_image_1_mini",
    "openai_gpt_image_1_5",
    "openai_gpt_image_2",
  ] {
    let model = RouterImageModel::from_slug(slug).unwrap();
    assert_eq!(model.slug(), slug);
    assert_eq!(model.spec().provider, catalog::Provider::Openai);
  }
  for slug in ["google_nano_banana", "google_nano_banana_pro", "google_nano_banana_2", "google_nano_banana_2_lite"] {
    let model = RouterImageModel::from_slug(slug).unwrap();
    assert_eq!(model.slug(), slug);
    assert_eq!(model.spec().provider, catalog::Provider::Google);
  }
}

#[test]
fn google_routes_map_and_prune_the_gpt_flavored_params() {
  use foundation_router::input_strategy::InputStrategy;
  use foundation_router::image::plan::{RouterImageQuality, RouterImageSize};
  let mut p = plan(RouterImageModel::GoogleNanoBanana2);
  p.n = 3;
  p.size = Some(RouterImageSize::Landscape1536x1024);
  p.quality = Some(RouterImageQuality::High);
  p.output_format = Some(foundation_router::image::plan::RouterOutputFormat::Png);
  p.image_urls = vec!["https://cdn.example/in.png".into()];
  let built = build_image_route(p).unwrap();
  // size→aspect + quality→tier coerced, output_format pruned; inputs pend for upload-first.
  assert_eq!(built.route.fan_out(), 3);
  assert_eq!(built.route.input_strategy(), InputStrategy::ProviderUpload);
  assert_eq!(built.route.input_urls(), ["https://cdn.example/in.png"]);
  let params: Vec<&str> = built.adjustments.iter().map(|a| a.param.as_str()).collect();
  assert!(params.contains(&"size"));
  assert!(params.contains(&"quality"));
  assert!(params.contains(&"output_format"));
  // Priced per slice through the client estimator: 3 × $0.151 (4K tier).
  let quote = built.route.price();
  assert_eq!(quote.amount_micros, 3 * 151_000);
  assert_eq!(quote.line_items.len(), 3);
}

#[test]
fn veo_routes_build_price_exactly_and_coerce_lite_4k() {
  use foundation_router::video::build::build_video_route;
  use foundation_router::video::plan::*;
  let built = build_video_route(VideoGenerationPlan {
    model: RouterVideoModel::GoogleVeo3_1Lite,
    prompt: "a candle flame".into(),
    negative_prompt: None,
    image_url: None,
    n: 2,
    resolution: Some(RouterVideoResolution::K4),
    duration: Some(RouterVideoDuration::S4),
    aspect_ratio: Some(RouterVideoAspectRatio::Portrait9x16),
  })
  .unwrap();
  // Lite has no 4K → coerced to 1080p, which forces 8 s.
  let params: Vec<&str> = built.adjustments.iter().map(|a| a.param.as_str()).collect();
  assert!(params.contains(&"resolution"));
  assert!(params.contains(&"duration_seconds"));
  assert_eq!(built.route.fan_out(), 2);
  // 2 videos × 8 s × $0.08/s (lite 1080p) = $1.28, EXACT.
  let quote = built.route.price();
  assert_eq!(quote.amount_micros, 2 * 8 * 80_000);
  assert!(matches!(quote.certainty, foundation_router::pricing::PriceCertainty::Exact));
}

#[test]
fn gpt_image_2_prunes_input_fidelity_with_a_recorded_adjustment() {
  let mut p = plan(RouterImageModel::OpenaiGptImageV2);
  p.image_urls = vec!["https://cdn.example/in.png".into()];
  p.input_fidelity = Some(RouterInputFidelity::High);
  let built = build_image_route(p).unwrap();
  assert_eq!(built.adjustments.len(), 1);
  assert_eq!(built.adjustments[0].param, "input_fidelity");
  // …and the models that DO support it keep it, with no adjustment.
  let mut p = plan(RouterImageModel::OpenaiGptImageV1_5);
  p.image_urls = vec!["https://cdn.example/in.png".into()];
  p.input_fidelity = Some(RouterInputFidelity::High);
  let built = build_image_route(p).unwrap();
  assert!(built.adjustments.is_empty());
  match built.route {
    ImageRoute::OpenaiGptImage1_5(reqs) => assert!(reqs[0].input_fidelity.is_some()),
    other => panic!("wrong route: {other:?}"),
  }
}

#[test]
fn oversize_n_is_clamped_and_recorded() {
  let mut p = plan(RouterImageModel::OpenaiGptImageV1Mini);
  p.n = 25;
  let built = build_image_route(p).unwrap();
  assert_eq!(built.route.fan_out(), 10);
  assert_eq!(built.adjustments[0].param, "n");
}

#[test]
fn validation_rejections() {
  let mut empty = plan(RouterImageModel::OpenaiGptImageV1);
  empty.prompt = "  ".into();
  assert!(matches!(build_image_route(empty), Err(RouterError::Invalid(_))));

  let mut zero = plan(RouterImageModel::OpenaiGptImageV1);
  zero.n = 0;
  assert!(matches!(build_image_route(zero), Err(RouterError::Invalid(_))));

  let mut orphan_mask = plan(RouterImageModel::OpenaiGptImageV1);
  orphan_mask.mask_url = Some("https://cdn.example/mask.png".into());
  assert!(matches!(build_image_route(orphan_mask), Err(RouterError::Invalid(_))));

  let mut bad_url = plan(RouterImageModel::OpenaiGptImageV1);
  bad_url.image_urls = vec!["file:///etc/passwd".into()];
  assert!(matches!(build_image_route(bad_url), Err(RouterError::Invalid(_))));
}

#[test]
fn pricing_delegates_to_the_client_and_scales_with_n() {
  // Router price for n slices == n × the client's own calculator for one request.
  let mut p = plan(RouterImageModel::OpenaiGptImageV2);
  p.quality = Some(RouterImageQuality::High);
  p.n = 3;
  let built = build_image_route(p).unwrap();
  let quote = built.route.price();
  let per_slice = match &built.route {
    ImageRoute::OpenaiGptImage2(reqs) => reqs[0].estimate_cost().amount.micros().0,
    other => panic!("wrong route: {other:?}"),
  };
  assert_eq!(quote.amount_micros, per_slice * 3);
  assert_eq!(quote.line_items.len(), 3);
  assert_eq!(quote.certainty, PriceCertainty::Estimated);
  assert_eq!(per_slice, 211_000); // gpt-image-2 high, 1024² — pinned in the client's cost tests
}

#[test]
fn cheaper_models_price_cheaper() {
  let price = |model| {
    let mut p = plan(model);
    p.quality = Some(RouterImageQuality::Medium);
    build_image_route(p).unwrap().route.price().amount_micros
  };
  assert!(price(RouterImageModel::OpenaiGptImageV1Mini) < price(RouterImageModel::OpenaiGptImageV1));
  assert!(price(RouterImageModel::OpenaiGptImageV1_5) < price(RouterImageModel::OpenaiGptImageV2));
}

#[test]
fn openai_takes_public_urls() {
  assert_eq!(input_strategy_for(catalog::Provider::Openai), InputStrategy::PublicUrls);
  assert_eq!(input_strategy_for(catalog::Provider::Kinovi), InputStrategy::ProviderUpload);
}
