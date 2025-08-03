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

#[test]
fn test_get_bug_log_request() {
    let request = get_bug_log_request(123456);
    assert_eq!(request.name, "Envelope");
    assert_eq!(request.namespace.as_deref(), Some(XMLNS_SOAPENV));

    let body = request.children[1].as_element().unwrap();
    let get_bug_log = body.children[0].as_element().unwrap();
    assert_eq!(get_bug_log.name, "get_bug_log");

    let bugnumber = get_bug_log.children[0].as_element().unwrap();
    assert_eq!(bugnumber.name, "bugnumber");
    assert_eq!(bugnumber.children[0].as_text().unwrap(), "123456");
    assert_eq!(bugnumber.attributes.get("xsi:type").unwrap(), "xsd:int");
}

#[test]
fn test_get_bugs_request() {
    let query = SearchQuery {
        package: Some("test-package"),
        owner: Some("test@example.com"),
        ..Default::default()
    };
    let request = get_bugs_request(&query);

    assert_eq!(request.name, "Envelope");
    let body = request.children[1].as_element().unwrap();
    let get_bugs = body.children[0].as_element().unwrap();
    assert_eq!(get_bugs.name, "get_bugs");

    // Check that arguments are present (package + owner = 4 args: "package", value, "owner", value)
    let args: Vec<&Element> = get_bugs
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .collect();
    assert_eq!(args.len(), 4); // Should have package key, package value, owner key, owner value
}

#[test]
fn test_get_status_request_single() {
    let request = get_status_request(&[123456]);

    assert_eq!(request.name, "Envelope");
    let body = request.children[1].as_element().unwrap();
    let get_status = body.children[0].as_element().unwrap();
    assert_eq!(get_status.name, "get_status");

    let bugnumbers = get_status.children[0].as_element().unwrap();
    assert_eq!(bugnumbers.name, "arg0"); // The array is passed as the first argument
    let item = bugnumbers.children[0].as_element().unwrap();
    assert_eq!(item.name, "item");
    assert_eq!(item.children[0].as_text().unwrap(), "123456");
}

#[test]
fn test_get_status_request_multiple() {
    let request = get_status_request(&[123, 456, 789]);

    let body = request.children[1].as_element().unwrap();
    let get_status = body.children[0].as_element().unwrap();
    let bugnumbers = get_status.children[0].as_element().unwrap();
    assert_eq!(bugnumbers.name, "arg0");

    let items: Vec<i32> = bugnumbers
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter_map(|e| e.children[0].as_text())
        .filter_map(|t| t.parse().ok())
        .collect();

    assert_eq!(items, vec![123, 456, 789]);
}

#[test]
fn test_get_usertag_request() {
    let request = get_usertag_request("user@example.com", &["tag1", "tag2"]);

    assert_eq!(request.name, "Envelope");
    let body = request.children[1].as_element().unwrap();
    let get_usertag = body.children[0].as_element().unwrap();
    assert_eq!(get_usertag.name, "get_usertag");

    // First arg should be email
    let email_arg = get_usertag.children[0].as_element().unwrap();
    assert_eq!(email_arg.children[0].as_text().unwrap(), "user@example.com");

    // Following args should be tags
    let tag1_arg = get_usertag.children[1].as_element().unwrap();
    assert_eq!(tag1_arg.children[0].as_text().unwrap(), "tag1");

    let tag2_arg = get_usertag.children[2].as_element().unwrap();
    assert_eq!(tag2_arg.children[0].as_text().unwrap(), "tag2");
}

#[test]
fn test_to_arg_xml_string() {
    let s = "test string";
    let elem = s.to_arg_xml("test_arg".to_string());
    assert_eq!(elem.name, "test_arg");
    assert_eq!(elem.children[0].as_text().unwrap(), "test string");
}

#[test]
fn test_to_arg_xml_str_array() {
    let v = ["a", "b"];
    let slice: &[&str] = &v;
    let elem = slice.to_arg_xml("test_array".to_string());
    assert_eq!(elem.name, "test_array");
    assert!(elem.attributes.get("xsi:type").unwrap().contains("Array"));

    let items: Vec<String> = elem
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter_map(|e| e.children[0].as_text())
        .map(|t| t.to_string())
        .collect();
    assert_eq!(items, vec!["a", "b"]);
}

#[test]
fn test_add_arg_xml() {
    let mut params = Vec::new();
    add_arg_xml(&mut params, "test value");

    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "arg0");
    assert_eq!(params[0].children[0].as_text().unwrap(), "test value");

    add_arg_xml(&mut params, "second value");
    assert_eq!(params.len(), 2);
    assert_eq!(params[1].name, "arg1");
    assert_eq!(params[1].children[0].as_text().unwrap(), "second value");
}

#[test]
fn test_parse_fault() {
    let fault_xml = r###"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <soap:Fault>
      <faultcode>Client</faultcode>
      <faultstring>Invalid request</faultstring>
      <faultactor>http://bugs.debian.org</faultactor>
      <detail>Bug ID not found</detail>
    </soap:Fault>
  </soap:Body>
</soap:Envelope>"###;

    let fault = parse_fault(fault_xml).unwrap();
    assert_eq!(fault.faultcode, "Client");
    assert_eq!(fault.faultstring, "Invalid request");
    assert_eq!(fault.faultactor, Some("http://bugs.debian.org".to_string()));
    assert_eq!(fault.detail, Some("Bug ID not found".to_string()));
}

#[test]
fn test_parse_fault_minimal() {
    let fault_xml = r###"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <soap:Fault>
      <faultcode>Server</faultcode>
      <faultstring>Internal error</faultstring>
    </soap:Fault>
  </soap:Body>
</soap:Envelope>"###;

    let fault = parse_fault(fault_xml).unwrap();
    assert_eq!(fault.faultcode, "Server");
    assert_eq!(fault.faultstring, "Internal error");
    assert_eq!(fault.faultactor, None);
    assert_eq!(fault.detail, None);
}

#[test]
fn test_parse_get_bug_log_response() {
    let xml = r###"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:soapenc="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <soap:Body>
    <get_bug_logResponse xmlns="Debbugs/SOAP">
      <soapenc:Array soapenc:arrayType="xsd:ur-type[1]" xsi:type="soapenc:Array">
        <item>
          <header>Subject: Test bug</header>
          <body>This is a test bug report.</body>
          <msg_num>1</msg_num>
        </item>
      </soapenc:Array>
    </get_bug_logResponse>
  </soap:Body>
</soap:Envelope>"###;

    let bug_log = parse_get_bug_log_response(xml).unwrap();
    assert_eq!(bug_log.len(), 1);
    assert_eq!(bug_log[0].header, "Subject: Test bug");
    assert_eq!(bug_log[0].body, "This is a test bug report.");
    assert_eq!(bug_log[0].msgnum, 1);
}

#[test]
fn test_parse_get_bugs_response() {
    let xml = r###"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:soapenc="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <soap:Body>
    <get_bugsResponse xmlns="Debbugs/SOAP">
      <soapenc:Array soapenc:arrayType="xsd:int[3]" xsi:type="soapenc:Array">
        <item xsi:type="xsd:int">123</item>
        <item xsi:type="xsd:int">456</item>
        <item xsi:type="xsd:int">789</item>
      </soapenc:Array>
    </get_bugsResponse>
  </soap:Body>
</soap:Envelope>"###;

    let bug_ids = parse_get_bugs_response(xml).unwrap();
    assert_eq!(bug_ids, vec![123, 456, 789]);
}

#[test]
fn test_parse_get_status_response() {
    let xml = r###"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <get_statusResponse xmlns="Debbugs/SOAP">
      <s-gensym3>
        <item>
          <key>123</key>
          <value>
            <pending>pending</pending>
            <severity>normal</severity>
            <package>test-package</package>
            <subject>Test subject</subject>
          </value>
        </item>
      </s-gensym3>
    </get_statusResponse>
  </soap:Body>
</soap:Envelope>"###;

    let statuses = parse_get_status_response(xml).unwrap();
    assert_eq!(statuses.len(), 1);
    assert!(statuses.contains_key(&123));
    let bug_report = &statuses[&123];
    assert_eq!(bug_report.severity, Some("normal".to_string()));
    assert_eq!(bug_report.package, Some("test-package".to_string()));
    assert_eq!(bug_report.subject, Some("Test subject".to_string()));
}

#[test]
fn test_parse_get_usertag_response() {
    let xml = r###"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <get_usertagResponse xmlns="Debbugs/SOAP">
      <s-gensym3>
        <tag1>
          <item>123</item>
          <item>456</item>
        </tag1>
        <tag2>
          <item>789</item>
        </tag2>
      </s-gensym3>
    </get_usertagResponse>
  </soap:Body>
</soap:Envelope>"###;

    let usertags = parse_get_usertag_response(xml).unwrap();
    assert_eq!(usertags.len(), 2);
    assert_eq!(usertags["tag1"], vec![123, 456]);
    assert_eq!(usertags["tag2"], vec![789]);
}

#[test]
fn test_parse_response_envelope_invalid() {
    let invalid_xml = r###"<?xml version="1.0" encoding="UTF-8"?>
<invalid>
  <body>test</body>
</invalid>"###;

    let result = parse_response_envelope(invalid_xml, "test");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Root element is not a valid soap:Envelope"));
}

#[test]
fn test_parse_bool() {
    assert_eq!(parse_bool("1").unwrap(), true);
    assert_eq!(parse_bool("0").unwrap(), false);
    assert!(parse_bool("invalid").is_err());
    assert!(parse_bool("true").is_err());
}

#[test]
fn test_bug_report_from_xml_minimal() {
    let xml_str = r###"
    <value>
        <bug_num>123456</bug_num>
        <subject>Test bug subject</subject>
        <severity>normal</severity>
        <package>test-package</package>
    </value>
    "###;

    let element = xmltree::Element::parse(xml_str.as_bytes()).unwrap();
    let bug_report = BugReport::from(&element);

    assert_eq!(bug_report.bug_num, Some(123456));
    assert_eq!(bug_report.subject, Some("Test bug subject".to_string()));
    assert_eq!(bug_report.severity, Some("normal".to_string()));
    assert_eq!(bug_report.package, Some("test-package".to_string()));
}

#[test]
fn test_bug_report_from_xml_comprehensive() {
    let xml_str = r###"
    <value>
        <bug_num>123456</bug_num>
        <subject>Test bug subject</subject>
        <severity>important</severity>
        <package>test-package</package>
        <last_modified>1234567890</last_modified>
        <tags>patch,security</tags>
        <pending>pending</pending>
        <done>fixed in version 1.2</done>
        <archived>0</archived>
        <unarchived>1</unarchived>
        <forwarded>https://example.com/bug/123</forwarded>
        <mergedwith>789 101112</mergedwith>
        <blockedby>456</blockedby>
        <blocks>789</blocks>
        <summary>Short summary of the bug</summary>
        <affects>affected-package</affects>
        <log_modified>1234567900</log_modified>
        <location>main</location>
        <source>source-package</source>
        <owner>maintainer@example.com</owner>
        <originator>user@example.com</originator>
        <msgid>&lt;message-id@example.com&gt;</msgid>
        <found_versions>
            <item>1.0</item>
            <item>1.1</item>
        </found_versions>
        <fixed_versions>
            <item>1.2</item>
        </fixed_versions>
    </value>
    "###;

    let element = xmltree::Element::parse(xml_str.as_bytes()).unwrap();
    let bug_report = BugReport::from(&element);

    assert_eq!(bug_report.bug_num, Some(123456));
    assert_eq!(bug_report.subject, Some("Test bug subject".to_string()));
    assert_eq!(bug_report.severity, Some("important".to_string()));
    assert_eq!(bug_report.package, Some("test-package".to_string()));
    assert_eq!(bug_report.last_modified, Some(1234567890));
    assert_eq!(bug_report.tags, Some("patch,security".to_string())); // Tags are stored as a comma-separated string
    assert_eq!(bug_report.pending, Some(crate::Pending::Pending));
    assert_eq!(bug_report.done, Some("fixed in version 1.2".to_string())); // done field content
    assert_eq!(bug_report.archived, Some(false));
    assert_eq!(bug_report.unarchived, Some(true));
    assert_eq!(
        bug_report.forwarded,
        Some("https://example.com/bug/123".to_string())
    );
    assert_eq!(bug_report.mergedwith, Some(vec![789, 101112]));
    assert_eq!(bug_report.blockedby, Some("456".to_string())); // Stored as string
    assert_eq!(bug_report.blocks, Some("789".to_string())); // Stored as string
    assert_eq!(
        bug_report.summary,
        Some("Short summary of the bug".to_string())
    );
    assert_eq!(bug_report.affects, Some("affected-package".to_string()));
    assert_eq!(bug_report.log_modified, Some(1234567900));
    assert_eq!(bug_report.location, Some("main".to_string()));
    assert_eq!(bug_report.source, Some("source-package".to_string()));
    assert_eq!(bug_report.owner, Some("maintainer@example.com".to_string()));
    assert_eq!(bug_report.originator, Some("user@example.com".to_string())); // submitter -> originator
    assert_eq!(
        bug_report.msgid,
        Some("<message-id@example.com>".to_string())
    );
    // Note: found_versions and fixed_versions have complex types, testing basic presence
    assert!(bug_report.found_versions.is_some());
    assert!(bug_report.fixed_versions.is_some());
}

#[test]
fn test_bug_report_from_xml_empty() {
    let xml_str = r###"<value></value>"###;

    let element = xmltree::Element::parse(xml_str.as_bytes()).unwrap();
    let bug_report = BugReport::from(&element);

    // All fields should be None for empty XML
    assert_eq!(bug_report.bug_num, None);
    assert_eq!(bug_report.subject, None);
    assert_eq!(bug_report.severity, None);
    assert_eq!(bug_report.package, None);
    assert_eq!(bug_report.last_modified, None);
    assert_eq!(bug_report.tags, None);
    assert_eq!(bug_report.pending, None);
    assert_eq!(bug_report.done, None);
    assert_eq!(bug_report.archived, None);
}

#[test]
fn test_bug_report_from_xml_invalid_values() {
    let xml_str = r###"
    <value>
        <bug_num>not-a-number</bug_num>
        <last_modified>invalid-date</last_modified>
        <archived>sometimes</archived>
        <pending>unknown-status</pending>
    </value>
    "###;

    let element = xmltree::Element::parse(xml_str.as_bytes()).unwrap();
    let bug_report = BugReport::from(&element);

    // Invalid values should result in None
    assert_eq!(bug_report.bug_num, None);
    assert_eq!(bug_report.last_modified, None);
    assert_eq!(bug_report.archived, None);
    assert_eq!(bug_report.pending, None);
    // done field accepts any string value
    assert!(bug_report.done.is_none()); // Not provided in test XML
}

#[derive(Debug)]
/// Detailed information about a bug report
///
/// Contains comprehensive metadata about a bug including its status, severity,
/// package information, and related bugs.
pub struct BugReport {
    /// The pending status of the bug
    pub pending: Option<crate::Pending>,
    /// Message ID of the initial bug report email
    pub msgid: Option<String>,
    /// Email address of the person who currently owns/is working on this bug
    pub owner: Option<String>,
    /// Keywords associated with the bug (deprecated, use `tags` instead)
    #[deprecated = "Use tags instead"]
    pub keywords: Option<String>,
    /// Packages that are affected by this bug (in addition to the primary package)
    pub affects: Option<String>,
    /// Whether the bug has been unarchived and can be archived again
    pub unarchived: Option<bool>,
    /// Email address or URL where the bug has been forwarded to upstream
    pub forwarded: Option<String>,
    /// Short summary description of the bug
    pub summary: Option<String>,
    /// The unique bug number identifier
    pub bug_num: Option<BugId>,
    /// Whether the bug has been archived (old/resolved bugs)
    pub archived: Option<bool>,
    /// Versions of the package where this bug was found to exist
    pub found_versions: Option<Vec<Version>>,
    /// Email address of the person who marked this bug as done/resolved
    pub done: Option<String>,
    /// Severity level of the bug (e.g., "serious", "important", "normal", "minor", "wishlist")
    pub severity: Option<String>,
    /// Name of the package this bug affects
    pub package: Option<String>,
    /// Versions of the package where this bug has been fixed
    pub fixed_versions: Option<Vec<(Option<String>, Option<Version>)>>,
    /// Email address of the person who originally reported this bug
    pub originator: Option<String>,
    /// Comma-separated list of bug IDs that this bug blocks
    pub blocks: Option<String>,
    /// Dates when the bug was found in specific versions (deprecated, currently empty)
    #[deprecated(note = "empty for now")]
    pub found_date: Option<Vec<u32>>,
    /// Free-form text describing the outlook for fixing this bug
    pub outlook: Option<String>,
    /// Legacy bug ID field (deprecated, use `bug_num` instead)
    #[deprecated(note = "use bug_num")]
    pub id: Option<BugId>,
    /// Whether the bug has been found in any versions
    pub found: bool,
    /// Whether the bug has been fixed in any versions
    pub fixed: bool,
    /// Unix timestamp of when the bug was last modified
    pub last_modified: Option<u32>,
    /// Space-separated list of tags associated with this bug
    pub tags: Option<String>,
    /// The title/subject line of the bug report
    pub subject: Option<String>,
    /// Physical location or context information for the bug
    pub location: Option<String>,
    /// List of bug IDs that this bug has been merged with
    pub mergedwith: Option<Vec<BugId>>,
    /// Comma-separated list of bug IDs that block this bug
    pub blockedby: Option<String>,
    /// Dates when the bug was fixed in specific versions (deprecated, currently empty)
    #[deprecated(note = "empty for now")]
    pub fixed_date: Option<Vec<u32>>,
    /// Unix timestamp of when the bug log was last modified
    pub log_modified: Option<u32>,
    /// Source package name (for binary packages built from source)
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
/// A log entry (email message) from a bug's communication history
///
/// Represents a single email message in the bug's conversation thread,
/// including the initial bug report and all subsequent correspondence.
pub struct BugLog {
    /// The email headers as a raw string (From, To, Subject, Date, etc.)
    pub header: String,
    /// Sequential message number within this bug's history
    pub msgnum: BugId,
    /// The email message body content
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
/// Search criteria for finding bugs matching specific conditions
///
/// All fields are optional - only specify the criteria you want to filter by.
/// Multiple criteria are combined with AND logic (all must match).
///
/// # Examples
///
/// ```no_run
/// use debbugs::SearchQuery;
///
/// // Find all serious bugs in the rust-debbugs package
/// let query = SearchQuery {
///     package: Some("rust-debbugs"),
///     severity: Some("serious"),
///     ..Default::default()
/// };
///
/// // Find bugs owned by a specific person
/// let query = SearchQuery {
///     owner: Some("maintainer@example.com"),
///     ..Default::default()
/// };
/// ```
pub struct SearchQuery<'a> {
    /// Package name to search for bugs in
    pub package: Option<&'a str>,
    /// Specific bug IDs to retrieve (useful for batch operations)
    pub bug_ids: Option<&'a [BugId]>,
    /// Email address of the person who submitted the bug
    pub submitter: Option<&'a str>,
    /// Email address of the package maintainer
    pub maintainer: Option<&'a str>,
    /// Source package name (for bugs affecting source packages)
    pub src: Option<&'a str>,
    /// Severity level (e.g., "critical", "serious", "important", "normal", "minor", "wishlist")
    pub severity: Option<&'a str>,
    /// Current status of the bug (open, done, forwarded)
    pub status: Option<crate::BugStatus>,
    /// Email address of the person currently owning/working on the bug
    pub owner: Option<&'a str>,
    /// Email address of someone who has participated in the bug discussion
    pub correspondent: Option<&'a str>,
    /// Whether to include archived bugs, non-archived bugs, or both
    pub archive: Option<crate::Archived>,
    /// Tags to filter by (bugs must have all specified tags)
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
