mod current;
mod insert;
mod lock_for_admission;
mod lock_for_update;

pub use current::current;
pub use insert::insert;
pub use lock_for_admission::lock_for_admission;
pub use lock_for_update::lock_for_update;

pub struct PolicyRow {
    pub revision: i32,
    pub policy: serde_json::Value,
    pub project_id: Option<String>,
}
