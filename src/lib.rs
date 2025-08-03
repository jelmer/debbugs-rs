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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_bug_status_from_str() {
        assert_eq!(BugStatus::from_str("done").unwrap(), BugStatus::Done);
        assert_eq!(
            BugStatus::from_str("forwarded").unwrap(),
            BugStatus::Forwarded
        );
        assert_eq!(BugStatus::from_str("open").unwrap(), BugStatus::Open);
    }

    #[test]
    fn test_bug_status_from_str_invalid() {
        assert!(BugStatus::from_str("invalid").is_err());
        assert!(BugStatus::from_str("").is_err());
        assert!(BugStatus::from_str("DONE").is_err());
        assert!(BugStatus::from_str("Done").is_err());
    }

    #[test]
    fn test_bug_status_display() {
        assert_eq!(BugStatus::Done.to_string(), "done");
        assert_eq!(BugStatus::Forwarded.to_string(), "forwarded");
        assert_eq!(BugStatus::Open.to_string(), "open");
    }

    #[test]
    fn test_bug_status_roundtrip() {
        let statuses = vec![BugStatus::Done, BugStatus::Forwarded, BugStatus::Open];
        for status in statuses {
            let s = status.to_string();
            let parsed = BugStatus::from_str(&s).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_pending_from_str() {
        assert_eq!(Pending::from_str("pending").unwrap(), Pending::Pending);
        assert_eq!(
            Pending::from_str("pending-fixed").unwrap(),
            Pending::PendingFixed
        );
        assert_eq!(Pending::from_str("fixed").unwrap(), Pending::Fixed);
        assert_eq!(Pending::from_str("done").unwrap(), Pending::Done);
        assert_eq!(Pending::from_str("forwarded").unwrap(), Pending::Forwarded);
    }

    #[test]
    fn test_pending_from_str_invalid() {
        assert!(Pending::from_str("invalid").is_err());
        assert!(Pending::from_str("").is_err());
        assert!(Pending::from_str("PENDING").is_err());
        assert!(Pending::from_str("pending_fixed").is_err());
    }

    #[test]
    fn test_pending_display() {
        assert_eq!(Pending::Pending.to_string(), "pending");
        assert_eq!(Pending::PendingFixed.to_string(), "pending-fixed");
        assert_eq!(Pending::Fixed.to_string(), "fixed");
        assert_eq!(Pending::Done.to_string(), "done");
        assert_eq!(Pending::Forwarded.to_string(), "forwarded");
    }

    #[test]
    fn test_pending_roundtrip() {
        let pendings = vec![
            Pending::Pending,
            Pending::PendingFixed,
            Pending::Fixed,
            Pending::Done,
            Pending::Forwarded,
        ];
        for pending in pendings {
            let s = pending.to_string();
            let parsed = Pending::from_str(&s).unwrap();
            assert_eq!(pending, parsed);
        }
    }

    #[test]
    fn test_archived_from_str() {
        assert_eq!(Archived::from_str("1").unwrap(), Archived::Archived);
        assert_eq!(Archived::from_str("archived").unwrap(), Archived::Archived);
        assert_eq!(Archived::from_str("0").unwrap(), Archived::NotArchived);
        assert_eq!(
            Archived::from_str("unarchived").unwrap(),
            Archived::NotArchived
        );
        assert_eq!(Archived::from_str("both").unwrap(), Archived::Both);
    }

    #[test]
    fn test_archived_from_str_invalid() {
        assert!(Archived::from_str("invalid").is_err());
        assert!(Archived::from_str("").is_err());
        assert!(Archived::from_str("2").is_err());
        assert!(Archived::from_str("ARCHIVED").is_err());
        assert!(Archived::from_str("not-archived").is_err());
    }

    #[test]
    fn test_archived_display() {
        assert_eq!(Archived::Archived.to_string(), "archived");
        assert_eq!(Archived::NotArchived.to_string(), "unarchived");
        assert_eq!(Archived::Both.to_string(), "both");
    }

    #[test]
    fn test_archived_roundtrip() {
        let archiveds = vec![Archived::Archived, Archived::NotArchived, Archived::Both];
        for archived in archiveds {
            let s = archived.to_string();
            // Note: from_str accepts both numeric and string forms, but display uses string form
            let parsed = Archived::from_str(&s).unwrap();
            assert_eq!(archived, parsed);
        }
    }

    #[test]
    fn test_archived_default() {
        assert_eq!(Archived::default(), Archived::NotArchived);
    }

    #[test]
    fn test_error_display() {
        let soap_err = Error::SoapError("test error".to_string());
        assert_eq!(soap_err.to_string(), "SOAP Error: test error");

        let xml_err = Error::XmlError("xml parse failed".to_string());
        assert_eq!(xml_err.to_string(), "XML Error: xml parse failed");

        // We can't easily create a real reqwest::Error in tests, so we'll skip testing
        // the exact error message format for ReqwestError

        let fault = soap::Fault {
            faultcode: "Client".to_string(),
            faultstring: "Invalid request".to_string(),
            faultactor: None,
            detail: Some("Missing required parameter".to_string()),
        };
        let fault_err = Error::Fault(fault);
        assert_eq!(fault_err.to_string(), "Fault: { faultcode: Client, faultstring: Invalid request, faultactor: None, detail: Some(\"Missing required parameter\") }");
    }

    #[test]
    fn test_error_conversions() {
        // We can't easily create a real reqwest::Error in tests without an actual HTTP failure
        // The From<reqwest::Error> trait is trivial and doesn't need extensive testing
    }
}
