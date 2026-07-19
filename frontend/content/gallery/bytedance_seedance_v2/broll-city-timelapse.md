---
id: broll-city-timelapse
model: bytedance_seedance_v2
task: text_to_video
title: City skyline timelapse, dusk to night
techniques: [timelapse, wide-angle, long-exposure]
aspect: "16:9"
prompt: "Timelapse of a city skyline transitioning from dusk to night, clouds streaking overhead, office lights flickering on window by window, car light trails on the avenues, long-exposure look, wide establishing shot."
params:
  duration: 5
  resolution: "1080p"
  camera: "static"
  aspect_ratio: "16:9"
  seed: 9004
asset: samples/bytedance_seedance_v2/broll-city-timelapse.mp4
poster: samples/bytedance_seedance_v2/broll-city-timelapse.poster.jpg
job_id: job_01JABCCITY
featured: false
weight: 82
---
Timelapse cues the model to compress time: "clouds streaking", "lights flickering on window by window", "car light trails". Keep the camera **static** so the time-compression is the only motion — b-roll wants to sit under a voiceover.
