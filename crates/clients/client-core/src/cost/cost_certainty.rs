/// How binding a priced amount is to the final bill.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CostCertainty {
    /// The request params fully determine the charge.
    Exact,
    /// Good-faith estimate — the provider's reported usage is authoritative
    /// (e.g. thinking tokens billed on top of a per-image rate).
    Estimated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_a_plain_comparable_flag() {
        assert_eq!(CostCertainty::Exact, CostCertainty::Exact);
        assert_ne!(CostCertainty::Exact, CostCertainty::Estimated);
    }
}
