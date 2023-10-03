use lazy_regex::regex_is_match;
use maplit::hashmap;

use xmltree::{Element, XMLNode};

pub const SOAP_ENCODING: &str = "http://www.w3.org/2003/05/soap-encoding";
pub const XMLNS_SOAP: &str = "http://xml.apache.org/xml-soap";
pub const XMLNS_SOAPENV: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const XMLNS_SOAPENC: &str = "http://schemas.xmlsoap.org/soap/encoding/";

fn build_request_envelope(name: &str, arguments: Vec<Element>) -> xmltree::Element {
    let mut namespace = xmltree::Namespace::empty();
    namespace.put("soapenv", XMLNS_SOAPENV);
    namespace.put("tns", XMLNS_SOAP);

    Element {
        name: "Envelope".to_string(),
        prefix: Some("soapenv".to_string()),
        namespaces: Some(namespace.clone()),
        namespace: Some(XMLNS_SOAPENV.to_string()),
        attributes: hashmap! {
            "soapenv:encodingStyle".to_string() => SOAP_ENCODING.to_string(),

        },
        children: vec![XMLNode::Element(Element {
            name: "Body".to_string(),
            prefix: Some("soapenv".to_string()),
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
        })],
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

#[test]
fn test_newest_bufs_request_serialize() {
    let request = newest_bugs_request(10);
    assert_eq!(request.name, "Envelope");
    assert_eq!(request.namespace.as_deref(), Some(XMLNS_SOAPENV));
    assert_eq!(
        request.attributes.get("soapenv:encodingStyle"),
        Some(&SOAP_ENCODING.to_string())
    );
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
            if !regex_is_match!(r"xsd:int\[[0-9]+\]", value) {
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
    let text = r###"<?xml version="1.0" encoding="UTF-8"?><soap:Envelope soap:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/" xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:soapenc="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><soap:Body><newest_bugsResponse xmlns="Debbugs/SOAP"><soapenc:Array soapenc:arrayType="xsd:int[10]" xsi:type="soapenc:Array"><item xsi:type="xsd:int">66320</item><item xsi:type="xsd:int">66321</item><item xsi:type="xsd:int">66322</item><item xsi:type="xsd:int">66323</item><item xsi:type="xsd:int">66324</item><item xsi:type="xsd:int">66325</item><item xsi:type="xsd:int">66326</item><item xsi:type="xsd:int">66327</item><item xsi:type="xsd:int">66328</item><item xsi:type="xsd:int">66329</item></soapenc:Array></newest_bugsResponse></soap:Body></soap:Envelope>"###;
    let integers = parse_newest_bugs_response(text).unwrap();
    assert_eq!(
        integers,
        vec![66320, 66321, 66322, 66323, 66324, 66325, 66326, 66327, 66328, 66329]
    );
}
