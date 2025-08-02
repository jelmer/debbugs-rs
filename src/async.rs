use crate::soap;
use crate::{BugId, BugLog, Error, SearchQuery, SoapResponse, DEFAULT_URL};
use log::debug;

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
            .header("SOAPAction", action);
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
        Self::new(DEFAULT_URL)
    }
}

impl Debbugs {
    pub fn new<S: Into<String>>(url: S) -> Self {
        Debbugs {
            client: reqwest::Client::new(),
            url: url.into(),
        }
    }
}

pub struct Debbugs {
    client: reqwest::Client,
    url: String,
}

impl Debbugs {
    pub async fn newest_bugs(&self, amount: i32) -> Result<Vec<BugId>, Error> {
        let request = soap::newest_bugs_request(amount);
        let (_status, response) = self.send_soap_request(&request, "newest_bugs").await?;

        soap::parse_newest_bugs_response(&response).map_err(Error::XmlError)
    }

    pub async fn get_bug_log(&self, bug_id: BugId) -> Result<Vec<BugLog>, Error> {
        let request = soap::get_bug_log_request(bug_id);
        let (_status, response) = self.send_soap_request(&request, "get_bug_log").await?;

        soap::parse_get_bug_log_response(&response).map_err(Error::XmlError)
    }

    pub async fn get_bugs(&self, query: &SearchQuery<'_>) -> Result<Vec<BugId>, Error> {
        let request = soap::get_bugs_request(query);
        let (_status, response) = self.send_soap_request(&request, "get_bugs").await?;

        soap::parse_get_bugs_response(&response).map_err(Error::XmlError)
    }

    pub async fn get_status(
        &self,
        bug_ids: &[BugId],
    ) -> Result<std::collections::HashMap<BugId, crate::soap::BugReport>, Error> {
        let request = crate::soap::get_status_request(bug_ids);
        let (_status, response) = self.send_soap_request(&request, "get_status").await?;

        crate::soap::parse_get_status_response(&response).map_err(Error::XmlError)
    }

    pub async fn get_usertag(
        &self,
        email: &str,
        usertags: &[&str],
    ) -> Result<std::collections::HashMap<String, Vec<BugId>>, Error> {
        let request = crate::soap::get_usertag_request(email, usertags);
        let (_status, response) = self.send_soap_request(&request, "get_usertag").await?;

        crate::soap::parse_get_usertag_response(&response).map_err(Error::XmlError)
    }
}
