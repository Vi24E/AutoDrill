/// Explicit theme exception: multiplication-table difficulty is ranked by
/// the logarithm of the answer, not by the reusable arithmetic primitive model.
pub(crate) fn effort(answer: u8) -> f64 {
    if answer == 0 { 0.0 } else { f64::from(answer).log10() }
}
