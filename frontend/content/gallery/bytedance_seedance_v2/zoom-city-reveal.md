---
id: zoom-city-reveal
model: bytedance_seedance_v2
task: text_to_video
category: Camera control
title: Dramatic zoom-out city reveal
techniques: [dolly, wide-angle, action]
aspect: "16:9"
prompt: "Start tight on a single lit apartment window, then a smooth continuous zoom-out and pull-back revealing the entire glittering city skyline at night, parallax between buildings, cinematic, one continuous move."
params:
  duration: 5
  resolution: "1080p"
  camera: "zoom"
  aspect_ratio: "16:9"
  seed: 55330
asset: samples/bytedance_seedance_v2/zoom-city-reveal.mp4
poster: samples/bytedance_seedance_v2/zoom-city-reveal.poster.jpg
featured: false
weight: 72
---
Camera moves want a **start and an end** state: "start tight on a window" → "reveal the entire skyline", plus "one continuous move" so the model doesn't cut. Parallax is what makes the pull-back feel three-dimensional.
