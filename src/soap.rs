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
        faultcode: faultcode.get_text().unwrap().to_string(),
        faultstring: faultstring.get_text().unwrap().to_string(),
        faultactor: faultactor.and_then(|s| s.get_text()).map(|s| s.to_string()),
        detail: detail.and_then(|s| s.get_text()).map(|s| s.to_string()),
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
        .map(|e| e.clone())
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
    pub pending: Option<crate::BugStatus>,
    pub msgid: Option<String>,
    pub owner: Option<String>,
    pub keywords: Option<String>,
    pub affects: Option<String>,
    pub unarchived: Option<String>,
    pub forwarded: Option<String>,
    pub summary: Option<String>,
    pub bug_num: Option<i32>,
    pub archived: Option<crate::Archived>,
    pub found_versions: Option<Vec<Version>>,
    pub done: Option<String>,
    pub severity: Option<String>,
    pub package: Option<String>,
    pub fixed_versions: Option<Vec<(Option<String>, Version)>>,
    pub originator: Option<String>,
    pub blocks: Option<String>,
    pub found_date: Option<Vec<u32>>,
    pub outlook: Option<String>,
    pub id: Option<BugId>,
    pub found: bool,
    pub fixed: bool,
    pub last_modified: Option<u32>,
    pub tags: Option<String>,
    pub subject: Option<String>,
    pub location: Option<String>,
    pub mergedwith: Option<String>,
    pub blockedby: Option<String>,
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

fn parse_version(input: &str) -> (Option<String>, Version) {
    match input.split_once('/') {
        None => (None, input.parse().unwrap()),
        Some((package, version)) => (Some(package.to_string()), version.parse().unwrap()),
    }
}

impl From<&xmltree::Element> for BugReport {
    fn from(item: &xmltree::Element) -> Self {
        Self {
            pending: item
                .get_child("pending")
                .map(|e| e.get_text().unwrap().parse().unwrap()),
            msgid: item
                .get_child("msgid")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            owner: item
                .get_child("owner")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            keywords: item
                .get_child("keywords")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            affects: item
                .get_child("affects")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            unarchived: item
                .get_child("unarchived")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            blocks: item
                .get_child("blocks")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            found_date: item.get_child("found_date").map(|e| {
                e.children
                    .iter()
                    .filter_map(|c| c.as_element())
                    .filter_map(|c| {
                        if c.name == "item" {
                            Some(c.get_text().unwrap().parse().unwrap())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            }),
            fixed_versions: item.get_child("fixed_versions").map(|f| {
                f.children
                    .iter()
                    .filter_map(|c| {
                        c.as_element()
                            .and_then(|c| if c.name == "item" { Some(c) } else { None })
                    })
                    .map(|d| parse_version(d.get_text().unwrap().as_ref()))
                    .collect::<Vec<_>>()
            }),
            outlook: item
                .get_child("outlook")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            done: item
                .get_child("done")
                .map(|e| e.get_text().unwrap().parse().unwrap()),
            forwarded: item
                .get_child("forwarded")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            summary: item
                .get_child("summary")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            bug_num: item
                .get_child("bug_num")
                .map(|e| e.get_text().unwrap().parse().unwrap()),
            id: item
                .get_child("id")
                .map(|e| e.get_text().unwrap().parse().unwrap()),
            archived: item
                .get_child("archived")
                .map(|e| e.get_text().unwrap().parse().unwrap()),
            found_versions: item.get_child("found_versions").map(|f| {
                f.children
                    .iter()
                    .filter_map(|c| {
                        c.as_element()
                            .and_then(|c| if c.name == "item" { Some(c) } else { None })
                    })
                    .map(|d| d.get_text().unwrap().parse::<Version>().unwrap())
                    .collect::<Vec<_>>()
            }),
            found: item.get_child("found").is_some(),
            fixed: item.get_child("fixed").is_some(),
            last_modified: item
                .get_child("last_modified")
                .map(|e| e.get_text().unwrap().parse().unwrap()),
            tags: item
                .get_child("tags")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            subject: item
                .get_child("subject")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            source: item
                .get_child("source")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            originator: item
                .get_child("originator")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            package: item
                .get_child("package")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            location: item
                .get_child("location")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            log_modified: item
                .get_child("log_modified")
                .map(|e| e.get_text().unwrap().parse().unwrap()),
            mergedwith: item
                .get_child("mergedwith")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            severity: item
                .get_child("severity")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            blockedby: item
                .get_child("blockedby")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string()),
            fixed_date: item.get_child("fixed_date").map(|e| {
                e.children
                    .iter()
                    .filter_map(|c| c.as_element())
                    .filter_map(|c| {
                        if c.name == "item" {
                            Some(c.get_text().unwrap().parse().unwrap())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BugLog {
    pub header: String,
    pub msgnum: usize,
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
                    header = Some(e.get_text().unwrap().to_string());
                }
                "msg_num" => {
                    msgnum = Some(e.get_text().unwrap().parse().unwrap());
                }
                "body" => {
                    body = Some(e.get_text().unwrap().to_string());
                }
                "attachments" => {
                    if !e.children.is_empty() {
                        panic!("Attachments not supported yet");
                    }
                }
                n => {
                    panic!("Unknown element: {}", n)
                }
            }
        }
    }
    Ok(BugLog {
        header: header.unwrap(),
        msgnum: msgnum.unwrap(),
        body: body.unwrap(),
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
        let mut children = Vec::new();
        for s in self.iter() {
            children.push(xmltree::Element {
                prefix: None,
                namespace: None,
                namespaces: None,
                name: "item".to_string(),
                attributes: hashmap! {
                    "xsi:type".to_string() => "xsd:string".to_string(),
                },
                children: vec![xmltree::XMLNode::Text(s.to_string())],
            });
        }
        xmltree::Element {
            prefix: None,
            namespace: None,
            namespaces: Some(namespace),
            name,
            attributes: hashmap! {
                "xsi:type".to_string() => "soapenc:Array".to_string(),
                "soapenc:arrayType".to_string() => "xsd:string[]".to_string(),
            },
            children: children
                .into_iter()
                .map(xmltree::XMLNode::Element)
                .collect(),
        }
    }
}

impl ToArgXml for &[BugId] {
    fn to_arg_xml(&self, name: String) -> xmltree::Element {
        let mut namespace = xmltree::Namespace::empty();
        namespace.put("xsi", XMLNS_XSI);
        namespace.put("soapenc", XMLNS_SOAPENC);
        namespace.put("xsd", XMLNS_XSD);
        let mut children = Vec::new();
        for bug_id in self.iter() {
            children.push(xmltree::Element {
                prefix: None,
                namespace: None,
                namespaces: None,
                name: "item".to_string(),
                attributes: hashmap! {
                    "xsi:type".to_string() => "xsd:int".to_string(),
                },
                children: vec![xmltree::XMLNode::Text(bug_id.to_string())],
            });
        }
        xmltree::Element {
            prefix: None,
            namespace: None,
            namespaces: Some(namespace),
            name,
            attributes: hashmap! {
                "xsi:type".to_string() => "soapenc:Array".to_string(),
                "soapenc:arrayType".to_string() => format!("xsd:int[{}]", self.len()),
            },
            children: children
                .into_iter()
                .map(xmltree::XMLNode::Element)
                .collect(),
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
                    .unwrap();

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
                        ids.push(e.get_text().unwrap().parse::<BugId>().unwrap());
                    }
                }
            }
            ret.insert(e.name.clone(), ids);
        }
    }

    Ok(ret)
}
