//! Assemble a shareable diagnostic report. Secrets are stripped before return.

mod report;

pub use report::{collect_report, DiagnosticReport, Probe};
