use std::ffi::OsString;
use std::path::StripPrefixError;
use subprocess::PopenError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FixMyLibErrors {
    #[error("Error starting DB: {0}")]
    DbInit(String),

    #[error("Error starting subprocess: {0}")]
    OpenSubprocess(String),

    #[error("Failure parsing path: {0}")]
    PathParsing(String),
}

impl From<rusqlite::Error> for FixMyLibErrors {
    fn from(e: rusqlite::Error) -> Self {
        FixMyLibErrors::DbInit(e.to_string())
    }
}
impl From<StripPrefixError> for FixMyLibErrors {
    fn from(e: StripPrefixError) -> Self {
        FixMyLibErrors::PathParsing(e.to_string())
    }
}

impl From<OsString> for FixMyLibErrors {
    fn from(e: OsString) -> Self {
        FixMyLibErrors::PathParsing(e.into_string().unwrap())
    }
}

impl From<std::io::Error> for FixMyLibErrors {
    fn from(e: std::io::Error) -> Self {
        FixMyLibErrors::OpenSubprocess(e.to_string())
    }
}

impl From<PopenError> for FixMyLibErrors {
    fn from(e: PopenError) -> Self {
        FixMyLibErrors::OpenSubprocess(e.to_string())
    }
}

impl From<walkdir::Error> for FixMyLibErrors {
    fn from(e: walkdir::Error) -> Self {
        FixMyLibErrors::PathParsing(e.to_string())
    }
}
