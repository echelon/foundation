---
id: edit-relight
model: google_nano_banana_2
task: image_edit
category: Editing
title: Relight a flat portrait into dramatic Rembrandt lighting
techniques: [relight, low-key, portrait]
aspect: "1:1"
prompt: "Relight this evenly-lit portrait with dramatic low-key Rembrandt lighting from the left, deep shadows on the right cheek, warm key light, keep the subject, pose, and background unchanged."
params:
  aspect_ratio: "1:1"
  guidance: 4
  seed: 20330
asset: samples/google_nano_banana_2/edit-relight-out.png
references: [samples/google_nano_banana_2/edit-relight-in.png=Input photo]
featured: false
weight: 70
---
Relighting is an edit, not a regeneration — so pin everything you *don't* want touched ("keep the subject, pose, and background unchanged") and describe only the light direction and quality.
