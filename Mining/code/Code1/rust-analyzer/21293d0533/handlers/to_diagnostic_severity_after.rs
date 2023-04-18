fn to_diagnostic_severity(severity: Severity) -> DiagnosticSeverity {
    use ra_analysis::Severity::*;

    match severity {
        Error => DiagnosticSeverity::Error,
        WeakWarning => DiagnosticSeverity::Hint,
    }
}
