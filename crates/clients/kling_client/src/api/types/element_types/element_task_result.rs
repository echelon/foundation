use serde::Deserialize;

use crate::api::types::element_types::element_record::ElementRecord;

/// The `task_result` payload of element management tasks.
#[derive(Clone, Debug, Deserialize)]
pub struct ElementTaskResult {
  #[serde(default)]
  pub elements: Vec<ElementRecord>,
}
