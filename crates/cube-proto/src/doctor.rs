//! `doctor.report` payload — SDS §11.1.
//!
//! Carried in `Response.result`. A `DoctorReportPayload` lists the outcome of
//! every check `cubectl doctor` runs.

use serde::{Deserialize, Serialize};

/// Severity of an individual check (SDS §11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    Ok,
    Warn,
    Fail,
}

/// Outcome of a single doctor check (SDS §11.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub level: CheckLevel,
    pub detail: String,
}

/// Full `doctor.report` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoctorReportPayload {
    pub checks: Vec<CheckResult>,
}

impl DoctorReportPayload {
    /// Whether any check failed.
    #[must_use]
    pub fn any_fail(&self) -> bool {
        self.checks.iter().any(|c| c.level == CheckLevel::Fail)
    }
}
