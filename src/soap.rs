use debversion::Version;
use lazy_regex::regex_is_match;
use maplit::hashmap;

use crate::BugId;

use std::collections::HashMap;
use xmltree::{Element, XMLNode};

#[allow(dead_code)]
pub const XMLNS_SOAP: &str = "http://xml.apache.org/xml-soap";
pub const XMLNS_SOAPENV: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const XMLNS_SOAPENC: &str = "http://schemas.xmlsoap.org/soap/encoding/";
pub const XMLNS_XSI: &str = "http://www.w3.org/1999/XMLSchema-instance";
pub const XMLNS_XSD: &str = "http://www.w3.org/1999/XMLSchema";
pub const XMLNS_DEBBUGS: &str = "Debbugs/SOAP";

#[derive(Debug, PartialEq)]
pub struct Fault {
    pub faultcode: String,
    pub faultstring: String,
    pub faultactor: Option<String>,
    pub detail: Option<String>,
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{{ faultcode: {}, faultstring: {}, faultactor: {:?}, detail: {:?} }}",
            self.faultcode, self.faultstring, self.faultactor, self.detail
        )
    }
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(format!("Invalid boolean value: {}", s)),
    }
}

pub(crate) fn parse_fault(input: &str) -> Result<Fault, String> {
    // Parse the input XML string into an Element
    let root = Element::parse(input.as_bytes()).map_err(|e| e.to_string())?;

    if root.name != "Envelope" || root.namespace.as_deref() != Some(XMLNS_SOAPENV) {
        return Err("Root element is not a valid soap:Envelope".into());
    }

    let body_elem = root.get_child("Body").ok_or("soap:Body not found")?;

    if body_elem.namespace.as_deref() != Some(XMLNS_SOAPENV) {
        return Err(format!(
            "Namespace for soap:Body is incorrect: {:?}",
            body_elem.namespace
        ));
    }

    let fault = body_elem.get_child("Fault").ok_or("soap:Fault not found")?;

    let faultcode = fault.get_child("faultcode").ok_or("faultcode not found")?;
    let faultstring = fault
        .get_child("faultstring")
        .ok_or("faultstring not found")?;
    let faultactor = fault.get_child("faultactor");
    let detail = fault.get_child("detail");

    Ok(Fault {
        faultcode: faultcode
            .get_text()
            .ok_or("faultcode has no text")?
            .into_owned(),
        faultstring: faultstring
            .get_text()
            .ok_or("faultstring has no text")?
            .into_owned(),
        faultactor: faultactor
            .and_then(|s| s.get_text())
            .map(|s| s.into_owned()),
        detail: detail.and_then(|s| s.get_text()).map(|s| s.into_owned()),
    })
}

fn build_request_envelope(name: &str, arguments: Vec<Element>) -> xmltree::Element {
    let mut namespace = xmltree::Namespace::empty();
    namespace.put("soap", XMLNS_SOAPENV);
    namespace.put("xsi", XMLNS_XSI);
    namespace.put("xsd", XMLNS_XSD);

    Element {
        name: "Envelope".to_string(),
        prefix: Some("soap".to_string()),
        namespaces: Some(namespace.clone()),
        namespace: Some(XMLNS_SOAPENV.to_string()),
        attributes: hashmap! {},
        children: vec![
            XMLNode::Element(Element {
                name: "Header".to_string(),
                prefix: Some("soap".to_string()),
                namespaces: Some(namespace.clone()),
                namespace: Some(XMLNS_SOAPENV.to_string()),
                attributes: hashmap![],
                children: vec![],
            }),
            XMLNode::Element(Element {
                name: "Body".to_string(),
                prefix: Some("soap".to_string()),
                namespaces: Some(namespace.clone()),
                namespace: Some(XMLNS_SOAPENV.to_string()),
                attributes: hashmap![],
                children: vec![XMLNode::Element(Element {
                    name: name.to_string(),
                    namespaces: None,
                    children: arguments.into_iter().map(XMLNode::Element).collect(),
                    prefix: None,
                    namespace: None,
                    attributes: hashmap! {},
                })],
            }),
        ],
    }
}

pub(crate) fn newest_bugs_request(amount: i32) -> xmltree::Element {
    build_request_envelope(
        "newest_bugs",
        vec![Element {
            name: "amount".to_string(),
            namespaces: None,
            children: vec![XMLNode::Text(amount.to_string())],
            prefix: None,
            namespace: None,
            attributes: hashmap! {},
        }],
    )
}

pub(crate) fn get_bug_log_request(bugid: i32) -> xmltree::Element {
    let mut namespace = xmltree::Namespace::empty();
    namespace.put("xsi", XMLNS_XSI);
    namespace.put("xsd", XMLNS_XSD);
    build_request_envelope(
        "get_bug_log",
        vec![Element {
            name: "bugnumber".to_string(),
            namespaces: Some(namespace.clone()),
            children: vec![XMLNode::Text(bugid.to_string())],
            prefix: None,
            namespace: None,
            attributes: hashmap! {
                "xsi:type".into() => "xsd:int".into(),
            },
        }],
    )
}

#[test]
fn test_newest_bugs_request_serialize() {
    let request = newest_bugs_request(10);
    assert_eq!(request.name, "Envelope");
    assert_eq!(request.namespace.as_deref(), Some(XMLNS_SOAPENV));
    assert_eq!(request.children.len(), 2);
    let header = request.children[0].as_element().unwrap();
    assert_eq!(header.name, "Header");
    let body = request.children[1].as_element().unwrap();
    assert_eq!(body.name, "Body");
    assert_eq!(body.namespace.as_deref(), Some(XMLNS_SOAPENV));
    assert_eq!(body.children.len(), 1);
    let newest_bugs = body.children[0].as_element().unwrap();
    assert_eq!(newest_bugs.name, "newest_bugs");
    assert_eq!(newest_bugs.namespace, None);
    assert_eq!(newest_bugs.children.len(), 1);
    let amount = newest_bugs.children[0].as_element().unwrap();
    assert_eq!(amount.name, "amount");
    assert_eq!(amount.namespace, None);
    assert_eq!(amount.children.len(), 1);
    assert_eq!(amount.children[0].as_text().unwrap(), "10");
}

fn parse_response_envelope(input: &str, name: &str) -> Result<xmltree::Element, String> {
    // Parse the input XML string into an Element
    let root = Element::parse(input.as_bytes()).map_err(|e| e.to_string())?;

    if root.name != "Envelope" || root.namespace.as_deref() != Some(XMLNS_SOAPENV) {
        return Err("Root element is not a valid soap:Envelope".into());
    }

    let body_elem = root.get_child("Body").ok_or("soap:Body not found")?;

    if body_elem.namespace.as_deref() != Some(XMLNS_SOAPENV) {
        return Err(format!(
            "Namespace for soap:Body is incorrect: {:?}",
            body_elem.namespace
        ));
    }

    let elem_name = format!("{}Response", name);

    body_elem
        .get_child(elem_name.as_str())
        .ok_or(format!("{} not found", elem_name))
        .cloned()
}

pub(crate) fn parse_newest_bugs_response(input: &str) -> Result<Vec<i32>, String> {
    let response_elem = parse_response_envelope(input, "newest_bugs")?;

    let array_elem = response_elem
        .get_child("Array")
        .ok_or("soapenc:Array not found")?;

    if array_elem.namespace.as_deref() != Some(XMLNS_SOAPENC) {
        return Err(format!(
            "Namespace for soapenc:Array is incorrect: {:?}",
            array_elem.namespace
        ));
    }

    match array_elem.attributes.get("arrayType") {
        None => return Err("soapenc:Array does not have soapenc:arrayType attribute".to_string()),
        Some(value) => {
            if !regex_is_match!(r"xsd:int\[[0-9]+\]", value) && value != "xsd:anyType[0]" {
                return Err(format!(
                    "soapenc:Array has incorrect soapenc:arrayType attribute: {}",
                    value
                ));
            }
        }
    }

    // Extract the integers from the item elements
    let mut integers = Vec::new();
    for item in array_elem.children.iter() {
        if let xmltree::XMLNode::Element(e) = item {
            if e.name == "item" {
                if let Some(text) = e.get_text() {
                    if let Ok(num) = text.parse() {
                        integers.push(num);
                    }
                }
            }
        }
    }

    Ok(integers)
}

#[test]
fn test_parse_newest_bugs_response() {
    let text = r###"<?xml version="1.0" encoding="UTF-8"?><soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:soapenc="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><soap:Body><newest_bugsResponse xmlns="Debbugs/SOAP"><soapenc:Array soapenc:arrayType="xsd:int[10]" xsi:type="soapenc:Array"><item xsi:type="xsd:int">66320</item><item xsi:type="xsd:int">66321</item><item xsi:type="xsd:int">66322</item><item xsi:type="xsd:int">66323</item><item xsi:type="xsd:int">66324</item><item xsi:type="xsd:int">66325</item><item xsi:type="xsd:int">66326</item><item xsi:type="xsd:int">66327</item><item xsi:type="xsd:int">66328</item><item xsi:type="xsd:int">66329</item></soapenc:Array></newest_bugsResponse></soap:Body></soap:Envelope>"###;
    let integers = parse_newest_bugs_response(text).unwrap();
    assert_eq!(
        integers,
        vec![66320, 66321, 66322, 66323, 66324, 66325, 66326, 66327, 66328, 66329]
    );
}

#[derive(Debug)]
pub struct BugReport {
    pub pending: Option<crate::Pending>,
    pub msgid: Option<String>,
    pub owner: Option<String>,
    #[deprecated = "Use tags instead"]
    pub keywords: Option<String>,
    pub affects: Option<String>,
    /// Has the bug been unrarchived and can be archived again
    pub unarchived: Option<bool>,
    pub forwarded: Option<String>,
    pub summary: Option<String>,
    /// The bugnumber
    pub bug_num: Option<BugId>,
    /// The bug is archived or not
    pub archived: Option<bool>,
    pub found_versions: Option<Vec<Version>>,
    pub done: Option<String>,
    /// Severity of the bugreport
    pub severity: Option<String>,
    /// Package of the bugreport
    pub package: Option<String>,
    pub fixed_versions: Option<Vec<(Option<String>, Option<Version>)>>,
    pub originator: Option<String>,
    pub blocks: Option<String>,
    #[deprecated(note = "empty for now")]
    pub found_date: Option<Vec<u32>>,
    pub outlook: Option<String>,
    #[deprecated(note = "use bug_num")]
    pub id: Option<BugId>,
    pub found: bool,
    pub fixed: bool,
    pub last_modified: Option<u32>,
    pub tags: Option<String>,
    /// Subject/Title of the bugreport
    pub subject: Option<String>,
    pub location: Option<String>,
    /// The bugs this bug was merged with
    pub mergedwith: Option<Vec<BugId>>,
    pub blockedby: Option<String>,
    #[deprecated(note = "empty for now")]
    pub fixed_date: Option<Vec<u32>>,
    pub log_modified: Option<u32>,
    pub source: Option<String>,
}

impl std::fmt::Display for BugReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(bug_num) = self.bug_num {
            write!(f, "Bug #{}", bug_num)?;
        } else {
            write!(f, "Bug #?")?;
        }
        if let Some(package) = &self.package {
            write!(f, " in {}", package)?;
        }
        if let Some(summary) = &self.summary {
            write!(f, ": {}", summary)?;
        }
        Ok(())
    }
}

fn parse_version(input: &str) -> (Option<String>, Option<Version>) {
    match input.split_once('/') {
        None => (None, input.parse().ok()),
        Some((package, version)) => (Some(package.to_string()), version.parse().ok()),
    }
}

// Hide the deprecated warnings, since we intentionally still populate the deprecated fields
#[allow(deprecated)]
impl From<&xmltree::Element> for BugReport {
    fn from(item: &xmltree::Element) -> Self {
        // Helper function to get text and convert to owned String
        fn get_text_owned(element: Option<&xmltree::Element>) -> Option<String> {
            element.and_then(|e| e.get_text()).map(|s| s.into_owned())
        }

        // Helper function to parse a child element's text content
        fn parse_child<T: std::str::FromStr>(
            item: &xmltree::Element,
            child_name: &str,
        ) -> Option<T> {
            item.get_child(child_name)
                .and_then(|e| e.get_text())
                .and_then(|text| text.parse().ok())
        }

        // Helper function to parse items from a list element
        fn parse_list_items<T, F>(element: Option<&xmltree::Element>, parser: F) -> Option<Vec<T>>
        where
            F: Fn(&str) -> Option<T>,
        {
            element.map(|e| {
                e.children
                    .iter()
                    .filter_map(|c| c.as_element())
                    .filter(|c| c.name == "item")
                    .filter_map(|c| c.get_text().and_then(|text| parser(text.as_ref())))
                    .collect()
            })
        }

        Self {
            pending: parse_child(item, "pending"),
            msgid: get_text_owned(item.get_child("msgid")),
            owner: get_text_owned(item.get_child("owner")),
            keywords: get_text_owned(item.get_child("keywords")),
            affects: get_text_owned(item.get_child("affects")),
            unarchived: item
                .get_child("unarchived")
                .and_then(|e| e.get_text())
                .and_then(|s| parse_bool(s.as_ref()).ok()),
            blocks: get_text_owned(item.get_child("blocks")),
            found_date: parse_list_items(item.get_child("found_date"), |s| s.parse().ok()),
            fixed_versions: parse_list_items(item.get_child("fixed_versions"), |s| {
                Some(parse_version(s))
            }),
            outlook: get_text_owned(item.get_child("outlook")),
            done: get_text_owned(item.get_child("done")),
            forwarded: get_text_owned(item.get_child("forwarded")),
            summary: get_text_owned(item.get_child("summary")),
            bug_num: parse_child(item, "bug_num"),
            id: parse_child(item, "id"),
            archived: item
                .get_child("archived")
                .and_then(|e| e.get_text())
                .and_then(|t| parse_bool(t.as_ref()).ok()),
            found_versions: parse_list_items(item.get_child("found_versions"), |s| {
                s.parse::<Version>().ok()
            }),
            found: item.get_child("found").is_some(),
            fixed: item.get_child("fixed").is_some(),
            last_modified: parse_child(item, "last_modified"),
            tags: get_text_owned(item.get_child("tags")),
            subject: get_text_owned(item.get_child("subject")),
            source: get_text_owned(item.get_child("source")),
            originator: get_text_owned(item.get_child("originator")),
            package: get_text_owned(item.get_child("package")),
            location: get_text_owned(item.get_child("location")),
            log_modified: parse_child(item, "log_modified"),
            mergedwith: item
                .get_child("mergedwith")
                .and_then(|e| e.get_text())
                .map(|s| {
                    s.split_whitespace()
                        .filter_map(|i| i.parse().ok())
                        .collect()
                }),
            severity: get_text_owned(item.get_child("severity")),
            blockedby: get_text_owned(item.get_child("blockedby")),
            fixed_date: parse_list_items(item.get_child("fixed_date"), |s| s.parse().ok()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BugLog {
    pub header: String,
    pub msgnum: BugId,
    pub body: String,
}

impl BugLog {
    #[cfg(feature = "mailparse")]
    pub fn headers(&self) -> Result<Vec<mailparse::MailHeader>, mailparse::MailParseError> {
        let (headers, _ix_body) = mailparse::parse_headers(self.header.as_bytes())?;
        Ok(headers)
    }
}

fn parse_buglog(item: &xmltree::Element) -> Result<BugLog, String> {
    let mut header = None;
    let mut msgnum = None;
    let mut body = None;
    for child in item.children.iter() {
        if let xmltree::XMLNode::Element(e) = child {
            match e.name.as_str() {
                "header" => {
                    header = e.get_text().map(|s| s.into_owned());
                }
                "msg_num" => {
                    msgnum = e.get_text().and_then(|s| s.parse().ok());
                }
                "body" => {
                    body = e.get_text().map(|s| s.into_owned());
                }
                "attachments" => {
                    if !e.children.is_empty() {
                        log::warn!("Attachments found but not supported (apparently not implemented on the server side)");
                    }
                }
                n => {
                    return Err(format!("Unknown element: {}", n));
                }
            }
        }
    }
    Ok(BugLog {
        header: header.ok_or("Missing header element")?,
        msgnum: msgnum.ok_or("Missing or invalid msg_num element")?,
        body: body.ok_or("Missing body element")?,
    })
}

pub(crate) fn parse_get_bug_log_response(input: &str) -> Result<Vec<BugLog>, String> {
    let response_elem = parse_response_envelope(input, "get_bug_log")?;

    let array_elem = response_elem
        .get_child("Array")
        .ok_or("soapenc:Array not found")?;

    if array_elem.namespace.as_deref() != Some(XMLNS_SOAPENC) {
        return Err(format!(
            "Namespace for soapenc:Array is incorrect: {:?}",
            array_elem.namespace
        ));
    }

    match array_elem.attributes.get("arrayType") {
        None => return Err("soapenc:Array does not have soapenc:arrayType attribute".to_string()),
        Some(value) => {
            if !regex_is_match!(r"xsd:ur-type\[[0-9]+\]", value) && value != "xsd:anyType[0]" {
                return Err(format!(
                    "soapenc:Array has incorrect soapenc:arrayType attribute: {}",
                    value
                ));
            }
        }
    }

    if array_elem.attributes.get("type") != Some(&"soapenc:Array".to_string()) {
        return Err(format!(
            "soapenc:Array does not have xsi:type attribute: {:?}",
            array_elem.attributes.get("type")
        ));
    }

    let mut ret = vec![];
    for item in array_elem.children.iter() {
        if let xmltree::XMLNode::Element(e) = item {
            if e.name == "item" {
                ret.push(parse_buglog(e)?);
            }
        }
    }
    Ok(ret)
}

trait ToArgXml {
    fn to_arg_xml(&self, name: String) -> xmltree::Element;
}

impl ToArgXml for &str {
    fn to_arg_xml(&self, name: String) -> xmltree::Element {
        xmltree::Element {
            prefix: None,
            namespace: None,
            namespaces: None,
            name,
            attributes: HashMap::new(),
            children: vec![xmltree::XMLNode::Text(self.to_string())],
        }
    }
}

impl ToArgXml for &[&str] {
    fn to_arg_xml(&self, name: String) -> xmltree::Element {
        let mut namespace = xmltree::Namespace::empty();
        namespace.put("xsi", XMLNS_XSI);
        namespace.put("soapenc", XMLNS_SOAPENC);
        namespace.put("xsd", XMLNS_XSD);
        let children: Vec<_> = self
            .iter()
            .map(|s| {
                xmltree::XMLNode::Element(xmltree::Element {
                    prefix: None,
                    namespace: None,
                    namespaces: None,
                    name: "item".to_string(),
                    attributes: hashmap! {
                        "xsi:type".to_string() => "xsd:string".to_string(),
                    },
                    children: vec![xmltree::XMLNode::Text(s.to_string())],
                })
            })
            .collect();
        xmltree::Element {
            prefix: None,
            namespace: None,
            namespaces: Some(namespace),
            name,
            attributes: hashmap! {
                "xsi:type".to_string() => "soapenc:Array".to_string(),
                "soapenc:arrayType".to_string() => "xsd:string[]".to_string(),
            },
            children,
        }
    }
}

impl ToArgXml for &[BugId] {
    fn to_arg_xml(&self, name: String) -> xmltree::Element {
        let mut namespace = xmltree::Namespace::empty();
        namespace.put("xsi", XMLNS_XSI);
        namespace.put("soapenc", XMLNS_SOAPENC);
        namespace.put("xsd", XMLNS_XSD);
        let children: Vec<_> = self
            .iter()
            .map(|bug_id| {
                xmltree::XMLNode::Element(xmltree::Element {
                    prefix: None,
                    namespace: None,
                    namespaces: None,
                    name: "item".to_string(),
                    attributes: hashmap! {
                        "xsi:type".to_string() => "xsd:int".to_string(),
                    },
                    children: vec![xmltree::XMLNode::Text(bug_id.to_string())],
                })
            })
            .collect();
        xmltree::Element {
            prefix: None,
            namespace: None,
            namespaces: Some(namespace),
            name,
            attributes: hashmap! {
                "xsi:type".to_string() => "soapenc:Array".to_string(),
                "soapenc:arrayType".to_string() => format!("xsd:int[{}]", self.len()),
            },
            children,
        }
    }
}

fn add_arg_xml<T: ToArgXml>(params: &mut Vec<xmltree::Element>, arg: T) {
    params.push(arg.to_arg_xml(format!("arg{}", params.len())));
}

#[derive(Debug, Default, Clone)]
pub struct SearchQuery<'a> {
    pub package: Option<&'a str>,
    pub bug_ids: Option<&'a [BugId]>,
    pub submitter: Option<&'a str>,
    pub maintainer: Option<&'a str>,
    pub src: Option<&'a str>,
    pub severity: Option<&'a str>,
    pub status: Option<crate::BugStatus>,
    pub owner: Option<&'a str>,
    pub correspondent: Option<&'a str>,
    pub archive: Option<crate::Archived>,
    pub tag: Option<&'a [&'a str]>,
}

pub(crate) fn get_bugs_request(query: &SearchQuery) -> xmltree::Element {
    let mut params = Vec::new();

    if let Some(package) = query.package {
        add_arg_xml(&mut params, "package");
        add_arg_xml(&mut params, package);
    }

    if let Some(bug_ids) = query.bug_ids {
        add_arg_xml(&mut params, "bugs");
        add_arg_xml(&mut params, bug_ids);
    }

    if let Some(submitter) = query.submitter {
        add_arg_xml(&mut params, "submitter");
        add_arg_xml(&mut params, submitter);
    }

    if let Some(maintainer) = query.maintainer {
        add_arg_xml(&mut params, "maint");
        add_arg_xml(&mut params, maintainer);
    }

    if let Some(src) = query.src {
        add_arg_xml(&mut params, "src");
        add_arg_xml(&mut params, src);
    }

    if let Some(severity) = query.severity {
        add_arg_xml(&mut params, "severity");
        add_arg_xml(&mut params, severity);
    }

    if let Some(status) = query.status {
        add_arg_xml(&mut params, "status");
        add_arg_xml(&mut params, status.to_string().as_str());
    }

    if let Some(owner) = query.owner {
        add_arg_xml(&mut params, "owner");
        add_arg_xml(&mut params, owner);
    }

    if let Some(correspondent) = query.correspondent {
        add_arg_xml(&mut params, "correspondent");
        add_arg_xml(&mut params, correspondent);
    }

    if let Some(archive) = query.archive {
        add_arg_xml(&mut params, "archive");
        add_arg_xml(&mut params, archive.to_string().as_str());
    }

    if let Some(tag) = query.tag {
        add_arg_xml(&mut params, "tag");
        add_arg_xml(&mut params, tag);
    }

    build_request_envelope("get_bugs", params)
}

pub(crate) fn parse_get_bugs_response(input: &str) -> Result<Vec<crate::BugId>, String> {
    let response_elem = parse_response_envelope(input, "get_bugs")?;

    let array_elem = response_elem
        .get_child("Array")
        .ok_or("soapenc:Array not found")?;

    if array_elem.namespace.as_deref() != Some(XMLNS_SOAPENC) {
        return Err(format!(
            "Namespace for soapenc:Array is incorrect: {:?}",
            array_elem.namespace
        ));
    }

    match array_elem.attributes.get("arrayType") {
        None => return Err("soapenc:Array does not have soapenc:arrayType attribute".to_string()),
        Some(value) => {
            if !regex_is_match!(r"xsd:int\[[0-9]+\]", value) && value != "xsd:anyType[0]" {
                return Err(format!(
                    "soapenc:Array has incorrect soapenc:arrayType attribute: {}",
                    value
                ));
            }
        }
    }

    // Extract the integers from the item elements
    let mut integers = Vec::new();
    for item in array_elem.children.iter() {
        if let xmltree::XMLNode::Element(e) = item {
            if e.name == "item" {
                if let Some(text) = e.get_text() {
                    if let Ok(num) = text.parse() {
                        integers.push(num);
                    }
                }
            }
        }
    }

    Ok(integers)
}

pub(crate) fn get_status_request(bug_ids: &[BugId]) -> xmltree::Element {
    let mut params = Vec::new();
    add_arg_xml(&mut params, bug_ids);
    build_request_envelope("get_status", params)
}

pub(crate) fn parse_get_status_response(input: &str) -> Result<HashMap<BugId, BugReport>, String> {
    let response_elem = parse_response_envelope(input, "get_status")?;

    if response_elem.namespace.as_deref() != Some(XMLNS_DEBBUGS) {
        return Err(format!(
            "Namespace for get_statusResponse is incorrect: {:?}",
            response_elem.namespace
        ));
    }

    let container = response_elem
        .get_child("s-gensym3")
        .ok_or("s-gensym3 not found")?;

    let mut ret = HashMap::new();
    for item in container.children.iter() {
        if let xmltree::XMLNode::Element(e) = item {
            if e.name == "item" {
                if e.namespace.as_deref() != Some(XMLNS_DEBBUGS) {
                    return Err(format!(
                        "Namespace for item is incorrect: {:?}",
                        e.namespace
                    ));
                }

                let key = e
                    .get_child("key")
                    .ok_or("key not found")?
                    .get_text()
                    .ok_or("key has no text")?
                    .parse::<BugId>()
                    .map_err(|_| "Invalid BugId format")?;

                let value = BugReport::from(e.get_child("value").ok_or("value not found")?);

                ret.insert(key, value);
            }
        }
    }

    Ok(ret)
}

pub(crate) fn get_usertag_request(email: &str, tags: &[&str]) -> xmltree::Element {
    let mut params = Vec::new();
    add_arg_xml(&mut params, email);
    for tag in tags {
        add_arg_xml(&mut params, *tag);
    }
    build_request_envelope("get_usertag", params)
}

pub(crate) fn parse_get_usertag_response(
    input: &str,
) -> Result<HashMap<String, Vec<crate::BugId>>, String> {
    let response_elem = parse_response_envelope(input, "get_usertag")?;

    let container = response_elem
        .get_child("s-gensym3")
        .ok_or("s-gensym3 not found")?;

    let mut ret = HashMap::new();

    for child in container.children.iter() {
        if let Some(e) = child.as_element() {
            let mut ids = vec![];
            for item in e.children.iter() {
                if let xmltree::XMLNode::Element(e) = item {
                    if e.name == "item" {
                        if let Some(text) = e.get_text() {
                            if let Ok(id) = text.parse::<BugId>() {
                                ids.push(id);
                            }
                        }
                    }
                }
            }
            ret.insert(e.name.clone(), ids);
        }
    }

    Ok(ret)
}
