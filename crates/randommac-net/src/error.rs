use randommac_core::MacAddress;

/// Errors returned by network backends. [`NetError::code`] gives a stable identifier that the UI
/// maps to a translated message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NetError {
    #[error("adapter not found: {0}")]
    NotFound(String),
    #[error("administrator rights are required to change MAC addresses")]
    NotElevated,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("authentication was cancelled")]
    AuthCancelled,
    #[error("authentication is not available: {0}")]
    AuthUnavailable(String),
    #[error("not supported: {0}")]
    Unsupported(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("NetworkManager is not available: {0}")]
    NmUnavailable(String),
    #[error("the adapter has no active NetworkManager connection")]
    NmNoActiveConnection,
    #[error("the adapter did not come back within the time limit")]
    VerifyTimeout,
    #[error("restoring the previous configuration failed: {0}")]
    RollbackFailed(String),
    #[error("no original address is known for this adapter")]
    NoOriginal,
    #[error("{program} failed (exit code {code:?}): {stderr}")]
    CommandFailed {
        program: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("{0} timed out")]
    Timeout(String),
    #[error("{context} failed (OS error {code})")]
    Os { context: String, code: u32 },
    #[error("{0}")]
    Io(String),
    #[error("the driver ignored {requested}; the adapter still reports {actual:?}")]
    DriverRejected {
        requested: MacAddress,
        actual: Option<MacAddress>,
    },
}

impl NetError {
    pub fn code(&self) -> &'static str {
        match self {
            NetError::NotFound(_) => "ADAPTER_NOT_FOUND",
            NetError::NotElevated => "NOT_ELEVATED",
            NetError::PermissionDenied(_) => "PERMISSION_DENIED",
            NetError::AuthCancelled => "AUTH_CANCELLED",
            NetError::AuthUnavailable(_) => "AUTH_UNAVAILABLE",
            NetError::Unsupported(_) => "UNSUPPORTED",
            NetError::InvalidInput(_) => "INVALID_INPUT",
            NetError::NmUnavailable(_) => "NM_UNAVAILABLE",
            NetError::NmNoActiveConnection => "NM_NO_ACTIVE_CONNECTION",
            NetError::VerifyTimeout => "VERIFY_TIMEOUT",
            NetError::RollbackFailed(_) => "ROLLBACK_FAILED",
            NetError::NoOriginal => "NO_ORIGINAL",
            NetError::CommandFailed { .. } => "COMMAND_FAILED",
            NetError::Timeout(_) => "TIMEOUT",
            NetError::Os { .. } => "OS_ERROR",
            NetError::Io(_) => "IO_ERROR",
            NetError::DriverRejected { .. } => "DRIVER_REJECTED",
        }
    }
}

impl From<std::io::Error> for NetError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            NetError::PermissionDenied(e.to_string())
        } else {
            NetError::Io(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_has_a_stable_code() {
        let mac: MacAddress = "02:11:22:33:44:55".parse().expect("mac");
        let cases = [
            (NetError::NotFound("x".into()), "ADAPTER_NOT_FOUND"),
            (NetError::NotElevated, "NOT_ELEVATED"),
            (NetError::PermissionDenied("x".into()), "PERMISSION_DENIED"),
            (NetError::AuthCancelled, "AUTH_CANCELLED"),
            (NetError::AuthUnavailable("x".into()), "AUTH_UNAVAILABLE"),
            (NetError::Unsupported("x".into()), "UNSUPPORTED"),
            (NetError::InvalidInput("x".into()), "INVALID_INPUT"),
            (NetError::NmUnavailable("x".into()), "NM_UNAVAILABLE"),
            (NetError::NmNoActiveConnection, "NM_NO_ACTIVE_CONNECTION"),
            (NetError::VerifyTimeout, "VERIFY_TIMEOUT"),
            (NetError::RollbackFailed("x".into()), "ROLLBACK_FAILED"),
            (NetError::NoOriginal, "NO_ORIGINAL"),
            (
                NetError::CommandFailed {
                    program: "ip".into(),
                    code: Some(2),
                    stderr: "bad".into(),
                },
                "COMMAND_FAILED",
            ),
            (NetError::Timeout("x".into()), "TIMEOUT"),
            (
                NetError::Os {
                    context: "x".into(),
                    code: 5,
                },
                "OS_ERROR",
            ),
            (NetError::Io("x".into()), "IO_ERROR"),
            (
                NetError::DriverRejected {
                    requested: mac,
                    actual: None,
                },
                "DRIVER_REJECTED",
            ),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code, "{error}");
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn messages_include_the_details() {
        let e = NetError::CommandFailed {
            program: "nmcli".into(),
            code: None,
            stderr: "not running".into(),
        };
        assert_eq!(e.to_string(), "nmcli failed (exit code None): not running");
        let e = NetError::Os {
            context: "opening the driver key".into(),
            code: 5,
        };
        assert_eq!(e.to_string(), "opening the driver key failed (OS error 5)");
    }

    #[test]
    fn io_errors_keep_permission_problems_apart() {
        use std::io::{Error, ErrorKind};
        let denied: NetError = Error::new(ErrorKind::PermissionDenied, "no access").into();
        assert_eq!(denied, NetError::PermissionDenied("no access".into()));
        let other: NetError = Error::new(ErrorKind::NotFound, "missing").into();
        assert_eq!(other, NetError::Io("missing".into()));
    }
}
