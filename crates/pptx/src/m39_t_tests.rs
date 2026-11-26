//! M39 package T: PPTX import maps `a:highlight` and an outer shadow on run properties.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use coursecraft_course::{Color, ObjectKind};

use crate::import_pptx;
use crate::m26_fidelity_tests::build_pptx;

#[test]
fn highlight_and_run_shadow_are_imported() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Text"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r><a:rPr><a:highlight><a:srgbClr val="FFFF00"/></a:highlight></a:rPr><a:t>Marked</a:t></a:r>
          <a:r><a:rPr><a:effectLst><a:outerShdw blurRad="38100" dist="38100" dir="2700000"><a:srgbClr val="000000"><a:alpha val="40000"/></a:srgbClr></a:outerShdw></a:effectLst></a:rPr><a:t>Shadowed</a:t></a:r>
          <a:r><a:t>Plain</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let bytes = build_pptx(&[slide_xml], &[]);
    let imported = import_pptx(&bytes, "Highlight").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    let runs = match &objs[0].kind {
        ObjectKind::Text { runs, .. } => runs,
        ObjectKind::Shape {
            text: Some(shape_text),
            ..
        } => &shape_text.runs,
        other => panic!("expected text, got {other:?}"),
    };
    assert_eq!(runs.len(), 3, "{runs:?}");
    assert_eq!(runs[0].highlight, Some(Color("#FFFF00".into())));
    assert!(runs[0].shadow.is_none());
    assert_eq!(runs[1].shadow, Some(true));
    assert!(runs[1].highlight.is_none());
    assert!(runs[2].is_plain());
}
