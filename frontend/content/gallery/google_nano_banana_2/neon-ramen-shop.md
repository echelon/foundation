---
id: neon-ramen-shop
model: google_nano_banana_2
task: text_to_image
category: Photorealistic
title: Neon-lit ramen shop at night
techniques: [neon, cinematic-lighting, photorealistic, shallow-depth]
aspect: "16:9"
prompt: "A tiny neon-lit ramen shop on a rainy Tokyo backstreet, steam rising from the counter, warm interior glow spilling onto wet asphalt, reflections of pink and cyan signage, cinematic 35mm, shallow depth of field, night."
params:
  aspect_ratio: "16:9"
  guidance: 4.5
  seed: 128773
asset: samples/google_nano_banana_2/neon-ramen-shop.png
featured: true
weight: 100
---
Neon reads as neon because the prompt names **specific light sources** ("neon", "reflections on wet asphalt") instead of "a colorful scene". Drop *shallow depth of field* and the background sharpens; push guidance past ~7 and reflections over-saturate.
