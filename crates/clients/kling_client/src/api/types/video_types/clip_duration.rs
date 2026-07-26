/// Output duration for the models that only offer 5s or 10s clips
/// (Kling 2.6 and 2.5 Turbo). The 3.0-family models take any integer in
/// their range instead and use a plain validated field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipDuration {
  S5,
  S10,
}

impl ClipDuration {
  pub fn seconds(&self) -> u32 {
    match self {
      Self::S5 => 5,
      Self::S10 => 10,
    }
  }
}
