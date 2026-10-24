//! Errors returned to the UI. `code` is stable and translated by the frontend; `message` is a
//! technical English fallback that also goes to the log.

use randommac_net::NetError;
use randommac_net::ops::OpError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("INVALID_INPUT", message)
    }

    pub fn busy() -> Self {
        Self::new("BUSY", "another MAC change is in progress")
    }

    pub fn read_only() -> Self {
        Self::new(
            "NOT_ELEVATED",
            "RandomMac is running without administrator rights",
        )
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<NetError> for ApiError {
    fn from(e: NetError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<OpError> for ApiError {
    fn from(e: OpError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        Self::new("IO_ERROR", e.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("PARSE_ERROR", e.to_string())
    }
}

impl From<tauri::Error> for ApiError {
    fn from(e: tauri::Error) -> Self {
        Self::new("INTERNAL", e.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self::new("INTERNAL", format!("{e:#}"))
    }
}

impl From<tauri_plugin_updater::Error> for ApiError {
    fn from(e: tauri_plugin_updater::Error) -> Self {
        Self::new("UPDATE_FAILED", e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_use_the_codes_the_ui_translates() {
        let e = ApiError::invalid("bad pattern");
        assert_eq!(
            (e.code.as_str(), e.message.as_str()),
            ("INVALID_INPUT", "bad pattern")
        );
        assert_eq!(ApiError::busy().code, "BUSY");
        assert_eq!(ApiError::read_only().code, "NOT_ELEVATED");
        assert_eq!(
            ApiError::new("X", "y").to_string(),
            "X: y",
            "Display is code: message"
        );
    }

    #[test]
    fn conversions_keep_backend_codes() {
        let e: ApiError = NetError::NmNoActiveConnection.into();
        assert_eq!(e.code, "NM_NO_ACTIVE_CONNECTION");
        assert_eq!(e.message, NetError::NmNoActiveConnection.to_string());
        let e: ApiError = OpError::Busy.into();
        assert_eq!(e.code, "BUSY");
        let e: ApiError = std::io::Error::other("disk").into();
        assert_eq!((e.code.as_str(), e.message.as_str()), ("IO_ERROR", "disk"));
        let e: ApiError = serde_json::from_str::<u8>("x").expect_err("invalid").into();
        assert_eq!(e.code, "PARSE_ERROR");
        let e: ApiError = anyhow::anyhow!("inner").context("outer").into();
        assert_eq!(
            (e.code.as_str(), e.message.as_str()),
            ("INTERNAL", "outer: inner")
        );
        let e: ApiError = tauri::Error::WindowNotFound.into();
        assert_eq!(e.code, "INTERNAL");
        let e: ApiError = tauri_plugin_updater::Error::ReleaseNotFound.into();
        assert_eq!(e.code, "UPDATE_FAILED");
    }

    #[test]
    fn serializes_camel_case() {
        let json = serde_json::to_value(ApiError::new("BUSY", "x")).expect("json");
        assert_eq!(json, serde_json::json!({ "code": "BUSY", "message": "x" }));
    }
}
