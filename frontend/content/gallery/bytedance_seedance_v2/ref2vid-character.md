---
id: ref2vid-character
model: bytedance_seedance_v2
task: reference_to_video
category: Reference-to-video
title: Put a referenced character into a new shot
techniques: [reference-consistency, dialogue, cinematic-lighting]
aspect: "16:9"
prompt: "Using the subject in reference 1 and the color/style of reference 2, create a cinematic medium shot of the character turning to camera and smiling, shallow depth of field, soft key light, consistent face and wardrobe."
params:
  duration: 4
  resolution: "1080p"
  camera: "static"
  aspect_ratio: "16:9"
  seed: 44120
asset: samples/bytedance_seedance_v2/ref2vid-character.mp4
poster: samples/bytedance_seedance_v2/ref2vid-character.poster.jpg
references: [samples/google_nano_banana_2/char-ref-a.png=Subject, samples/bytedance_seedance_v2/style-ref.png=Style]
featured: true
weight: 74
---
**Reference-to-video** takes one or more stills and carries their subject/style into motion. Assign each reference a role ("reference 1 = subject, reference 2 = style") and name the invariants ("consistent face and wardrobe") so identity survives the animation.
