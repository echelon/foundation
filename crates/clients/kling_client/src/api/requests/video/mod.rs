//! Video generation bindings — one module per model × mode, matching Kling's
//! model-version-in-path endpoints. All are async create → poll (see
//! [`crate::api::requests::tasks`]).

pub(crate) mod common;

pub mod image_to_video_kling_2_5_turbo;
pub mod image_to_video_kling_2_6;
pub mod image_to_video_kling_3_0;
pub mod image_to_video_kling_3_0_turbo;
pub mod motion_control_kling_2_6;
pub mod motion_control_kling_3_0;
pub mod omni_video_kling_3_0_omni;
pub mod omni_video_kling_o1;
pub mod text_to_video_kling_2_5_turbo;
pub mod text_to_video_kling_2_6;
pub mod text_to_video_kling_3_0;
pub mod text_to_video_kling_3_0_turbo;
