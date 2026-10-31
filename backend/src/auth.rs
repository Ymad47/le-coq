//! API key authentication.
//!
//! The backend compares the key sent by the client against a secret
//! configured at startup (`LE_COQ_API_KEY`). The comparison is constant-time
//! so response timing does not reveal how many leading bytes matched.

use std::fmt;
use std::net::SocketAddr;

use subtle::ConstantTimeEq;

/// Minimum accepted key length. Generate one with `openssl rand -hex 32`.
pub const MIN_API_KEY_LEN: usize = 32;

/// Header carrying the key. Fixed on purpose: the Vite proxy, the nginx
/// template and the Zig CLI all send this exact name.
pub const API_KEY_HEADER: &str = "X-API-Key";

/// Secret API key. `Debug` is redacted so the key never ends up in logs.
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(key: String) -> Result<Self, AuthConfigError> {
        if key.len() < MIN_API_KEY_LEN {
            return Err(AuthConfigError::KeyTooShort);
        }
        Ok(Self(key))
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(<redacted>)")
    }
}

#[derive(Clone, Debug)]
pub enum AuthMode {
    /// Requests must carry exactly this key.
    ApiKey(ApiKey),
    /// No check at all. Only allowed when bound to a loopback address.
    Disabled,
}

impl AuthMode {
    /// Builds the auth mode from raw settings.
    ///
    /// - `disable_auth`: value of `DISABLE_AUTH` (`"true"` disables auth)
    /// - `api_key`: value of `LE_COQ_API_KEY`
    ///
    /// A configured key always wins over `DISABLE_AUTH`, so a leftover
    /// `DISABLE_AUTH=true` line can never silently turn auth off.
    pub fn from_settings(
        disable_auth: Option<&str>,
        api_key: Option<String>,
        bind_address: &str,
    ) -> Result<Self, AuthConfigError> {
        let disable_requested = disable_auth == Some("true");

        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            if disable_requested {
                tracing::warn!("DISABLE_AUTH=true ignored because LE_COQ_API_KEY is set");
            }
            return Ok(Self::ApiKey(ApiKey::new(key)?));
        }

        if disable_requested {
            let addr: SocketAddr = bind_address
                .parse()
                .map_err(|_| AuthConfigError::InvalidBindAddress(bind_address.to_owned()))?;
            if !addr.ip().is_loopback() {
                return Err(AuthConfigError::DisabledOnPublicBind(
                    bind_address.to_owned(),
                ));
            }
            return Ok(Self::Disabled);
        }

        Err(AuthConfigError::MissingKey)
    }

    /// Returns true if a request carrying `provided` (raw header bytes) is allowed.
    pub fn allows(&self, provided: Option<&[u8]>) -> bool {
        match self {
            Self::Disabled => true,
            Self::ApiKey(expected) => match provided {
                // ct_eq returns false immediately on length mismatch: this
                // leaks the key length only, which is not secret.
                Some(p) => bool::from(p.ct_eq(expected.0.as_bytes())),
                None => false,
            },
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthConfigError {
    #[error("LE_COQ_API_KEY is not set (generate one with `openssl rand -hex 32`), or set DISABLE_AUTH=true for local dev")]
    MissingKey,
    #[error("LE_COQ_API_KEY must be at least {MIN_API_KEY_LEN} characters")]
    KeyTooShort,
    #[error("DISABLE_AUTH=true is only allowed with a loopback BIND_ADDRESS, got {0}")]
    DisabledOnPublicBind(String),
    #[error("invalid BIND_ADDRESS: {0}")]
    InvalidBindAddress(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef";

    fn key_mode() -> AuthMode {
        AuthMode::from_settings(None, Some(KEY.into()), "0.0.0.0:8000").unwrap()
    }

    #[test]
    fn correct_key_is_allowed() {
        assert!(key_mode().allows(Some(KEY.as_bytes())));
    }

    #[test]
    fn missing_key_is_rejected() {
        assert!(!key_mode().allows(None));
    }

    #[test]
    fn wrong_key_is_rejected() {
        assert!(!key_mode().allows(Some(b"0123456789abcdef0123456789abcdeX")));
        assert!(!key_mode().allows(Some(b"anything")));
        assert!(!key_mode().allows(Some(b"")));
    }

    #[test]
    fn prefix_of_key_is_rejected() {
        assert!(!key_mode().allows(Some(&KEY.as_bytes()[..10])));
    }

    #[test]
    fn startup_fails_without_key() {
        let err = AuthMode::from_settings(None, None, "127.0.0.1:8000").unwrap_err();
        assert_eq!(err, AuthConfigError::MissingKey);
        let err = AuthMode::from_settings(None, Some(String::new()), "127.0.0.1:8000").unwrap_err();
        assert_eq!(err, AuthConfigError::MissingKey);
    }

    #[test]
    fn startup_fails_with_short_key() {
        let err =
            AuthMode::from_settings(None, Some("short".into()), "127.0.0.1:8000").unwrap_err();
        assert_eq!(err, AuthConfigError::KeyTooShort);
    }

    #[test]
    fn disable_auth_allowed_on_loopback() {
        let mode = AuthMode::from_settings(Some("true"), None, "127.0.0.1:8000").unwrap();
        assert!(mode.allows(None));
        assert!(AuthMode::from_settings(Some("true"), None, "[::1]:8000").is_ok());
    }

    #[test]
    fn disable_auth_refused_on_public_bind() {
        let err = AuthMode::from_settings(Some("true"), None, "0.0.0.0:8000").unwrap_err();
        assert!(matches!(err, AuthConfigError::DisabledOnPublicBind(_)));
    }

    #[test]
    fn configured_key_wins_over_disable_auth() {
        let mode =
            AuthMode::from_settings(Some("true"), Some(KEY.into()), "127.0.0.1:8000").unwrap();
        assert!(matches!(mode, AuthMode::ApiKey(_)));
        assert!(!mode.allows(None));
        // With a key set, a public bind is fine even if DISABLE_AUTH is left over.
        let mode = AuthMode::from_settings(Some("true"), Some(KEY.into()), "0.0.0.0:8000").unwrap();
        assert!(matches!(mode, AuthMode::ApiKey(_)));
    }

    #[test]
    fn disable_auth_with_short_key_still_fails() {
        let err = AuthMode::from_settings(Some("true"), Some("short".into()), "127.0.0.1:8000")
            .unwrap_err();
        assert_eq!(err, AuthConfigError::KeyTooShort);
    }

    #[test]
    fn disable_auth_other_values_do_not_disable() {
        let err = AuthMode::from_settings(Some("1"), None, "127.0.0.1:8000").unwrap_err();
        assert_eq!(err, AuthConfigError::MissingKey);
    }

    #[test]
    fn debug_does_not_leak_key() {
        let s = format!("{:?}", key_mode());
        assert!(!s.contains(KEY));
    }
}
