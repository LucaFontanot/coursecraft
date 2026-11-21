//! Strict manifest and package structure tests (Cases A5.1–A5.4).

use coursecraft_scorm::{ManifestSpec, manifest_scorm12, manifest_scorm2004};
use coursecraft_strict_lms::{
    PackageStandardKind, validate_cmi5_xml, validate_package, validate_scorm12_manifest,
    validate_scorm2004_manifest, validate_tincan_xml,
};
use coursecraft_xapi::{PackageSpec, cmi5_xml, tincan_xml};

#[test]
fn a5_1_scorm12_manifest_catches_self_listing_and_bad_ncname() {
    let spec = ManifestSpec {
        identifier: "coursecraft.hello".to_owned(),
        title: "Hello".to_owned(),
        launch: "index.html".to_owned(),
        files: vec!["index.html".to_owned(), "course.json".to_owned()],
        mastery_score: Some(80),
    };
    let xml = manifest_scorm12(&spec).expect("scorm12 xml");
    let pkg_files = ["index.html", "course.json", "imsmanifest.xml"];
    let v = validate_scorm12_manifest(&xml, Some(&pkg_files));
    assert!(v.is_empty(), "expected 0 violations, got: {v:?}");

    // Exposes BUG-A7: imsmanifest.xml listing itself.
    let mut bad_spec = spec.clone();
    bad_spec.files.push("imsmanifest.xml".to_owned());
    let bad_xml = manifest_scorm12(&bad_spec).expect("bad xml");
    let v_self = validate_scorm12_manifest(&bad_xml, Some(&pkg_files));
    assert!(v_self.iter().any(|x| x.rule == "MANIFEST-SELF-LISTED"));

    // Exposes BUG-A7: leading digit in identifier is invalid XML NCName.
    let mut digit_spec = spec;
    digit_spec.identifier = "123-course".to_owned();
    let digit_xml = manifest_scorm12(&digit_spec).expect("digit xml");
    let v_id = validate_scorm12_manifest(&digit_xml, Some(&pkg_files));
    assert!(v_id.iter().any(|x| x.rule == "MANIFEST-NCNAME"));
}

#[test]
fn a5_2_scorm2004_manifest_validates_cleanly() {
    let spec = ManifestSpec {
        identifier: "coursecraft.showcase".to_owned(),
        title: "Showcase".to_owned(),
        launch: "index.html".to_owned(),
        files: vec![
            "index.html".to_owned(),
            "course.json".to_owned(),
            "coursecraft-package.json".to_owned(),
        ],
        mastery_score: Some(80),
    };
    let xml = manifest_scorm2004(&spec).expect("2004 xml");
    let files: Vec<(&str, &[u8])> = vec![
        ("index.html", b"<html/>"),
        ("course.json", b"{}"),
        ("coursecraft-package.json", br#"{"standard":"scorm2004"}"#),
        ("imsmanifest.xml", xml.as_bytes()),
    ];
    let v = validate_package(PackageStandardKind::Scorm2004, &files);
    assert!(v.is_empty(), "expected 0 violations, got: {v:?}");

    // Rejects missing marker or masteryscore leak.
    let v_direct = validate_scorm2004_manifest(
        &xml.replace(
            "</metadata>",
            "</metadata><adlcp:masteryscore>80</adlcp:masteryscore>",
        ),
        None,
    );
    assert!(
        v_direct
            .iter()
            .any(|x| x.rule == "MANIFEST-2004-MASTERYSCORE")
    );
}

#[test]
fn a5_3_cmi5_and_tincan_xml_validate_cleanly() {
    let spec = PackageSpec {
        course_id: "hello-course".to_owned(),
        title: "Hello Course".to_owned(),
        mastery_score: Some(80),
    };
    let cmi5 = cmi5_xml(&spec);
    assert!(validate_cmi5_xml(&cmi5).is_empty());

    let tincan = tincan_xml(&spec);
    assert!(validate_tincan_xml(&tincan).is_empty());

    let cmi5_files: Vec<(&str, &[u8])> = vec![
        ("index.html", b"<html/>"),
        ("coursecraft-package.json", br#"{"standard":"cmi5"}"#),
        ("cmi5.xml", cmi5.as_bytes()),
    ];
    assert!(validate_package(PackageStandardKind::Cmi5, &cmi5_files).is_empty());

    let xapi_files: Vec<(&str, &[u8])> = vec![
        ("index.html", b"<html/>"),
        ("coursecraft-package.json", br#"{"standard":"xapi"}"#),
        ("tincan.xml", tincan.as_bytes()),
    ];
    assert!(validate_package(PackageStandardKind::Xapi, &xapi_files).is_empty());
}

#[test]
fn a5_4_lti_and_web_packages_forbid_manifests_and_templates() {
    let web_files: Vec<(&str, &[u8])> = vec![
        ("index.html", b"<html/>"),
        ("coursecraft-package.json", br#"{"standard":"web"}"#),
        ("course.json", br#"{"id":"c1"}"#),
    ];
    assert!(validate_package(PackageStandardKind::Web, &web_files).is_empty());

    let lti_bad: Vec<(&str, &[u8])> = vec![
        ("index.html", b"<html/>"),
        ("coursecraft-package.json", br#"{"standard":"lti"}"#),
        ("course.json", br#"{"id":"c1","templates":[{"id":"t1"}]}"#),
        ("imsmanifest.xml", b"<manifest/>"),
    ];
    let v = validate_package(PackageStandardKind::Lti, &lti_bad);
    assert!(v.iter().any(|x| x.rule == "PKG-UNEXPECTED-MANIFEST"));
    assert!(v.iter().any(|x| x.rule == "PKG-TEMPLATES-NOT-STRIPPED"));
}
