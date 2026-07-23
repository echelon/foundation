//! The ONE rounding rule for all money math.

/// Round-half-up to the nearest integer. Every charge in the system rounds
/// through this single function so no two code paths can disagree by a micro.
pub(crate) fn round_half_up(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_round_up_and_sign_behaves() {
        assert_eq!(round_half_up(2.5), 3);
        assert_eq!(round_half_up(2.4999), 2);
        assert_eq!(round_half_up(0.0), 0);
        assert_eq!(round_half_up(-1.5), -1); // half-up, not half-away-from-zero
    }
}
