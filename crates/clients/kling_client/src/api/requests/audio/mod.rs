//! Audio generation bindings (`/v1/audio/*`) — legacy-style async create →
//! poll, statuses spelled `succeed`.

pub mod get_text_to_audio_task;
pub mod get_video_to_audio_task;
pub mod list_text_to_audio_tasks;
pub mod list_video_to_audio_tasks;
pub mod text_to_audio;
pub mod video_to_audio;
