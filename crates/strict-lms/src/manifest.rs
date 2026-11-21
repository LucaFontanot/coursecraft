//! Strict package & XML manifest conformance validators for SCORM 1.2,
//! SCORM 2004 4th Edition, cmi5 (`cmi5.xml`), Tin Can (`tincan.xml`), LTI, and Web.

use std::collections::{BTreeMap, HashSet};

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde_json::Value;

use crate::{Violation, parse_plain_decimal};

/// Package standard being validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStandardKind {
    Scorm12,
    Scorm2004,
    Cmi5,
    Xapi,
    Lti,
    Web,
}

impl PackageStandardKind {
    fn marker_id(self) -> &'static str {
        match self {
            Self::Scorm12 => "scorm12",
            Self::Scorm2004 => "scorm2004",
            Self::Cmi5 => "cmi5",
            Self::Xapi => "xapi",
            Self::Lti => "lti",
            Self::Web => "web",
        }
    }
}

/// Validate a SCORM 1.2 `imsmanifest.xml` document.
///
/// If `package_files` is `Some`, also verifies every `<file href>` exists in
/// `package_files`, every file in `package_files` except `imsmanifest.xml` is
/// listed, and `imsmanifest.xml` does NOT list itself.
#[must_use]
pub fn validate_scorm12_manifest(xml: &str, package_files: Option<&[&str]>) -> Vec<Violation> {
    validate_scorm_manifest(xml, package_files, false)
}

/// Validate a SCORM 2004 4th Edition `imsmanifest.xml` document.
#[must_use]
pub fn validate_scorm2004_manifest(xml: &str, package_files: Option<&[&str]>) -> Vec<Violation> {
    validate_scorm_manifest(xml, package_files, true)
}

/// Validate a `cmi5.xml` course structure document.
#[must_use]
pub fn validate_cmi5_xml(xml: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Ok(parsed) = parse_xml(xml) else {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-XML",
            "/",
            "cmi5.xml is not well-formed XML",
        ));
        return violations;
    };

    let Some(root) = parsed.first() else {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-EMPTY",
            "/",
            "cmi5.xml has no root element",
        ));
        return violations;
    };
    if local_name(&root.name) != "courseStructure" {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-ROOT",
            "/",
            format!("expected <courseStructure>, got <{}>", root.name),
        ));
    }
    if root.attrs.get("xmlns").map(String::as_str)
        != Some("https://w3id.org/xapi/profiles/cmi5/v1/CourseStructure.xsd")
    {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-XMLNS",
            "/courseStructure/@xmlns",
            "cmi5.xml missing standard xmlns",
        ));
    }

    let course = parsed.iter().find(|e| local_name(&e.name) == "course");
    match course {
        Some(c) => {
            let id = c.attrs.get("id").map(String::as_str).unwrap_or("");
            if !is_iri(id) {
                violations.push(manifest_violation(
                    "MANIFEST-CMI5-COURSE-ID",
                    "/courseStructure/course/@id",
                    format!("course id `{id}` is not a valid IRI"),
                ));
            }
        }
        None => violations.push(manifest_violation(
            "MANIFEST-CMI5-NO-COURSE",
            "/courseStructure/course",
            "cmi5.xml missing <course>",
        )),
    }

    let aus: Vec<&XmlElem> = parsed
        .iter()
        .filter(|e| local_name(&e.name) == "au")
        .collect();
    if aus.is_empty() {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-NO-AU",
            "/courseStructure/course/au",
            "cmi5.xml must contain at least one <au>",
        ));
    }
    for au in aus {
        validate_cmi5_au(au, &parsed, &mut violations);
    }

    violations
}

fn validate_cmi5_au(au: &XmlElem, parsed: &[XmlElem], violations: &mut Vec<Violation>) {
    let id = au.attrs.get("id").map(String::as_str).unwrap_or("");
    if !is_iri(id) {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-AU-ID",
            "/courseStructure/course/au/@id",
            format!("au id `{id}` is not a valid IRI"),
        ));
    }
    let move_on = au.attrs.get("moveOn").map(String::as_str).unwrap_or("");
    if !matches!(
        move_on,
        "Passed" | "Completed" | "CompletedAndPassed" | "CompletedOrPassed" | "NotApplicable"
    ) {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-MOVEON",
            "/courseStructure/course/au/@moveOn",
            format!("invalid moveOn `{move_on}`"),
        ));
    }
    let launch_method = au
        .attrs
        .get("launchMethod")
        .map(String::as_str)
        .unwrap_or("");
    if !matches!(launch_method, "AnyWindow" | "OwnWindow") {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-LAUNCH-METHOD",
            "/courseStructure/course/au/@launchMethod",
            format!("invalid launchMethod `{launch_method}`"),
        ));
    }
    if let Some(ms) = au.attrs.get("masteryScore") {
        let ok = parse_plain_decimal(ms).is_some_and(|v| (0.0..=1.0).contains(&v));
        if !ok {
            violations.push(manifest_violation(
                "MANIFEST-CMI5-MASTERY",
                "/courseStructure/course/au/@masteryScore",
                format!("masteryScore `{ms}` must be in 0..=1"),
            ));
        }
    }
    let url = parsed
        .iter()
        .find(|e| local_name(&e.name) == "url")
        .map(|e| e.text.trim())
        .unwrap_or("");
    if !is_relative_package_path(url) {
        violations.push(manifest_violation(
            "MANIFEST-CMI5-URL",
            "/courseStructure/course/au/url",
            format!("au <url> `{url}` must be a relative path"),
        ));
    }
}

/// Validate a Tin Can `tincan.xml` manifest document.
#[must_use]
pub fn validate_tincan_xml(xml: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Ok(parsed) = parse_xml(xml) else {
        violations.push(manifest_violation(
            "MANIFEST-TINCAN-XML",
            "/",
            "tincan.xml is not well-formed XML",
        ));
        return violations;
    };

    let Some(root) = parsed.first() else {
        violations.push(manifest_violation(
            "MANIFEST-TINCAN-EMPTY",
            "/",
            "tincan.xml has no root element",
        ));
        return violations;
    };
    if local_name(&root.name) != "tincan" {
        violations.push(manifest_violation(
            "MANIFEST-TINCAN-ROOT",
            "/",
            format!("expected <tincan>, got <{}>", root.name),
        ));
    }
    if root.attrs.get("xmlns").map(String::as_str) != Some("http://projecttincan.com/tincan.xsd") {
        violations.push(manifest_violation(
            "MANIFEST-TINCAN-XMLNS",
            "/tincan/@xmlns",
            "tincan.xml missing standard xmlns",
        ));
    }

    let activities: Vec<&XmlElem> = parsed
        .iter()
        .filter(|e| local_name(&e.name) == "activity")
        .collect();
    if activities.is_empty() {
        violations.push(manifest_violation(
            "MANIFEST-TINCAN-NO-ACTIVITY",
            "/tincan/activities/activity",
            "tincan.xml must contain at least one <activity>",
        ));
    }
    for act in activities {
        let id = act.attrs.get("id").map(String::as_str).unwrap_or("");
        if !is_iri(id) {
            violations.push(manifest_violation(
                "MANIFEST-TINCAN-ACTIVITY-ID",
                "/tincan/activities/activity/@id",
                format!("activity id `{id}` is not a valid IRI"),
            ));
        }
    }
    let launch = parsed
        .iter()
        .find(|e| local_name(&e.name) == "launch")
        .map(|e| e.text.trim())
        .unwrap_or("");
    if !is_relative_package_path(launch) {
        violations.push(manifest_violation(
            "MANIFEST-TINCAN-LAUNCH",
            "/tincan/activities/activity/launch",
            format!("launch `{launch}` must be a relative path"),
        ));
    }

    violations
}

/// Validate a complete published package (represented as `(relative_path, bytes)`).
#[must_use]
pub fn validate_package(standard: PackageStandardKind, files: &[(&str, &[u8])]) -> Vec<Violation> {
    let mut violations = Vec::new();
    let by_path: BTreeMap<&str, &[u8]> = files.iter().copied().collect();
    let file_paths: Vec<&str> = files.iter().map(|(p, _)| *p).collect();

    if standard != PackageStandardKind::Scorm12 {
        match by_path.get("coursecraft-package.json") {
            Some(bytes) => {
                if let Ok(val) = serde_json::from_slice::<Value>(bytes) {
                    let got = val.get("standard").and_then(Value::as_str).unwrap_or("");
                    if got != standard.marker_id() {
                        violations.push(manifest_violation(
                            "PKG-MARKER-STANDARD",
                            "coursecraft-package.json",
                            format!("expected standard `{}`, got `{got}`", standard.marker_id()),
                        ));
                    }
                } else {
                    violations.push(manifest_violation(
                        "PKG-MARKER-JSON",
                        "coursecraft-package.json",
                        "coursecraft-package.json is not valid JSON",
                    ));
                }
            }
            None => violations.push(manifest_violation(
                "PKG-MARKER-MISSING",
                "coursecraft-package.json",
                "missing coursecraft-package.json",
            )),
        }
    }

    match standard {
        PackageStandardKind::Scorm12 | PackageStandardKind::Scorm2004 => {
            if let Some(bytes) = by_path.get("imsmanifest.xml") {
                if let Ok(xml) = std::str::from_utf8(bytes) {
                    let mut v = if standard == PackageStandardKind::Scorm12 {
                        validate_scorm12_manifest(xml, Some(&file_paths))
                    } else {
                        validate_scorm2004_manifest(xml, Some(&file_paths))
                    };
                    violations.append(&mut v);
                } else {
                    violations.push(manifest_violation(
                        "MANIFEST-UTF8",
                        "imsmanifest.xml",
                        "imsmanifest.xml is not valid UTF-8",
                    ));
                }
            } else {
                violations.push(manifest_violation(
                    "MANIFEST-MISSING",
                    "imsmanifest.xml",
                    "missing imsmanifest.xml",
                ));
            }
        }
        PackageStandardKind::Cmi5 => {
            if by_path.contains_key("imsmanifest.xml") {
                violations.push(manifest_violation(
                    "PKG-UNEXPECTED-IMSMANIFEST",
                    "imsmanifest.xml",
                    "cmi5 package must not contain imsmanifest.xml",
                ));
            }
            match by_path
                .get("cmi5.xml")
                .and_then(|b| std::str::from_utf8(b).ok())
            {
                Some(xml) => violations.extend(validate_cmi5_xml(xml)),
                None => violations.push(manifest_violation(
                    "MANIFEST-CMI5-MISSING",
                    "cmi5.xml",
                    "missing or invalid UTF-8 cmi5.xml",
                )),
            }
        }
        PackageStandardKind::Xapi => {
            if by_path.contains_key("imsmanifest.xml") {
                violations.push(manifest_violation(
                    "PKG-UNEXPECTED-IMSMANIFEST",
                    "imsmanifest.xml",
                    "xAPI package must not contain imsmanifest.xml",
                ));
            }
            match by_path
                .get("tincan.xml")
                .and_then(|b| std::str::from_utf8(b).ok())
            {
                Some(xml) => violations.extend(validate_tincan_xml(xml)),
                None => violations.push(manifest_violation(
                    "MANIFEST-TINCAN-MISSING",
                    "tincan.xml",
                    "missing or invalid UTF-8 tincan.xml",
                )),
            }
        }
        PackageStandardKind::Lti | PackageStandardKind::Web => {
            for forbidden in ["imsmanifest.xml", "cmi5.xml", "tincan.xml"] {
                if by_path.contains_key(forbidden) {
                    violations.push(manifest_violation(
                        "PKG-UNEXPECTED-MANIFEST",
                        forbidden,
                        format!(
                            "{} package must not contain {forbidden}",
                            standard.marker_id()
                        ),
                    ));
                }
            }
        }
    }

    if let Some(course_bytes) = by_path.get("course.json")
        && let Ok(course_val) = serde_json::from_slice::<Value>(course_bytes)
        && course_val
            .get("templates")
            .and_then(Value::as_array)
            .is_some_and(|arr| !arr.is_empty())
    {
        violations.push(manifest_violation(
            "PKG-TEMPLATES-NOT-STRIPPED",
            "course.json",
            "published course.json must strip templates",
        ));
    }

    violations
}

fn validate_scorm_manifest(
    xml: &str,
    package_files: Option<&[&str]>,
    is_2004: bool,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Ok(parsed) = parse_xml(xml) else {
        violations.push(manifest_violation(
            "MANIFEST-SCORM-XML",
            "/",
            "imsmanifest.xml is not well-formed XML",
        ));
        return violations;
    };

    let Some(root) = parsed.first() else {
        violations.push(manifest_violation(
            "MANIFEST-SCORM-EMPTY",
            "/",
            "imsmanifest.xml has no root element",
        ));
        return violations;
    };

    if local_name(&root.name) != "manifest" {
        violations.push(manifest_violation(
            "MANIFEST-SCORM-ROOT",
            "/",
            format!("expected <manifest>, got <{}>", root.name),
        ));
    }

    let expected_xmlns = if is_2004 {
        "http://www.imsglobal.org/xsd/imscp_v1p1"
    } else {
        "http://www.imsproject.org/xsd/imscp_rootv1p1p2"
    };
    let expected_adlcp = if is_2004 {
        "http://www.adlnet.org/xsd/adlcp_v1p3"
    } else {
        "http://www.adlnet.org/xsd/adlcp_rootv1p2"
    };
    if root.attrs.get("xmlns").map(String::as_str) != Some(expected_xmlns) {
        violations.push(manifest_violation(
            "MANIFEST-SCORM-XMLNS",
            "/manifest/@xmlns",
            format!("expected xmlns `{expected_xmlns}`"),
        ));
    }
    if root.attrs.get("xmlns:adlcp").map(String::as_str) != Some(expected_adlcp) {
        violations.push(manifest_violation(
            "MANIFEST-SCORM-ADLCP-XMLNS",
            "/manifest/@xmlns:adlcp",
            format!("expected xmlns:adlcp `{expected_adlcp}`"),
        ));
    }

    let root_id = root
        .attrs
        .get("identifier")
        .map(String::as_str)
        .unwrap_or("");
    if !is_xml_ncname(root_id) {
        violations.push(manifest_violation(
            "MANIFEST-NCNAME",
            "/manifest/@identifier",
            format!("manifest identifier `{root_id}` is not a valid XML NCName"),
        ));
    }

    let schema_ver = parsed
        .iter()
        .find(|e| local_name(&e.name) == "schemaversion")
        .map(|e| e.text.trim())
        .unwrap_or("");
    let expected_ver = if is_2004 { "2004 4th Edition" } else { "1.2" };
    if schema_ver != expected_ver {
        violations.push(manifest_violation(
            "MANIFEST-SCHEMAVERSION",
            "/manifest/metadata/schemaversion",
            format!("expected schemaversion `{expected_ver}`, got `{schema_ver}`"),
        ));
    }

    if is_2004 {
        if parsed.iter().any(|e| local_name(&e.name) == "masteryscore") {
            violations.push(manifest_violation(
                "MANIFEST-2004-MASTERYSCORE",
                "/manifest//adlcp:masteryscore",
                "SCORM 2004 manifest must not contain adlcp:masteryscore",
            ));
        }
        if parsed.iter().any(|e| local_name(&e.name) == "sequencing") {
            violations.push(manifest_violation(
                "MANIFEST-2004-SEQUENCING",
                "/manifest//imsss:sequencing",
                "CourseCraft SCORM 2004 manifest must not contain imsss:sequencing",
            ));
        }
    }

    validate_scorm_orgs_and_resources(&parsed, is_2004, &mut violations);
    if let Some(pkg_files) = package_files {
        validate_scorm_file_list(&parsed, pkg_files, &mut violations);
    }

    violations
}

fn validate_scorm_orgs_and_resources(
    parsed: &[XmlElem],
    is_2004: bool,
    violations: &mut Vec<Violation>,
) {
    let org_ids: HashSet<&str> = parsed
        .iter()
        .filter(|e| local_name(&e.name) == "organization")
        .filter_map(|e| e.attrs.get("identifier").map(String::as_str))
        .collect();
    if let Some(orgs) = parsed
        .iter()
        .find(|e| local_name(&e.name) == "organizations")
    {
        let def = orgs.attrs.get("default").map(String::as_str).unwrap_or("");
        if !org_ids.contains(def) {
            violations.push(manifest_violation(
                "MANIFEST-ORG-DEFAULT",
                "/manifest/organizations/@default",
                format!("organizations default `{def}` does not match any <organization>"),
            ));
        }
    }
    for org_id in &org_ids {
        if !is_xml_ncname(org_id) {
            violations.push(manifest_violation(
                "MANIFEST-NCNAME",
                "/manifest/organizations/organization/@identifier",
                format!("organization identifier `{org_id}` is not a valid XML NCName"),
            ));
        }
    }

    let res_ids: HashSet<&str> = parsed
        .iter()
        .filter(|e| local_name(&e.name) == "resource")
        .filter_map(|e| e.attrs.get("identifier").map(String::as_str))
        .collect();
    for item in parsed.iter().filter(|e| local_name(&e.name) == "item") {
        let item_id = item
            .attrs
            .get("identifier")
            .map(String::as_str)
            .unwrap_or("");
        if !is_xml_ncname(item_id) {
            violations.push(manifest_violation(
                "MANIFEST-NCNAME",
                "/manifest/organizations/organization/item/@identifier",
                format!("item identifier `{item_id}` is not a valid XML NCName"),
            ));
        }
        let iref = item
            .attrs
            .get("identifierref")
            .map(String::as_str)
            .unwrap_or("");
        if !res_ids.contains(iref) {
            violations.push(manifest_violation(
                "MANIFEST-ITEM-REF",
                "/manifest/organizations/organization/item/@identifierref",
                format!("item identifierref `{iref}` does not match any <resource>"),
            ));
        }
    }

    for res in parsed.iter().filter(|e| local_name(&e.name) == "resource") {
        let res_id = res
            .attrs
            .get("identifier")
            .map(String::as_str)
            .unwrap_or("");
        if !is_xml_ncname(res_id) {
            violations.push(manifest_violation(
                "MANIFEST-NCNAME",
                "/manifest/resources/resource/@identifier",
                format!("resource identifier `{res_id}` is not a valid XML NCName"),
            ));
        }
        let scorm_type_attr = if is_2004 {
            "adlcp:scormType"
        } else {
            "adlcp:scormtype"
        };
        if res.attrs.get(scorm_type_attr).map(String::as_str) != Some("sco") {
            violations.push(manifest_violation(
                "MANIFEST-SCORMTYPE",
                "/manifest/resources/resource/@adlcp:scormtype",
                format!("expected {scorm_type_attr}=\"sco\""),
            ));
        }
        let href = res.attrs.get("href").map(String::as_str).unwrap_or("");
        if !is_relative_package_path(href) {
            violations.push(manifest_violation(
                "MANIFEST-RESOURCE-HREF",
                "/manifest/resources/resource/@href",
                format!("resource href `{href}` must be a relative path"),
            ));
        }
    }
}

fn validate_scorm_file_list(
    parsed: &[XmlElem],
    package_files: &[&str],
    violations: &mut Vec<Violation>,
) {
    let listed: Vec<&str> = parsed
        .iter()
        .filter(|e| local_name(&e.name) == "file")
        .filter_map(|e| e.attrs.get("href").map(String::as_str))
        .collect();

    if listed.contains(&"imsmanifest.xml") {
        violations.push(manifest_violation(
            "MANIFEST-SELF-LISTED",
            "/manifest/resources/resource/file/@href",
            "imsmanifest.xml must not list itself as a <file href>",
        ));
    }

    let pkg_set: HashSet<&str> = package_files.iter().copied().collect();
    let listed_set: HashSet<&str> = listed.iter().copied().collect();

    for href in &listed {
        if !pkg_set.contains(href) {
            violations.push(manifest_violation(
                "MANIFEST-MISSING-FILE",
                "/manifest/resources/resource/file/@href",
                format!("manifest lists `{href}` which is missing from package"),
            ));
        }
    }
    for file in package_files {
        if *file != "imsmanifest.xml" && !listed_set.contains(file) {
            violations.push(manifest_violation(
                "MANIFEST-UNLISTED-FILE",
                "/manifest/resources/resource",
                format!("package file `{file}` is not listed in imsmanifest.xml"),
            ));
        }
    }
}

struct XmlElem {
    name: String,
    attrs: BTreeMap<String, String>,
    text: String,
}

fn parse_xml(xml: &str) -> Result<Vec<XmlElem>, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut elems: Vec<XmlElem> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let elem = build_elem(&e, &reader)?;
                elems.push(elem);
                stack.push(elems.len() - 1);
            }
            Event::Empty(e) => {
                let elem = build_elem(&e, &reader)?;
                elems.push(elem);
            }
            Event::Text(e) => {
                if let Some(&idx) = stack.last()
                    && let Ok(txt) = e.xml_content(quick_xml::XmlVersion::Implicit1_0)
                    && let Some(elem) = elems.get_mut(idx)
                {
                    elem.text.push_str(&txt);
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(elems)
}

fn build_elem(e: &BytesStart<'_>, _reader: &Reader<&[u8]>) -> Result<XmlElem, quick_xml::Error> {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut attrs = BTreeMap::new();
    for attr in e.attributes() {
        let attr = attr?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let val = attr
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map(std::borrow::Cow::into_owned)
            .unwrap_or_default();
        attrs.insert(key, val);
    }
    Ok(XmlElem {
        name,
        attrs,
        text: String::new(),
    })
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

fn is_xml_ncname(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn is_iri(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && !rest.is_empty()
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
        && !s.chars().any(char::is_whitespace)
}

fn is_relative_package_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\\')
        && !p.contains("://")
        && p.split('/').all(|seg| !seg.is_empty() && seg != "..")
}

fn manifest_violation(rule: &'static str, element: &str, message: impl Into<String>) -> Violation {
    Violation {
        standard: "manifest",
        rule,
        call_index: 0,
        element: element.to_owned(),
        message: message.into(),
        error_code: None,
    }
}
