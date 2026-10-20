//! The mutant check every toolset crate runs: a build of the implementation with one property
//! switched off must fail at least one scenario, or the suite is not evidence for that property.

use crate::conformance::Report;

/// Checks that every named mutant's report has at least one failed scenario.
///
/// # Errors
///
/// The names of the mutants that passed every scenario, in the order given.
pub fn each_mutant_fails(report_per_mutant: &[(&str, Report)]) -> Result<(), Vec<String>> {
    let survivors: Vec<String> = report_per_mutant
        .iter()
        .filter(|(_, report)| report.failed().next().is_none())
        .map(|(name, _)| (*name).to_string())
        .collect();
    if survivors.is_empty() {
        Ok(())
    } else {
        Err(survivors)
    }
}
