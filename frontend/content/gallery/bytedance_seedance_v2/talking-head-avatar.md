---
id: talking-head-avatar
model: bytedance_seedance_v2
task: image_to_video
title: Talking-head presenter loop
techniques: [dialogue, static, portrait]
aspect: "16:9"
prompt: "A friendly presenter facing the camera, natural blinking and subtle head movement, speaking gesture, soft key light, neutral studio background, locked-off framing, seamless loop for an explainer."
params:
  duration: 4
  resolution: "1080p"
  camera: "static"
  aspect_ratio: "16:9"
  seed: 27650
asset: samples/bytedance_seedance_v2/talking-head-avatar.mp4
poster: samples/bytedance_seedance_v2/talking-head-avatar.poster.jpg
job_id: job_01JABCHEAD
featured: false
weight: 72
---
Talking heads need **micro-motion, not macro**: "natural blinking and subtle head movement" avoids the uncanny frozen stare, while "locked-off framing" stops the camera from wandering. Great as a base layer for a lipsync pass.
