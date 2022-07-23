use crate::utils::IntoStr;

use std::fmt::{self, Display, Formatter, Debug};
use std::error::Error as StdError;
use std::io;

use thin_str::ThinStr;

pub type Result<T> = std::result::Result<T, PixivError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErrorCode(u16);

impl From<u16> for ErrorCode {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} code {}", self.into_str(), self.0)
    }
}

macro_rules! error_codes {
    ($(
        ($num:expr, $konst:ident);
    )+) => {
        impl ErrorCode {$(
            pub const $konst: ErrorCode = ErrorCode($num);
        )+}

        impl From<ErrorCode> for &str {
            fn from(value: ErrorCode) -> Self {
                match value.0 {
                $(
                    $num => casey::lower!(stringify!($konst)),
                )+
                    _ => "unknown"
                }
            }
        }
    }
}

error_codes! {
    /* HTTP error codes */
    (400, BAD_REQUEST);
    (403, FORBIDDEN);
    (404, NOT_FOUND);

    /* OAuth error codes */
    (918,  INVALID_REQUEST);
    (1508, INVALID_GRANT);
}

macro_rules! error_wrapper {
    ($name:ident {
        $($var:ident($err:path),)+
    }) => {
        #[derive(Debug)]
        pub enum $name {$(
            $var($err),
        )+}

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {$(
                    Self::$var(e) => write!(f, "{e}"),
                )+}
            }
        }

        impl StdError for $name {
            fn source(&self) -> Option<&(dyn StdError + 'static)> {
                match self {$(
                    Self::$var(e) => e.source(),
                )+}
            }
        }

    $(
        impl From<$err> for PixivError {
            fn from(e: $err) -> Self {
                Self::$var(e)
            }
        }
    )+
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind {
    OAuth,
    AAPI,
}

impl From<Kind> for &str {
    fn from(value: Kind) -> Self {
        match value {
            Kind::OAuth => "OAuth",
            Kind::AAPI => "App-API",
        }
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub(crate) kind: Kind,
    pub(crate) code: ErrorCode,
    pub(crate) msg:  ThinStr,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} error: {} ({})", self.kind.into_str(), self.msg, self.code)
    }
}

impl StdError for ApiError {}

error_wrapper!(PixivError {
    Request(reqwest::Error),
    Decode(serde_json::Error),
    Io(io::Error),
    API(ApiError),
});
