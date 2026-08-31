/// Error separating "bad input" (CLI exit 2) from "verification failed"
/// (CLI exit 1), so the CLI exit codes and the long-lived MCP server stay
/// correct without `process::exit` scattered through the logic (spec 041 §3.3).
#[derive(Debug)]
pub enum VerifyDeriveError {
    /// Invalid input — maps to CLI exit code 2.
    InvalidInput(String),
    /// Verification failed — maps to CLI exit code 1.
    VerificationFailed(String),
    /// Internal/unexpected — maps to CLI exit code 1.
    Internal(String),
}

impl std::fmt::Display for VerifyDeriveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyDeriveError::InvalidInput(m)
            | VerifyDeriveError::VerificationFailed(m)
            | VerifyDeriveError::Internal(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for VerifyDeriveError {}

impl From<String> for VerifyDeriveError {
    fn from(s: String) -> Self {
        VerifyDeriveError::Internal(s)
    }
}

impl VerifyDeriveError {
    /// The CLI exit code this error maps to.
    pub fn exit_code(&self) -> i32 {
        match self {
            VerifyDeriveError::InvalidInput(_) => 2,
            VerifyDeriveError::VerificationFailed(_) | VerifyDeriveError::Internal(_) => 1,
        }
    }

    /// Prints the error exactly as the CLI always has: invalid input and
    /// internal errors as `Error: <message>`, verification failures with the
    /// operation, expression and retry hint.
    pub fn print_cli(&self, operation: &str, expression: Option<&str>) {
        match self {
            VerifyDeriveError::VerificationFailed(message) => eprintln!(
                "Error: verification failed for {operation} \"{}\"\n  {message}\n  hint: retry with the correct result",
                expression.unwrap_or("")
            ),
            other => eprintln!("Error: {other}"),
        }
    }
}
