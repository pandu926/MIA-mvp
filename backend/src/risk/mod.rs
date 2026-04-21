pub mod scorer;
pub mod signals;

#[cfg(test)]
pub use scorer::{categorize_score, RiskCategory};
pub use scorer::{compute_composite_score, RiskSignals};
