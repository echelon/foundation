---
id: dialogue-cafe-scene
model: bytedance_seedance_v2
task: text_to_video
title: Two friends talking in a sunlit café
techniques: [dialogue, cinematic-lighting, shallow-depth]
aspect: "16:9"
prompt: "Two friends laughing over coffee at a sunlit café window, warm morning light, gentle handheld motion, shallow depth of field, natural skin tones, subtle rack focus between them, cinematic."
params:
  duration: 5
  resolution: "1080p"
  camera: "static"
  aspect_ratio: "16:9"
  seed: 30012
asset: samples/bytedance_seedance_v2/dialogue-cafe-scene.mp4
poster: samples/bytedance_seedance_v2/dialogue-cafe-scene.poster.jpg
job_id: job_01JABCCAFE
featured: true
weight: 100
---
Dialogue scenes live and die on **motion restraint**: "gentle handheld" plus "subtle rack focus" keeps it human without the model inventing chaotic camera moves. Ask for "natural skin tones" to avoid the waxy look t2v models drift toward.
