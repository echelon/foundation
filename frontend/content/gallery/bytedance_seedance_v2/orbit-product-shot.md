---
id: orbit-product-shot
model: bytedance_seedance_v2
task: image_to_video
category: Image-to-video
title: Orbit a product hero (from a still)
techniques: [orbit, studio-lighting, product]
aspect: "16:9"
prompt: "Slowly orbit around the serum bottle from the source image, keeping it centered and in focus, soft studio lighting, subtle specular highlights tracking across the glass, seamless loop."
params:
  duration: 4
  resolution: "1080p"
  camera: "orbit"
  aspect_ratio: "16:9"
  seed: 14550
asset: samples/bytedance_seedance_v2/orbit-product-shot.mp4
poster: samples/bytedance_seedance_v2/orbit-product-shot.poster.jpg
references: [samples/google_nano_banana_2/product-serum-bottle.png=Source image]
featured: true
weight: 90
---
**Image-to-video**: the still (made with Nano Banana 2) is the first frame; Seedance only animates the camera. Constrain it — "keeping it centered and in focus", "seamless loop" — so the model moves the camera, not the product.
