
use log::debug;


use crate::Error;

pub type SoapResponse = Result<(reqwest::StatusCode, String), reqwest::Error>;

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
        res.error_for_status_ref()?;
        let status = res.status();
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
}