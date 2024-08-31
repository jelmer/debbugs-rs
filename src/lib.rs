//! Rust client interface for the Debian Bug Tracking System (Debbugs)
//!
//! # Examples
//! ```no_run
//! use debbugs::blocking::Debbugs;
//! let debbugs = Debbugs::default();
//! println!("{:?}", debbugs.newest_bugs(10).unwrap());
//!```
//!
//! See https://wiki.debian.org/DebbugsSoapInterface for more information on the Debbugs SOAP
//! interface.
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

impl std::fmt::Display for BugStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BugStatus::Done => f.write_str("done"),
            BugStatus::Forwarded => f.write_str("forwarded"),
            BugStatus::Open => f.write_str("open"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Pending {
    Pending,
    PendingFixed,
    Fixed,
    Done,
    Forwarded,
}

impl std::str::FromStr for Pending {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Pending::Pending),
            "pending-fixed" => Ok(Pending::PendingFixed),
            "fixed" => Ok(Pending::Fixed),
            "done" => Ok(Pending::Done),
            "forwarded" => Ok(Pending::Forwarded),
            _ => Err(Error::SoapError(format!("Unknown pending: {}", s))),
        }
    }
}

impl std::fmt::Display for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Pending::Pending => f.write_str("pending"),
            Pending::PendingFixed => f.write_str("pending-fixed"),
            Pending::Done => f.write_str("done"),
            Pending::Forwarded => f.write_str("forwarded"),
            Pending::Fixed => f.write_str("fixed"),
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

impl std::fmt::Display for Archived {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Archived::Archived => f.write_str("archived"),
            Archived::NotArchived => f.write_str("unarchived"),
            Archived::Both => f.write_str("both"),
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

/// A bug ID
type BugId = i32;

pub use soap::SearchQuery;

#[cfg(feature = "blocking")]
pub mod blocking;

mod r#async;

pub use r#async::Debbugs;
