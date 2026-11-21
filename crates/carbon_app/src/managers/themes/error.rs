#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error(
        "This theme needs a newer version of GDLauncher (it uses theme API {required}, this version supports {supported})"
    )]
    TooNew { required: u64, supported: u32 },
    #[error("Theme \"{0}\" isn't installed")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
}

impl crate::error::FeErrorCode for ThemeError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::TooNew { .. } => "THEME_TOO_NEW",
            Self::NotFound(_) => "THEME_NOT_FOUND",
            Self::Invalid(_) => "THEME_INVALID",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeError;
    use crate::error::FeErrorCode;

    #[test]
    fn codes_match_the_variants() {
        assert_eq!(
            ThemeError::TooNew {
                required: 2,
                supported: 1
            }
            .error_code(),
            "THEME_TOO_NEW"
        );
        assert_eq!(
            ThemeError::NotFound("a".into()).error_code(),
            "THEME_NOT_FOUND"
        );
        assert_eq!(
            ThemeError::Invalid("a".into()).error_code(),
            "THEME_INVALID"
        );
    }
}
