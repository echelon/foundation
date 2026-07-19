---
id: i2v-landscape-drift
model: bytedance_seedance_v2
task: image_to_video
category: Image-to-video
title: Bring a landscape still to life
techniques: [pan, landscape, slow-motion]
aspect: "16:9"
prompt: "Animate the source landscape: drifting clouds, rippling water, swaying grass, a slow gentle push-in, keep the horizon and composition of the source image, natural motion, no new objects."
params:
  duration: 5
  resolution: "1080p"
  camera: "dolly"
  aspect_ratio: "16:9"
  seed: 33210
asset: samples/bytedance_seedance_v2/i2v-landscape-drift.mp4
poster: samples/bytedance_seedance_v2/i2v-landscape-drift.poster.jpg
references: [samples/bytedance_seedance_v2/landscape-ref.png=Source image]
featured: false
weight: 76
---
Living-photo look: list the **elements that should move** ("clouds, water, grass") and forbid invention ("no new objects"). A "slow gentle push-in" adds life without breaking the source composition.
