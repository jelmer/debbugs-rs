use lazy_regex::regex_is_match;
use maplit::hashmap;

use crate::BugId;

use std::collections::HashMap;
use xmltree::{Element, XMLNode};

pub const XMLNS_SOAP: &str = "http://xml.apache.org/xml-soap";
pub const XMLNS_SOAPENV: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const XMLNS_SOAPENC: &str = "http://schemas.xmlsoap.org/soap/encoding/";
pub const XMLNS_XSI: &str = "http://www.w3.org/1999/XMLSchema-instance";
pub const XMLNS_XSD: &str = "http://www.w3.org/1999/XMLSchema";

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
    assert_eq!(request.children.len(), 1);
    let body = request.children[0].as_element().unwrap();
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
pub struct BugReport {}

impl std::fmt::Display for BugReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
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

fn add_arg_xml<T: ToArgXml>(params: &mut Vec<xmltree::Element>, name: &str, arg: T) {
    params.push(xmltree::Element {
        prefix: None,
        namespace: None,
        namespaces: None,
        name: format!("arg{}", params.len()),
        attributes: HashMap::new(),
        children: vec![xmltree::XMLNode::Text(name.to_string())],
    });
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
        add_arg_xml(&mut params, "package", package);
    }

    if let Some(bug_ids) = query.bug_ids {
        add_arg_xml(&mut params, "bugs", bug_ids);
    }

    if let Some(submitter) = query.submitter {
        add_arg_xml(&mut params, "submitter", submitter);
    }

    if let Some(maintainer) = query.maintainer {
        add_arg_xml(&mut params, "maint", maintainer);
    }

    if let Some(src) = query.src {
        add_arg_xml(&mut params, "src", src);
    }

    if let Some(severity) = query.severity {
        add_arg_xml(&mut params, "severity", severity);
    }

    if let Some(status) = query.status {
        add_arg_xml(&mut params, "status", status.to_string().as_str());
    }

    if let Some(owner) = query.owner {
        add_arg_xml(&mut params, "owner", owner);
    }

    if let Some(correspondent) = query.correspondent {
        add_arg_xml(&mut params, "correspondent", correspondent);
    }

    if let Some(archive) = query.archive {
        add_arg_xml(&mut params, "archive", archive.to_string().as_str());
    }

    if let Some(tag) = query.tag {
        add_arg_xml(&mut params, "tag", tag);
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
