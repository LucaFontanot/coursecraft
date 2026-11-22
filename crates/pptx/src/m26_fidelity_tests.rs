//! Tests for M26 PPTX import fidelity fixes (backgrounds, theme colours,
//! placeholder inheritance, text features, bullets/numbering, frame clamping).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write as _;

use coursecraft_course::{
    Baseline, Color, FillStyle, ListKind, MAX_OBJECT_FRAME_SIZE, NumberScheme, ObjectKind,
    PictureFillMode, VAlign,
};

use crate::{SkipKind, import_pptx};

const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn build_pptx(slides: &[&str], extra: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        let mut sld_ids = String::new();
        for i in 0..slides.len() {
            let rid = format!("rId{}", i + 1);
            let id = 256 + i;
            sld_ids.push_str(&format!(r#"<p:sldId id="{id}" r:id="{rid}"/>"#));
        }
        let pres_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>{sld_ids}</p:sldIdLst>
  <p:sldSz cx="9144000" cy="5143500"/>
</p:presentation>"#
        );
        zip.start_file("ppt/presentation.xml", options).unwrap();
        zip.write_all(pres_xml.as_bytes()).unwrap();

        let mut rels = String::new();
        for i in 0..slides.len() {
            let rid = format!("rId{}", i + 1);
            let num = i + 1;
            rels.push_str(&format!(
                r#"<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{num}.xml"/>"#
            ));
        }
        let pres_rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#
        );
        zip.start_file("ppt/_rels/presentation.xml.rels", options)
            .unwrap();
        zip.write_all(pres_rels.as_bytes()).unwrap();

        for (i, slide_xml) in slides.iter().enumerate() {
            let num = i + 1;
            zip.start_file(format!("ppt/slides/slide{num}.xml"), options)
                .unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();
        }

        for (path, bytes) in extra {
            zip.start_file(*path, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    buffer.into_inner()
}

#[test]
fn slide_backgrounds_solid_gradient_picture_and_layout_inherited() {
    let solid_slide = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:srgbClr val="E8F8F5"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>
  </p:cSld>
</p:sld>"#;

    let grad_slide = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:gradFill>
      <a:gsLst>
        <a:gs pos="0"><a:srgbClr val="0B1F4B"/></a:gs>
        <a:gs pos="100000"><a:srgbClr val="F2C14E"/></a:gs>
      </a:gsLst>
      <a:lin ang="3600000"/>
    </a:gradFill></p:bgPr></p:bg>
    <p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>
  </p:cSld>
</p:sld>"#;

    let pic_slide = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg><p:bgPr><a:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></a:blipFill></p:bgPr></p:bg>
    <p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>
  </p:cSld>
</p:sld>"#;

    let inherited_slide = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#;

    let pic_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/bg.png"/>
</Relationships>"#;

    let inh_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:srgbClr val="FDF2E9"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>
  </p:cSld>
</p:sldLayout>"#;

    let bytes = build_pptx(
        &[solid_slide, grad_slide, pic_slide, inherited_slide],
        &[
            ("ppt/slides/_rels/slide3.xml.rels", pic_rels.as_bytes()),
            ("ppt/media/bg.png", TINY_PNG),
            ("ppt/slides/_rels/slide4.xml.rels", inh_rels.as_bytes()),
            ("ppt/slideLayouts/slideLayout1.xml", layout_xml.as_bytes()),
        ],
    );

    let imported = import_pptx(&bytes, "Bg test").unwrap();
    let slides = &imported.course.scenes[0].slides;
    assert_eq!(slides[0].background, Some(Color("#E8F8F5".into())));

    match &slides[1].objects[0].kind {
        ObjectKind::Shape {
            fill_style: Some(FillStyle::Linear { angle, stops }),
            ..
        } => {
            assert!((*angle - 60.0).abs() < 0.1);
            assert_eq!(stops.len(), 2);
            assert_eq!(stops[0].color, Color("#0B1F4B".into()));
            assert_eq!(stops[1].color, Color("#F2C14E".into()));
        }
        other => panic!("expected linear gradient bg shape, got {other:?}"),
    }

    match &slides[2].objects[0].kind {
        ObjectKind::Shape {
            fill_style:
                Some(FillStyle::Picture {
                    mode: PictureFillMode::Stretch,
                    ..
                }),
            ..
        } => {}
        other => panic!("expected picture bg shape, got {other:?}"),
    }

    assert_eq!(slides[3].background, Some(Color("#FDF2E9".into())));
}

#[test]
fn theme_scheme_colors_and_modifiers_resolve_for_fills_outlines_and_text() {
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="Custom">
      <a:dk1><a:srgbClr val="111111"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="1B2A49"/></a:dk2>
      <a:lt2><a:srgbClr val="F4EBD0"/></a:lt2>
      <a:accent1><a:srgbClr val="E91E63"/></a:accent1>
      <a:accent2><a:srgbClr val="00BCD4"/></a:accent2>
      <a:accent3><a:srgbClr val="FFC107"/></a:accent3>
      <a:accent4><a:srgbClr val="673AB7"/></a:accent4>
      <a:accent5><a:srgbClr val="4CAF50"/></a:accent5>
      <a:accent6><a:srgbClr val="FF5722"/></a:accent6>
      <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
      <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sldMaster xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
        <a:solidFill><a:schemeClr val="accent1"/></a:solidFill>
        <a:ln w="25400"><a:solidFill><a:schemeClr val="accent6"/></a:solidFill></a:ln>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r>
          <a:rPr><a:solidFill><a:schemeClr val="tx1"/></a:solidFill></a:rPr>
          <a:t>Hello</a:t>
        </a:r></a:p>
      </p:txBody>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Tinted"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="1200000" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
        <a:solidFill><a:schemeClr val="accent1"><a:lumMod val="50000"/></a:schemeClr></a:solidFill>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;
    let layout_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#;
    let master_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#;

    let bytes = build_pptx(
        &[slide_xml],
        &[
            ("ppt/slides/_rels/slide1.xml.rels", slide_rels.as_bytes()),
            ("ppt/slideLayouts/slideLayout1.xml", layout_xml.as_bytes()),
            (
                "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
                layout_rels.as_bytes(),
            ),
            ("ppt/slideMasters/slideMaster1.xml", master_xml.as_bytes()),
            (
                "ppt/slideMasters/_rels/slideMaster1.xml.rels",
                master_rels.as_bytes(),
            ),
            ("ppt/theme/theme1.xml", theme_xml.as_bytes()),
        ],
    );

    let imported = import_pptx(&bytes, "Theme test").unwrap();
    assert!(
        !imported
            .report
            .items
            .iter()
            .any(|i| i.kind == SkipKind::SchemeColor),
        "scheme colours should resolve cleanly"
    );
    let objs = &imported.course.scenes[0].slides[0].objects;
    match &objs[0].kind {
        ObjectKind::Shape {
            fill,
            outline,
            text,
            ..
        } => {
            assert_eq!(*fill, Color("#E91E63".into()));
            assert_eq!(outline.as_ref().unwrap().color, Color("#FF5722".into()));
            assert_eq!(
                text.as_ref().unwrap().runs[0].color,
                Some(Color("#111111".into()))
            );
        }
        other => panic!("expected shape, got {other:?}"),
    }
    match &objs[1].kind {
        ObjectKind::Shape { fill, .. } => {
            assert_ne!(*fill, Color("#E91E63".into()));
            assert_ne!(*fill, Color("#D9D9D9".into()));
        }
        other => panic!("expected shape, got {other:?}"),
    }
}

#[test]
fn placeholder_formatting_inherited_from_layout_and_master() {
    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="5000000" cy="1000000"/></a:xfrm></p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:lstStyle><a:lvl1pPr>
          <a:defRPr sz="4400" b="1"><a:solidFill><a:srgbClr val="C0392B"/></a:solidFill><a:latin typeface="Georgia"/></a:defRPr>
        </a:lvl1pPr></a:lstStyle>
      </p:txBody>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Body"/><p:cNvSpPr/><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="1200000"/><a:ext cx="5000000" cy="3000000"/></a:xfrm></p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:lstStyle>
          <a:lvl1pPr>
            <a:buChar char="►"/>
            <a:defRPr sz="2800"><a:solidFill><a:srgbClr val="1F4E79"/></a:solidFill></a:defRPr>
          </a:lvl1pPr>
          <a:lvl2pPr>
            <a:buChar char="–"/>
            <a:defRPr sz="2000" i="1"><a:solidFill><a:srgbClr val="2E8B57"/></a:solidFill></a:defRPr>
          </a:lvl2pPr>
        </a:lstStyle>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
      <p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>My Title</a:t></a:r></a:p></p:txBody>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Body"/><p:cNvSpPr/><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr>
      <p:spPr/><p:txBody><a:bodyPr/>
        <a:p><a:r><a:t>Level 1</a:t></a:r></a:p>
        <a:p><a:pPr lvl="1"/><a:r><a:t>Level 2</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;

    let bytes = build_pptx(
        &[slide_xml],
        &[
            ("ppt/slides/_rels/slide1.xml.rels", slide_rels.as_bytes()),
            ("ppt/slideLayouts/slideLayout1.xml", layout_xml.as_bytes()),
        ],
    );

    let imported = import_pptx(&bytes, "PH test").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    match &objs[0].kind {
        ObjectKind::Text { runs, .. } => {
            assert_eq!(runs[0].bold, Some(true));
            assert_eq!(runs[0].font.as_deref(), Some("Georgia"));
            assert_eq!(runs[0].color, Some(Color("#C0392B".into())));
            assert!((runs[0].size.unwrap() - 44.0 * (4.0 / 3.0) * (4.0 / 3.0)).abs() < 0.2);
        }
        other => panic!("expected text, got {other:?}"),
    }
    match &objs[1].kind {
        ObjectKind::Text {
            runs, paragraphs, ..
        } => {
            assert_eq!(paragraphs[0].list, ListKind::Bullet);
            assert_eq!(paragraphs[0].bullet.as_deref(), Some("►"));
            assert_eq!(runs[0].color, Some(Color("#1F4E79".into())));
            assert_eq!(paragraphs[1].list, ListKind::Bullet);
            assert_eq!(paragraphs[1].level, 1);
            assert_eq!(paragraphs[1].bullet.as_deref(), Some("–"));
            assert_eq!(runs[1].italic, Some(true));
            assert_eq!(runs[1].color, Some(Color("#2E8B57".into())));
        }
        other => panic!("expected text, got {other:?}"),
    }
}

#[test]
fn text_features_bullets_numbering_and_frame_clamping() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Text"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
      <p:txBody>
        <a:bodyPr anchor="ctr"/>
        <a:p>
          <a:pPr><a:buClr><a:srgbClr val="E67E22"/></a:buClr><a:buSzPct val="120000"/><a:buChar char="★"/></a:pPr>
          <a:r><a:rPr strike="sngStrike" baseline="30000"><a:latin typeface="Courier New"/></a:rPr><a:t>Sup</a:t></a:r>
          <a:r><a:rPr baseline="-25000"/><a:t>Sub</a:t></a:r>
        </a:p>
        <a:p>
          <a:pPr lvl="2"><a:buAutoNum type="romanLcPeriod" startAt="5"/></a:pPr>
          <a:r><a:t>Item</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Huge"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="2000000000" cy="1000000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;

    let bytes = build_pptx(&[slide_xml], &[]);
    let imported = import_pptx(&bytes, "Features").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;

    let (style, runs, paragraphs) = match &objs[0].kind {
        ObjectKind::Text {
            style,
            runs,
            paragraphs,
            ..
        } => (style, runs, paragraphs),
        ObjectKind::Shape {
            text: Some(shape_text),
            ..
        } => (&shape_text.style, &shape_text.runs, &shape_text.paragraphs),
        other => panic!("expected text box or shape with text, got {other:?}"),
    };
    assert_eq!(style.valign, VAlign::Middle);
    // 18pt default on 1280x720 (font_scale 4/3) = 32px
    assert!((style.size - 32.0).abs() < 0.2);
    assert_eq!(runs[0].strikethrough, Some(true));
    assert_eq!(runs[0].baseline, Some(Baseline::Superscript));
    assert_eq!(runs[0].font.as_deref(), Some("Courier New"));
    assert_eq!(runs[1].baseline, Some(Baseline::Subscript));
    assert_eq!(paragraphs[0].bullet.as_deref(), Some("★"));
    assert_eq!(paragraphs[0].bullet_color, Some(Color("#E67E22".into())));
    assert!((paragraphs[0].bullet_size.unwrap() - 38.4).abs() < 0.2);
    assert_eq!(paragraphs[1].list, ListKind::Number);
    assert_eq!(paragraphs[1].level, 2);
    assert_eq!(
        paragraphs[1].number_scheme,
        Some(NumberScheme::RomanLcPeriod)
    );
    assert_eq!(paragraphs[1].start_at, Some(5));

    assert!(objs[1].frame.w <= MAX_OBJECT_FRAME_SIZE);
    assert!(objs[1].frame.h <= MAX_OBJECT_FRAME_SIZE);
    assert!(
        imported
            .report
            .items
            .iter()
            .any(|i| i.kind == SkipKind::InvalidGeometry)
    );
}

#[test]
fn video_imports_with_poster_and_missing_media_falls_back_to_poster_image() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:pic>
      <p:nvPicPr><p:cNvPr id="2" name="clip.mp4"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link="rId2"/><p:extLst><p:ext uri="{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}"><p14:media xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main" r:embed="rId2"/></p:ext></p:extLst></p:nvPr></p:nvPicPr>
      <p:blipFill><a:blip r:embed="rId3"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
    </p:pic>
    <p:pic>
      <p:nvPicPr><p:cNvPr id="3" name="missing.mp4"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link="rId4"/></p:nvPr></p:nvPicPr>
      <p:blipFill><a:blip r:embed="rId3"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
      <p:spPr><a:xfrm><a:off x="1000000" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
    </p:pic>
  </p:spTree></p:cSld>
</p:sld>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.microsoft.com/office/2007/relationships/media" Target="../media/clip.mp4"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/poster.png"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/video" Target="../media/missing.mp4"/>
</Relationships>"#;
    let bytes = build_pptx(
        &[slide_xml],
        &[
            ("ppt/slides/_rels/slide1.xml.rels", rels.as_bytes()),
            ("ppt/media/clip.mp4", b"ftypmp42"),
            ("ppt/media/poster.png", TINY_PNG),
        ],
    );
    let imported = import_pptx(&bytes, "Video").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    assert_eq!(objs.len(), 2);
    match &objs[0].kind {
        ObjectKind::Video { src, poster, .. } => {
            assert_eq!(src, "clip.mp4");
            assert_eq!(poster.as_deref(), Some("poster.png"));
        }
        other => panic!("expected video, got {other:?}"),
    }
    match &objs[1].kind {
        ObjectKind::Image { src, .. } => assert!(src.starts_with("poster")),
        other => panic!("expected fallback poster image, got {other:?}"),
    }
    assert!(
        imported
            .report
            .items
            .iter()
            .any(|i| i.kind == SkipKind::Media)
    );
}

#[test]
fn table_style_banding_and_merged_cells_import() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Tbl"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="0" y="0"/><a:ext cx="4000000" cy="2000000"/></p:xfrm>
      <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl>
        <a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr>
        <a:tblGrid><a:gridCol w="1000000"/><a:gridCol w="1000000"/><a:gridCol w="1000000"/></a:tblGrid>
        <a:tr h="500000">
          <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>H1</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
          <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>H2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
          <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>H3</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
        </a:tr>
        <a:tr h="500000">
          <a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>Span</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
          <a:tc hMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc>
          <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>Last</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
        </a:tr>
      </a:tbl></a:graphicData></a:graphic>
    </p:graphicFrame>
  </p:spTree></p:cSld>
</p:sld>"#;
    let bytes = build_pptx(&[slide_xml], &[]);
    let imported = import_pptx(&bytes, "Table").unwrap();
    let ObjectKind::Table {
        rows, header_row, ..
    } = &imported.course.scenes[0].slides[0].objects[0].kind
    else {
        panic!("expected table");
    };
    assert!(*header_row);
    assert!(rows[0].cells[0].fill.is_some());
    assert_eq!(rows[1].cells[0].col_span, 2);
    assert_eq!(rows[1].cells[2].text, "Last");
}

#[test]
fn custom_geometry_subpaths_gradient_and_text_import() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Ring"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
        <a:custGeom><a:pathLst>
          <a:path w="100" h="100">
            <a:moveTo><a:pt x="0" y="50"/></a:moveTo>
            <a:arcTo wR="50" hR="50" stAng="10800000" swAng="21600000"/><a:close/>
            <a:moveTo><a:pt x="25" y="50"/></a:moveTo>
            <a:arcTo wR="25" hR="25" stAng="10800000" swAng="-21600000"/><a:close/>
          </a:path>
          <a:path w="100" h="100" fill="none"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="100" y="100"/></a:lnTo></a:path>
        </a:pathLst></a:custGeom>
        <a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs><a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs></a:gsLst><a:lin ang="0"/></a:gradFill>
      </p:spPr>
      <p:txBody><a:bodyPr/><a:p><a:r><a:t>Inside</a:t></a:r></a:p></p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let bytes = build_pptx(&[slide_xml], &[]);
    let imported = import_pptx(&bytes, "CustGeom").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    assert_eq!(objs.len(), 2);
    let ObjectKind::Path { path } = &objs[0].kind else {
        panic!("expected path");
    };
    assert_eq!(path.subpaths.len(), 1);
    assert!(path.fill_style.is_some());
    assert_eq!(path.text.as_ref().map(|t| t.text.as_str()), Some("Inside"));
}

#[test]
fn pattern_fill_tile_scale_and_alpha_gradient_import() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Pat"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:pattFill prst="pct50"><a:fgClr><a:srgbClr val="FF0000"/></a:fgClr><a:bgClr><a:srgbClr val="0000FF"/></a:bgClr></a:pattFill></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Tile"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:blipFill><a:blip r:embed="rId2"/><a:tile sx="40000" sy="40000"/></a:blipFill></p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/tile.png"/>
</Relationships>"#;
    let bytes = build_pptx(
        &[slide_xml],
        &[
            ("ppt/slides/_rels/slide1.xml.rels", rels.as_bytes()),
            ("ppt/media/tile.png", TINY_PNG),
        ],
    );
    let imported = import_pptx(&bytes, "Fills").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    if let ObjectKind::Shape { fill, .. } = &objs[0].kind {
        assert_ne!(fill.0, "#D9D9D9");
    }
    if let ObjectKind::Shape {
        fill_style: Some(FillStyle::Picture {
            mode, tile_scale, ..
        }),
        ..
    } = &objs[1].kind
    {
        assert_eq!(*mode, PictureFillMode::Tile);
        assert!((tile_scale[0] - 0.4).abs() < 0.01);
    } else {
        panic!("expected tiled picture fill");
    }
}

#[test]
fn geometry_dash_dot_picture_outline_ellipse_and_text_gaps_import() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Tri"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="triangle"><a:avLst><a:gd name="adj" fmla="val 20000"/></a:avLst></a:prstGeom><a:solidFill><a:srgbClr val="112233"/></a:solidFill><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:prstDash val="dashDot"/></a:ln></p:spPr>
    </p:sp>
    <p:pic>
      <p:nvPicPr><p:cNvPr id="3" name="EllPic"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
      <p:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="ellipse"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:ln></p:spPr>
    </p:pic>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="4" name="TxtBox"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln w="12700"><a:solidFill><a:srgbClr val="334455"/></a:solidFill></a:ln></p:spPr>
      <p:txBody><a:bodyPr wrap="none" vert="vert270" lIns="127000" tIns="0" rIns="0" bIns="0"/><a:p><a:pPr algn="just"/><a:r><a:t>Hi</a:t></a:r></a:p></p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/pic.png"/>
</Relationships>"#;
    let bytes = build_pptx(
        &[slide_xml],
        &[
            ("ppt/slides/_rels/slide1.xml.rels", rels.as_bytes()),
            ("ppt/media/pic.png", TINY_PNG),
        ],
    );
    let imported = import_pptx(&bytes, "GeoText").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    if let ObjectKind::Shape {
        adjust, outline, ..
    } = &objs[0].kind
    {
        assert!((adjust[0] - 0.2).abs() < 0.01);
        assert_eq!(
            outline.as_ref().unwrap().dash,
            coursecraft_course::DashStyle::DashDot
        );
    } else {
        panic!("expected triangle shape");
    }
    if let ObjectKind::Image { outline, shape, .. } = &objs[1].kind {
        assert_eq!(*shape, Some(coursecraft_course::ShapeKind::Ellipse));
        assert!(outline.is_some());
    } else {
        panic!("expected ellipse image");
    }
    if let ObjectKind::Text { style, outline, .. } = &objs[2].kind {
        assert_eq!(style.align, coursecraft_course::Align::Justify);
        assert!(!style.wrap);
        assert_eq!(
            style.direction,
            coursecraft_course::TextDirection::Vertical270
        );
        assert!(style.insets.unwrap().left > 5.0);
        assert!(outline.is_some());
    } else {
        panic!("expected text box");
    }
}

#[test]
fn imports_custom_and_preset_dash_patterns() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CustDash"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:custDash><a:ds d="400000" sp="200000"/><a:ds d="100000" sp="200000"/></a:custDash></a:ln></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="SysDash"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:prstDash val="sysDash"/></a:ln></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="4" name="LgDashDotDot"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:prstDash val="lgDashDotDot"/></a:ln></p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let bytes = build_pptx(&[slide_xml], &[]);
    let imported = import_pptx(&bytes, "Dashes").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    let outline_of = |idx: usize| match &objs[idx].kind {
        ObjectKind::Shape { outline, .. } => outline.as_ref().expect("outline").clone(),
        _ => panic!("expected shape at {idx}"),
    };
    let cust = outline_of(0);
    assert_eq!(cust.dash, coursecraft_course::DashStyle::Custom);
    assert_eq!(
        cust.dash_pattern.as_deref(),
        Some(&[4.0, 2.0, 1.0, 2.0][..])
    );

    let sys_dash = outline_of(1);
    assert_eq!(sys_dash.dash, coursecraft_course::DashStyle::Dash);
    assert_eq!(sys_dash.dash_pattern.as_deref(), Some(&[3.0, 1.0][..]));

    let lg_ddd = outline_of(2);
    assert_eq!(lg_ddd.dash, coursecraft_course::DashStyle::DashDot);
    assert_eq!(
        lg_ddd.dash_pattern.as_deref(),
        Some(&[8.0, 3.0, 1.0, 3.0, 1.0, 3.0][..])
    );
}

#[test]
fn imports_line_join_and_miter_limit() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Default"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Round"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:round/></a:ln></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="4" name="Bevel"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:bevel/></a:ln></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="5" name="Miter4"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:miter lim="400000"/></a:ln></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="6" name="Miter8"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100000" cy="100000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:miter lim="800000"/></a:ln></p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let bytes = build_pptx(&[slide_xml], &[]);
    let imported = import_pptx(&bytes, "Joins").unwrap();
    let objs = &imported.course.scenes[0].slides[0].objects;
    let outline_of = |idx: usize| match &objs[idx].kind {
        ObjectKind::Shape { outline, .. } => outline.as_ref().expect("outline").clone(),
        _ => panic!("expected shape at {idx}"),
    };
    assert_eq!(outline_of(0).join, coursecraft_course::LineJoin::Miter);
    assert_eq!(outline_of(0).miter_limit, None);

    assert_eq!(outline_of(1).join, coursecraft_course::LineJoin::Round);
    assert_eq!(outline_of(2).join, coursecraft_course::LineJoin::Bevel);

    assert_eq!(outline_of(3).join, coursecraft_course::LineJoin::Miter);
    assert_eq!(outline_of(3).miter_limit, Some(4.0));

    assert_eq!(outline_of(4).join, coursecraft_course::LineJoin::Miter);
    assert_eq!(outline_of(4).miter_limit, None);
}
