

/// Diagnostic level used for sorting and display.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DiagnosticLevel {
    Error,
    Warning,
    Help,
}

/// A structured diagnostic message.
#[derive(Debug, Clone)]
struct Diagnostic {
    level: DiagnosticLevel,
    message: String,
    // You can also store file, line, column, etc. if you parse those out.
}