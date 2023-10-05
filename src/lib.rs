//! Rust client interface for the Debian Bug Tracking System (Debbugs)
//!
//! # Examples
//! ```no_run
//! use debbugs::blocking::Debbugs;
//! let debbugs = Debbugs::default();
//! println!("{:?}", debbugs.newest_bugs(10).unwrap());
//!```
//!
mod soap;
pub use soap::{BugLog, BugReport};

const DEFAULT_URL: &str = "https://bugs.debian.org/cgi-bin/soap.cgi";

#[derive(Debug)]
pub enum Error {
    SoapError(String),
    XmlError(String),
    ReqwestError(reqwest::Error),
    Fault(soap::Fault),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ReqwestError(err)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BugStatus {
    Done,
    Forwarded,
    Open,
}

impl std::str::FromStr for BugStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "done" => Ok(BugStatus::Done),
            "forwarded" => Ok(BugStatus::Forwarded),
            "open" => Ok(BugStatus::Open),
            _ => Err(Error::SoapError(format!("Unknown status: {}", s))),
        }
    }
}

impl ToString for BugStatus {
    fn to_string(&self) -> String {
        match self {
            BugStatus::Done => "done".to_string(),
            BugStatus::Forwarded => "forwarded".to_string(),
            BugStatus::Open => "open".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub enum Archived {
    Archived,
    #[default]
    NotArchived,
    Both,
}

impl std::str::FromStr for Archived {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "archived" => Ok(Archived::Archived),
            "0" | "unarchived" => Ok(Archived::NotArchived),
            "both" => Ok(Archived::Both),
            _ => Err(Error::SoapError(format!("Unknown archived: {}", s))),
        }
    }
}

impl ToString for Archived {
    fn to_string(&self) -> String {
        match self {
            Archived::Archived => "1".to_string(),
            Archived::NotArchived => "0".to_string(),
            Archived::Both => "both".to_string(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            Error::SoapError(err) => write!(f, "SOAP Error: {}", err),
            Error::XmlError(err) => write!(f, "XML Error: {}", err),
            Error::ReqwestError(err) => write!(f, "Reqwest Error: {}", err),
            Error::Fault(err) => write!(f, "Fault: {}", err),
        }
    }
}

impl std::error::Error for Error {}

pub type SoapResponse = Result<(reqwest::StatusCode, String), Error>;

type BugId = i32;

pub use soap::SearchQuery;

#[cfg(feature = "blocking")]
pub mod blocking;

mod r#async;

pub use r#async::Debbugs;
