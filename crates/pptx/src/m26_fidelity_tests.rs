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

    match &objs[0].kind {
        ObjectKind::Shape {
            text: Some(shape_text),
            ..
        } => {
            assert_eq!(shape_text.style.valign, VAlign::Middle);
            // 18pt default on 1280x720 (font_scale 4/3) = 32px
            assert!((shape_text.style.size - 32.0).abs() < 0.2);
            assert_eq!(shape_text.runs[0].strikethrough, Some(true));
            assert_eq!(shape_text.runs[0].baseline, Some(Baseline::Superscript));
            assert_eq!(shape_text.runs[0].font.as_deref(), Some("Courier New"));
            assert_eq!(shape_text.runs[1].baseline, Some(Baseline::Subscript));
            assert_eq!(shape_text.paragraphs[0].bullet.as_deref(), Some("★"));
            assert_eq!(
                shape_text.paragraphs[0].bullet_color,
                Some(Color("#E67E22".into()))
            );
            assert!((shape_text.paragraphs[0].bullet_size.unwrap() - 38.4).abs() < 0.2);
            assert_eq!(shape_text.paragraphs[1].list, ListKind::Number);
            assert_eq!(shape_text.paragraphs[1].level, 2);
            assert_eq!(
                shape_text.paragraphs[1].number_scheme,
                Some(NumberScheme::RomanLcPeriod)
            );
            assert_eq!(shape_text.paragraphs[1].start_at, Some(5));
        }
        other => panic!("expected shape with text, got {other:?}"),
    }

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
