use log::debug;

use crate::{BugId, Error, SoapResponse};

impl Debbugs {
    fn send_soap_request(&self, request: &xmltree::Element, action: &str) -> SoapResponse {
        let mut body = Vec::new();
        request.write(&mut body).expect("failed to generate xml");
        debug!("SOAP Request: {}", String::from_utf8_lossy(&body));
        let req = self
            .client
            .post(&self.url)
            .body(body)
            .header("Content-Type", "text/xml")
            .header("Soapaction", action);
        let res = req.send()?;
        let status = res.status();
        if status.is_client_error() || status.is_server_error() {
            let txt = res.text().unwrap();
            debug!("SOAP Response: {}", txt);
            let fault = crate::soap::parse_fault(&txt).map_err(Error::XmlError)?;
            return Err(Error::Fault(fault));
        }
        debug!("SOAP Status: {}", status);
        let txt = res.text().unwrap_or_default();
        debug!("SOAP Response: {}", txt);
        Ok((status, txt))
    }
}

impl Default for Debbugs {
    fn default() -> Self {
        Self::new(crate::DEFAULT_URL)
    }
}

impl Debbugs {
    pub fn new(url: &str) -> Self {
        Debbugs {
            client: reqwest::blocking::Client::new(),
            url: url.to_string(),
        }
    }
}
pub struct Debbugs {
    client: reqwest::blocking::Client,
    url: String,
}
impl Debbugs {
    pub fn newest_bugs(&self, amount: i32) -> Result<Vec<BugId>, Error> {
        let request = crate::soap::newest_bugs_request(amount);
        let (_status, response) = self.send_soap_request(&request, "newest_bugs")?;

        crate::soap::parse_newest_bugs_response(&response).map_err(Error::XmlError)
    }

    pub fn get_bug_log(&self, bug_id: BugId) -> Result<Vec<crate::soap::BugLog>, Error> {
        let request = crate::soap::get_bug_log_request(bug_id);
        let (_status, response) = self.send_soap_request(&request, "get_bug_log")?;

        crate::soap::parse_get_bug_log_response(&response).map_err(Error::XmlError)
    }

    pub fn get_bugs(&self, query: &crate::SearchQuery) -> Result<Vec<BugId>, Error> {
        let request = crate::soap::get_bugs_request(query);
        let (_status, response) = self.send_soap_request(&request, "get_bugs")?;

        crate::soap::parse_get_bugs_response(&response).map_err(Error::XmlError)
    }
}
