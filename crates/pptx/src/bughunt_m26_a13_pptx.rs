//! M26 bug-hunt tests for A13.4–A13.8: PowerPoint import constructs & hostile inputs.
//!
//! Ordered P1 -> P2 -> P3.

use crate::import_pptx;
use coursecraft_course::{FillStyle, ObjectKind, Severity};
use std::io::Write as _;

fn build_pptx_custom(
    slides: &[&str],
    slide_rels: &[(usize, &str)],
    layout_xml: Option<&str>,
    master_xml: Option<&str>,
    presentation_size: Option<(u32, u32)>,
    extra: &[(&str, &[u8])],
) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default();

        let (cx, cy) = presentation_size.unwrap_or((12_192_000, 6_858_000));
        let mut sld_ids = String::new();
        let mut pres_rels = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdM1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdT1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#,
        );
        for i in 0..slides.len() {
            let n = i + 1;
            sld_ids.push_str(&format!(r#"<p:sldId id="{}" r:id="rIdS{n}"/>"#, 255 + n));
            pres_rels.push_str(&format!(
                r#"<Relationship Id="rIdS{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{n}.xml"/>"#
            ));
        }
        pres_rels.push_str("</Relationships>");

        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/></Types>"#).unwrap();

        w.start_file("_rels/.rels", opts).unwrap();
        w.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();

        w.start_file("ppt/presentation.xml", opts).unwrap();
        w.write_all(
            format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rIdM1"/></p:sldMasterIdLst>
  <p:sldIdLst>{sld_ids}</p:sldIdLst>
  <p:sldSz cx="{cx}" cy="{cy}"/>
</p:presentation>"#
            )
            .as_bytes(),
        )
        .unwrap();

        w.start_file("ppt/_rels/presentation.xml.rels", opts)
            .unwrap();
        w.write_all(pres_rels.as_bytes()).unwrap();

        w.start_file("ppt/theme/theme1.xml", opts).unwrap();
        w.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="C">
      <a:dk1><a:srgbClr val="111111"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="222222"/></a:dk2>
      <a:lt2><a:srgbClr val="EEEEEE"/></a:lt2>
      <a:accent1><a:srgbClr val="E91E63"/></a:accent1>
      <a:accent2><a:srgbClr val="00BCD4"/></a:accent2>
      <a:accent3><a:srgbClr val="FFC107"/></a:accent3>
      <a:accent4><a:srgbClr val="673AB7"/></a:accent4>
      <a:accent5><a:srgbClr val="4CAF50"/></a:accent5>
      <a:accent6><a:srgbClr val="FF5722"/></a:accent6>
      <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
      <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="F"><a:majorFont><a:latin typeface="Arial"/></a:majorFont><a:minorFont><a:latin typeface="Arial"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="Fmt"/>
  </a:themeElements>
</a:theme>"#,
        )
        .unwrap();

        let default_master = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rIdL1"/></p:sldLayoutIdLst>
</p:sldMaster>"#;
        w.start_file("ppt/slideMasters/slideMaster1.xml", opts)
            .unwrap();
        w.write_all(master_xml.unwrap_or(default_master).as_bytes())
            .unwrap();
        w.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", opts)
            .unwrap();
        w.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdL1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#).unwrap();

        let default_layout = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldLayout>"#;
        w.start_file("ppt/slideLayouts/slideLayout1.xml", opts)
            .unwrap();
        w.write_all(layout_xml.unwrap_or(default_layout).as_bytes())
            .unwrap();
        w.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", opts)
            .unwrap();
        w.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdM1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#).unwrap();

        for (i, xml) in slides.iter().enumerate() {
            let n = i + 1;
            w.start_file(format!("ppt/slides/slide{n}.xml"), opts)
                .unwrap();
            w.write_all(xml.as_bytes()).unwrap();
            let extra_rels = slide_rels
                .iter()
                .find(|(idx, _)| *idx == n)
                .map(|(_, s)| *s)
                .unwrap_or("");
            w.start_file(format!("ppt/slides/_rels/slide{n}.xml.rels"), opts)
                .unwrap();
            w.write_all(
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdL1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  {extra_rels}
</Relationships>"#
                )
                .as_bytes(),
            )
            .unwrap();
        }

        for (path, bytes) in extra {
            w.start_file(*path, opts).unwrap();
            w.write_all(bytes).unwrap();
        }
        w.finish().unwrap();
    }
    buf
}

fn png_4x4() -> Vec<u8> {
    // Minimal valid 1x1 RGBA PNG
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8,
        0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D, 0x1D, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a13_4_canva_style_constructs_nested_groups_custgeom_blipfill_bgref() {
    let slide1 = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg><p:bgRef idx="1001"><a:schemeClr val="accent1"/></p:bgRef></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="PicShape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="914400" y="914400"/><a:ext cx="1828800" cy="1828800"/></a:xfrm>
          <a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>
          <a:blipFill><a:blip r:embed="rIdImg1"/><a:stretch><a:fillRect/></a:stretch></a:blipFill>
        </p:spPr>
      </p:sp>
      <p:grpSp>
        <p:nvGrpSpPr><p:cNvPr id="3" name="OuterGrp"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
        <p:grpSpPr>
          <a:xfrm rot="1800000" flipH="1">
            <a:off x="3657600" y="914400"/><a:ext cx="2743200" cy="1828800"/>
            <a:chOff x="1000" y="1000"/><a:chExt cx="2000" cy="2000"/>
          </a:xfrm>
        </p:grpSpPr>
        <p:cxnSp>
          <p:nvCxnSpPr><p:cNvPr id="4" name="Line"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
          <p:spPr>
            <a:xfrm><a:off x="1000" y="1000"/><a:ext cx="1000" cy="1000"/></a:xfrm>
            <a:prstGeom prst="line"><a:avLst/></a:prstGeom>
            <a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln>
          </p:spPr>
        </p:cxnSp>
      </p:grpSp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="5" name="Cust"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="914400" y="3657600"/><a:ext cx="1828800" cy="914400"/></a:xfrm>
          <a:custGeom>
            <a:pathLst><a:path w="100" h="100" fill="none">
              <a:moveTo><a:pt x="0" y="50"/></a:moveTo>
              <a:cubicBezTo><a:pt x="30" y="0"/><a:pt x="70" y="100"/><a:pt x="100" y="50"/></a:cubicBezTo>
            </a:path></a:pathLst>
          </a:custGeom>
          <a:ln w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:ln>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    let bytes = build_pptx_custom(
        &[slide1],
        &[(
            1,
            r#"<Relationship Id="rIdImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/img1.png"/>"#,
        )],
        None,
        None,
        None,
        &[("ppt/media/img1.png", &png_4x4())],
    );
    let imp = import_pptx(&bytes, "Canva").expect("import");
    let slide = &imp.course.scenes[0].slides[0];
    assert_eq!(
        slide.background.as_ref().map(|c| c.0.as_str()),
        Some("#E91E63"),
        "bgRef accent1 should resolve"
    );
    assert!(slide.objects.iter().any(|o| matches!(
        &o.kind,
        ObjectKind::Shape {
            fill_style: Some(FillStyle::Picture { .. }),
            ..
        }
    )));
    assert!(
        slide
            .objects
            .iter()
            .any(|o| matches!(&o.kind, ObjectKind::Path { .. }))
    );
    for o in &slide.objects {
        assert!(o.frame.x.is_finite() && o.frame.y.is_finite());
        assert!(o.frame.w <= 1280.0 && o.frame.h <= 720.0);
    }
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a13_5_master_and_layout_inheritance_cascade() {
    let master = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:srgbClr val="223344"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="MasterLogo"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="FFAA00"/></a:solidFill></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rIdL1"/></p:sldLayoutIdLst>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr><a:defRPr sz="4000"><a:solidFill><a:srgbClr val="FFEEDD"/></a:solidFill></a:defRPr></a:lvl1pPr>
    </p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let slide1 = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="914400" y="914400"/><a:ext cx="5486400" cy="914400"/></a:xfrm></p:spPr>
      <p:txBody><a:bodyPr/><a:p><a:r><a:t>Hello</a:t></a:r></a:p></p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let bytes = build_pptx_custom(&[slide1], &[], None, Some(master), None, &[]);
    let imp = import_pptx(&bytes, "Master").expect("import");
    let slide = &imp.course.scenes[0].slides[0];
    assert_eq!(
        slide.background.as_ref().map(|c| c.0.as_str()),
        Some("#223344")
    );
    assert!(slide.objects.len() >= 2, "expected master logo + title");
}

#[test]
fn a13_6_color_map_overrides_on_dark_themed_slide() {
    let slide1 = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:schemeClr val="bg1"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="T"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="914400" y="914400"/><a:ext cx="3657600" cy="914400"/></a:xfrm></p:spPr>
        <p:txBody><a:bodyPr/><a:p><a:r><a:rPr><a:solidFill><a:schemeClr val="tx1"/></a:solidFill></a:rPr><a:t>Dark</a:t></a:r></a:p></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:overrideClrMapping bg1="dk1" tx1="lt1" bg2="dk2" tx2="lt2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:clrMapOvr>
</p:sld>"#;
    let bytes = build_pptx_custom(&[slide1], &[], None, None, None, &[]);
    let imp = import_pptx(&bytes, "ClrMap").expect("import");
    let slide = &imp.course.scenes[0].slides[0];
    assert_eq!(
        slide.background.as_ref().map(|c| c.0.as_str()),
        Some("#111111")
    );
}

#[test]
fn a13_7_non_16_9_slide_sizes_4_3_and_9_16() {
    for (cx, cy, expected_ratio) in [
        (9_144_000, 6_858_000, 4.0 / 3.0),
        (5_143_500, 9_144_000, 9.0 / 16.0),
    ] {
        let slide1 = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Sq"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="336699"/></a:solidFill></p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#,
            (cx - 914_400) / 2,
            (cy - 914_400) / 2
        );
        let bytes = build_pptx_custom(&[&slide1], &[], None, None, Some((cx, cy)), &[]);
        let imp = import_pptx(&bytes, "Ratio").expect("import");
        let sz = imp.course.slide_size;
        let ratio = sz.w as f64 / sz.h as f64;
        assert!((ratio - expected_ratio).abs() < 0.02, "ratio={ratio}");
        let sq = &imp.course.scenes[0].slides[0].objects[0];
        assert!((sq.frame.w - sq.frame.h).abs() < 1.0);
        assert!(
            !imp.course
                .validate()
                .iter()
                .any(|i| i.severity == Severity::Error)
        );
    }
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a13_8_hostile_and_malformed_pptx_inputs_never_panic() {
    // 1. Non-zip / truncated bytes
    assert!(import_pptx(b"not a zip", "Bad").is_err());

    // 2. Group with chExt=0, huge EMU, and path traversal rel
    let hostile_slide = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:grpSp>
      <p:nvGrpSpPr><p:cNvPr id="2" name="ZeroGrp"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000" cy="1000"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Child"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="10" y="10"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
      </p:sp>
    </p:grpSp>
    <p:pic>
      <p:nvPicPr><p:cNvPr id="4" name="EvilPic"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
      <p:blipFill><a:blip r:embed="rIdEvil"/></p:blipFill>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm></p:spPr>
    </p:pic>
  </p:spTree></p:cSld>
</p:sld>"#;
    let bytes = build_pptx_custom(
        &[hostile_slide],
        &[(
            1,
            r#"<Relationship Id="rIdEvil" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../../etc/passwd"/>"#,
        )],
        None,
        None,
        None,
        &[],
    );
    if let Ok(imp) = import_pptx(&bytes, "Hostile") {
        assert!(
            !imp.course
                .validate()
                .iter()
                .any(|i| i.severity == Severity::Error)
        );
    }
}
