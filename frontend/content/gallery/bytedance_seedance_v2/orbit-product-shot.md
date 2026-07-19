---
id: orbit-product-shot
model: bytedance_seedance_v2
task: image_to_video
title: Orbiting product hero (image-to-video)
techniques: [orbit, studio-lighting, product]
aspect: "16:9"
prompt: "Slowly orbit around the serum bottle on its podium, keeping it centered and in focus, soft studio lighting, subtle specular highlights tracking across the glass, seamless loop."
params:
  duration: 4
  resolution: "1080p"
  camera: "orbit"
  aspect_ratio: "16:9"
  seed: 14550
asset: samples/bytedance_seedance_v2/orbit-product-shot.mp4
poster: samples/bytedance_seedance_v2/orbit-product-shot.poster.jpg
source_asset: samples/google_nano_banana_2/product-serum-bottle.png
job_id: job_01JABCORBIT
featured: true
weight: 90
---
This is **image-to-video**: the still product shot (generated with Nano Banana 2) is the first frame, and Seedance only animates the camera. Constrain it — "keeping it centered and in focus", "seamless loop" — so the model moves the camera, not the product.
