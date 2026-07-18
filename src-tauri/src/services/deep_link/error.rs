#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeepLinkError {
    #[error("Warm instance arguments do not contain an import URI.")]
    MissingImportArgument,
    #[error("Warm instance arguments contain unsupported values.")]
    UnexpectedImportArguments,
    #[error("Deep link URI exceeds the supported size.")]
    UriTooLong,
    #[error("Deep link URI is invalid.")]
    InvalidUri,
    #[error("Deep link URI authority is invalid.")]
    InvalidUriAuthority,
    #[error("Deep link scheme is not supported.")]
    UnsupportedScheme,
    #[error("Deep link action is not supported.")]
    UnknownAction,
    #[error("Deep link path is not supported.")]
    UnexpectedPath,
    #[error("Deep link fragments are not supported.")]
    FragmentNotAllowed,
    #[error("Deep link source is required.")]
    MissingSource,
    #[error("Deep link source must be provided exactly once.")]
    DuplicateSource,
    #[error("Deep link parameter is not supported.")]
    UnknownParameter,
    #[error("Sensitive deep link parameters are not supported.")]
    SensitiveParameter,
    #[error("Deep link source must be percent encoded.")]
    SourceNotPercentEncoded,
    #[error("Deep link source is invalid.")]
    InvalidSource,
    #[error("Deep link source must use HTTPS.")]
    SourceNotHttps,
    #[error("Deep link source must use github.com.")]
    SourceNotGithub,
    #[error("Deep link source credentials are not supported.")]
    SourceCredentials,
    #[error("Deep link source ports are not supported.")]
    SourcePort,
    #[error("Deep link source parameters are not supported.")]
    SourceParameters,
    #[error("Deep link source contains an unsafe path.")]
    UnsafeSource,
    #[error("Deep link GitHub source is invalid.")]
    InvalidGithubSource,
    #[error("Import intent queue is unavailable.")]
    QueueUnavailable,
    #[error("Import intent event could not be delivered.")]
    EventDelivery,
}

impl DeepLinkError {
    pub fn code(self) -> &'static str {
        match self {
            Self::MissingImportArgument => "missing_import_argument",
            Self::UnexpectedImportArguments => "unexpected_import_arguments",
            Self::UriTooLong => "uri_too_long",
            Self::InvalidUri => "invalid_uri",
            Self::InvalidUriAuthority => "invalid_uri_authority",
            Self::UnsupportedScheme => "unsupported_scheme",
            Self::UnknownAction => "unknown_action",
            Self::UnexpectedPath => "unexpected_path",
            Self::FragmentNotAllowed => "fragment_not_allowed",
            Self::MissingSource => "missing_source",
            Self::DuplicateSource => "duplicate_source",
            Self::UnknownParameter => "unknown_parameter",
            Self::SensitiveParameter => "sensitive_parameter",
            Self::SourceNotPercentEncoded => "source_not_percent_encoded",
            Self::InvalidSource => "invalid_source",
            Self::SourceNotHttps => "source_not_https",
            Self::SourceNotGithub => "source_not_github",
            Self::SourceCredentials => "source_credentials",
            Self::SourcePort => "source_port",
            Self::SourceParameters => "source_parameters",
            Self::UnsafeSource => "unsafe_source",
            Self::InvalidGithubSource => "invalid_github_source",
            Self::QueueUnavailable => "queue_unavailable",
            Self::EventDelivery => "event_delivery",
        }
    }
}
