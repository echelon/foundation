---
id: compose-product-in-scene
model: google_nano_banana_2
task: image_edit
category: Multi-reference composition
title: Place a product into a new scene
techniques: [reference-consistency, product, studio-lighting]
aspect: "1:1"
prompt: "Place the serum bottle from the first reference onto the marble bathroom counter from the second reference, match the scene's soft morning light, add a soft contact shadow, keep the bottle's label and proportions exact."
params:
  aspect_ratio: "1:1"
  guidance: 5
  seed: 20440
asset: samples/google_nano_banana_2/compose-product-out.png
references: [samples/google_nano_banana_2/product-serum-bottle.png=Product, samples/google_nano_banana_2/scene-ref.png=Scene]
featured: true
weight: 68
---
Multi-reference composition is where Nano Banana shines for commerce. Refer to inputs by position ("the first reference", "the second reference") and demand fidelity on the hero object ("keep the label and proportions exact") while letting the model relight it into the new scene.
