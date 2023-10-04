use log::debug;

use crate::{Error, SoapResponse};

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
        Self::new("https://debbugs.gnu.org/cgi/soap.cgi")
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
    pub fn newest_bugs(&self, amount: i32) -> Result<Vec<i32>, Error> {
        let request = crate::soap::newest_bugs_request(amount);
        let (_status, response) = self.send_soap_request(&request, "Debbugs/SOAP")?;

        crate::soap::parse_newest_bugs_response(&response).map_err(Error::XmlError)
    }

    pub fn get_bug_log(&self, bug_id: i32) -> Result<Vec<crate::soap::BugLog>, Error> {
        let request = crate::soap::get_bug_log_request(bug_id);
        let (_status, response) = self.send_soap_request(&request, "Debbugs/SOAP")?;

        crate::soap::parse_get_bug_log_response(&response).map_err(Error::XmlError)
    }
}
