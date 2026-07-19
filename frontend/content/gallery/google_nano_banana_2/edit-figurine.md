---
id: edit-figurine
model: google_nano_banana_2
task: image_edit
category: Editing
title: Turn a photo into a collectible figurine
techniques: [reference-consistency, product, studio-lighting]
aspect: "1:1"
prompt: "Turn the person in the reference photo into a cute collectible vinyl figurine on a round base, studio product lighting, soft shadows, keep the face and outfit recognizable, glossy plastic material, blurred toy-shelf background."
params:
  aspect_ratio: "1:1"
  guidance: 5
  seed: 20114
asset: samples/google_nano_banana_2/edit-figurine-out.png
references: [samples/google_nano_banana_2/edit-figurine-in.png=Input photo]
featured: true
weight: 74
---
The classic Nano Banana edit. The trick is telling it **what to preserve** ("keep the face and outfit recognizable") and **what material to swap to** ("glossy plastic"). Edits work best when the instruction names the target explicitly rather than describing a whole new scene.
