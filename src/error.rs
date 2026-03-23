use std::fmt::{Debug, Display};
use std::num::ParseIntError;

use reqwest::Error as RqError;
use postcard::Error as PostardError;

pub enum Error {
    RqErr(RqError),
    StrErr(&'static str),
    StatusCodeErr(u16),
    ParseIntErr(ParseIntError),
    PostardError(PostardError),
}
impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RqErr(e) => Debug::fmt(e, f),
            Self::StrErr(e) => write!(f, "{:?}", e),
            Self::StatusCodeErr(status) => write!(f, "Satus Code: {status}"),
            Self::ParseIntErr(e) => Debug::fmt(e, f),
            Self::PostardError(e) => Debug::fmt(e, f)
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RqErr(e) => Display::fmt(e, f),
            Self::StrErr(e) => write!(f, "{e}"),
            Self::StatusCodeErr(status) => write!(f, "Satus Code: {status}"),
            Self::ParseIntErr(e) => Display::fmt(e, f),
            Self::PostardError(e) => Display::fmt(e, f),
        }
    }
}
impl std::error::Error for Error {}
impl From<RqError> for Error {
    fn from(value: RqError) -> Self {
        Self::RqErr(value)
    }
}