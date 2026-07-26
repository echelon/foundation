//! Image generation bindings. Two legacy-style endpoint families:
//!
//! - `/v1/images/generations` — Kling Image 3.0 (`model_name: kling-v3`),
//!   queried via [`get_image_task`] / [`list_image_tasks`].
//! - `/v1/images/omni-image` — Kling Image 3.0 Omni (`kling-v3-omni`) and
//!   Kling Image O1 (`kling-image-o1`), queried via [`get_omni_image_task`] /
//!   [`list_omni_image_tasks`].
//!
//! All are async create → poll with the legacy `succeed` vocabulary.

pub(crate) mod common;

pub mod generate_image_kling_3_0;
pub mod generate_image_kling_o1;
pub mod get_image_task;
pub mod get_omni_image_task;
pub mod list_image_tasks;
pub mod list_omni_image_tasks;
pub mod omni_image_kling_3_0_omni;
