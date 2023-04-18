fn to_diagnostic_severity(severity: Severity) -> DiagnosticSeverity {
    use ra_analysis::Severity::*;

    match severity {
        Error => DiagnosticSeverity::Error,
        Warning => DiagnosticSeverity::Warning,
        Information => DiagnosticSeverity::Information,
        Hint => DiagnosticSeverity::Hint,
    }
}
