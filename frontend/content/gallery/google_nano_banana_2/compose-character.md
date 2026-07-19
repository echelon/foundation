---
id: compose-character
model: google_nano_banana_2
task: image_edit
category: Multi-reference composition
title: Keep a character consistent across a new pose
techniques: [reference-consistency, character, cinematic-lighting]
aspect: "3:4"
prompt: "Using the character in reference 1 and the outfit in reference 2, generate the same character in a new three-quarter pose looking over the shoulder, consistent face and hairstyle, cinematic rim light, neutral studio backdrop."
params:
  aspect_ratio: "3:4"
  guidance: 5.5
  seed: 20550
asset: samples/google_nano_banana_2/compose-character-out.png
references: [samples/google_nano_banana_2/char-ref-a.png=Reference 1 (face), samples/google_nano_banana_2/char-ref-b.png=Reference 2 (outfit)]
featured: false
weight: 66
---
Character consistency across poses is the hardest ask — split the references by role ("reference 1 = face, reference 2 = outfit") and name the invariants ("consistent face and hairstyle") so the model knows what must not drift.
