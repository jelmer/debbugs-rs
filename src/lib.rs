//! Rust client interface for the Debian Bug Tracking System (Debbugs)
//!
//! # Examples
//! ```rust
//! use debbugs::blocking::Debbugs;
//! let debbugs = Debbugs::default();
//! println!("{:?}", debbugs.newest_bugs(10).unwrap());
//!```
//!
mod soap;
use log::debug;
pub use soap::{BugLog, BugReport};

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

impl Debbugs {
    async fn send_soap_request(&self, request: &xmltree::Element, action: &str) -> SoapResponse {
        let mut body = Vec::new();
        request.write(&mut body).expect("failed to generate xml");
        debug!("SOAP Request: {}", String::from_utf8_lossy(body.as_slice()));
        let req = self
            .client
            .post(&self.url)
            .body(body)
            .header("Content-Type", "text/xml")
            .header("Soapaction", action);
        let res = req.send().await?;
        let status = res.status();
        if status.is_client_error() || status.is_server_error() {
            let txt = res.text().await.unwrap();
            debug!("SOAP Response: {}", txt);
            let fault = soap::parse_fault(&txt).map_err(Error::XmlError)?;
            return Err(Error::Fault(fault));
        }
        debug!("SOAP Status: {}", status);
        let txt = res.text().await.unwrap_or_default();
        debug!("SOAP Response: {}", txt);
        Ok((status, txt))
    }
}

impl Default for Debbugs {
    fn default() -> Self {
        Self::new("https://debbugs.gnu.org/cgi/soap.cgi")
    }
}

impl Debbugs {
    pub fn new(url: &str) -> Self {
        Debbugs {
            client: reqwest::Client::new(),
            url: url.to_string(),
        }
    }
}

pub struct Debbugs {
    client: reqwest::Client,
    url: String,
}

impl Debbugs {
    pub async fn newest_bugs(&self, amount: i32) -> Result<Vec<i32>, Error> {
        let request = soap::newest_bugs_request(amount);
        let (_status, response) = self.send_soap_request(&request, "Debbugs/SOAP").await?;

        soap::parse_newest_bugs_response(&response).map_err(Error::XmlError)
    }

    pub async fn get_bug_log(&self, bug_id: i32) -> Result<Vec<BugLog>, Error> {
        let request = soap::get_bug_log_request(bug_id);
        let (_status, response) = self.send_soap_request(&request, "Debbugs/SOAP").await?;

        soap::parse_get_bug_log_response(&response).map_err(Error::XmlError)
    }
}

#[cfg(feature = "blocking")]
pub mod blocking;
