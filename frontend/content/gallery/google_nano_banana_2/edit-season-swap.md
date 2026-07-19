---
id: edit-season-swap
model: google_nano_banana_2
task: image_edit
category: Editing
title: Change summer to winter
techniques: [relight, background-swap, photorealistic]
aspect: "16:9"
prompt: "Change this summer landscape to deep winter: cover the ground and trees in fresh snow, cool blue overcast light, breath-fog in the air, keep the composition and camera angle identical."
params:
  aspect_ratio: "16:9"
  guidance: 4.5
  seed: 20220
asset: samples/google_nano_banana_2/edit-season-out.png
references: [samples/google_nano_banana_2/edit-season-in.png=Input photo]
featured: true
weight: 72
---
Season and time-of-day swaps are Nano Banana's sweet spot. Anchor it with "keep the composition and camera angle identical" so it edits the scene instead of regenerating a different one.
