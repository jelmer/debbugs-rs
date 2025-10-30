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
    /// Creates a new Debbugs client connected to the default Debian instance
    ///
    /// Uses the official Debian bug tracking system at bugs.debian.org
    fn default() -> Self {
        Self::new(DEFAULT_URL)
    }
}

impl Debbugs {
    /// Creates a new Debbugs client for a custom server
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the Debbugs SOAP endpoint
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use debbugs::Debbugs;
    ///
    /// let client = Debbugs::new("https://custom-debbugs.example.com/soap.cgi");
    /// ```
    pub fn new<S: Into<String>>(url: S) -> Self {
        Debbugs {
            client: reqwest::Client::new(),
            url: url.into(),
        }
    }
}

/// Async client for the Debian Bug Tracking System (Debbugs)
///
/// This client provides an async interface to query bug reports, search for bugs,
/// and retrieve detailed information from a Debbugs instance.
///
/// # Examples
///
/// ```no_run
/// use debbugs::Debbugs;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Debbugs::default();
///     let bugs = client.newest_bugs(10).await?;
///     println!("Found {} newest bugs", bugs.len());
///     Ok(())
/// }
/// ```
pub struct Debbugs {
    client: reqwest::Client,
    url: String,
}

impl Debbugs {
    /// Retrieves the newest bugs from the bug tracking system
    ///
    /// Returns a list of bug IDs, ordered from newest to oldest.
    ///
    /// # Arguments
    ///
    /// * `amount` - The maximum number of bug IDs to retrieve
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use debbugs::Debbugs;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Debbugs::default();
    ///     let bugs = client.newest_bugs(5).await?;
    ///     println!("Latest 5 bugs: {:?}", bugs);
    ///     Ok(())
    /// }
    /// ```
    pub async fn newest_bugs(&self, amount: i32) -> Result<Vec<BugId>, Error> {
        let request = soap::newest_bugs_request(amount);
        let (_status, response) = self.send_soap_request(&request, "newest_bugs").await?;

        soap::parse_newest_bugs_response(&response).map_err(Error::XmlError)
    }

    /// Retrieves the complete log of messages for a specific bug
    ///
    /// Returns all messages (emails) that have been sent regarding this bug,
    /// including the initial bug report and all subsequent correspondence.
    ///
    /// # Arguments
    ///
    /// * `bug_id` - The ID of the bug to retrieve logs for
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use debbugs::Debbugs;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Debbugs::default();
    ///     let logs = client.get_bug_log(12345).await?;
    ///     for log in logs {
    ///         println!("Message: {}", log.header);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_bug_log(&self, bug_id: BugId) -> Result<Vec<BugLog>, Error> {
        let request = soap::get_bug_log_request(bug_id);
        let (_status, response) = self.send_soap_request(&request, "get_bug_log").await?;

        soap::parse_get_bug_log_response(&response).map_err(Error::XmlError)
    }

    /// Searches for bugs matching the specified criteria
    ///
    /// Returns a list of bug IDs that match the search query. Use `SearchQuery`
    /// to specify search parameters like package, severity, status, etc.
    ///
    /// # Arguments
    ///
    /// * `query` - Search criteria for finding bugs
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use debbugs::{Debbugs, SearchQuery};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Debbugs::default();
    ///     let search = SearchQuery {
    ///         package: Some("rust-debbugs"),
    ///         severity: Some("serious"),
    ///         ..Default::default()
    ///     };
    ///     let bugs = client.get_bugs(&search).await?;
    ///     println!("Found {} serious bugs in rust-debbugs", bugs.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_bugs(&self, query: &SearchQuery<'_>) -> Result<Vec<BugId>, Error> {
        let request = soap::get_bugs_request(query);
        let (_status, response) = self.send_soap_request(&request, "get_bugs").await?;

        soap::parse_get_bugs_response(&response).map_err(Error::XmlError)
    }

    /// Retrieves detailed status information for specific bugs
    ///
    /// Returns a map of bug IDs to their detailed bug reports, including
    /// information like title, severity, status, package, and more.
    ///
    /// # Arguments
    ///
    /// * `bug_ids` - A slice of bug IDs to retrieve status for
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use debbugs::Debbugs;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Debbugs::default();
    ///     let reports = client.get_status(&[12345, 67890]).await?;
    ///     for (bug_id, report) in reports {
    ///         println!("Bug #{}: {} ({})",
    ///             bug_id,
    ///             report.subject.as_deref().unwrap_or("No subject"),
    ///             report.severity.as_deref().unwrap_or("No severity")
    ///         );
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_status(
        &self,
        bug_ids: &[BugId],
    ) -> Result<std::collections::HashMap<BugId, crate::soap::BugReport>, Error> {
        let request = crate::soap::get_status_request(bug_ids);
        let (_status, response) = self.send_soap_request(&request, "get_status").await?;

        crate::soap::parse_get_status_response(&response).map_err(Error::XmlError)
    }

    /// Retrieves user tags for a specific email address
    ///
    /// User tags allow users to categorize bugs with custom labels.
    /// This method returns bugs tagged by a specific user.
    ///
    /// # Arguments
    ///
    /// * `email` - The email address of the user whose tags to retrieve
    /// * `usertags` - A slice of specific tag names to filter by (empty slice for all tags)
    ///
    /// # Returns
    ///
    /// A map where keys are tag names and values are lists of bug IDs with that tag.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use debbugs::Debbugs;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Debbugs::default();
    ///     let tags = client.get_usertag("user@example.com", &[]).await?;
    ///     for (tag, bugs) in tags {
    ///         println!("Tag '{}' has {} bugs", tag, bugs.len());
    ///     }
    ///     Ok(())
    /// }
    /// ```
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
