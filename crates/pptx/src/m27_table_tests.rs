//! M27 Step C-1 tests: PowerPoint style-less & bordered tables (C2.1–C2.8, C2.10).
//! Rendered pixel check & Slide 14 fidelity score (C2.9) live in
//! `apps/coursecraft-cli/tests/pptx_fidelity.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write as _;

use coursecraft_course::{DashStyle, ObjectKind, TableBorders, TableRow, VAlign};

use crate::import_pptx;

/// Standard 16:9 PowerPoint slide size in EMUs (13.333" x 7.5").
const STD_16_9_CX: u64 = 12_192_000;
const STD_16_9_CY: u64 = 6_858_000;

fn build_table_pptx(cx_emu: u64, cy_emu: u64, tbl_xml: &str) -> Vec<u8> {
    let slide_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Table 1"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm>
          <a:off x="914400" y="914400"/>
          <a:ext cx="6858000" cy="3657600"/>
        </p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            {tbl_xml}
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#
    );

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        let pres_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst>
  <p:sldSz cx="{cx_emu}" cy="{cy_emu}"/>
</p:presentation>"#
        );
        zip.start_file("ppt/presentation.xml", options).unwrap();
        zip.write_all(pres_xml.as_bytes()).unwrap();

        let pres_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#;
        zip.start_file("ppt/_rels/presentation.xml.rels", options)
            .unwrap();
        zip.write_all(pres_rels.as_bytes()).unwrap();

        zip.start_file("ppt/slides/slide1.xml", options).unwrap();
        zip.write_all(slide_xml.as_bytes()).unwrap();

        zip.finish().unwrap();
    }
    buffer.into_inner()
}

fn import_single_table(
    cx_emu: u64,
    cy_emu: u64,
    tbl_xml: &str,
) -> (Vec<TableRow>, TableBorders, bool) {
    let bytes = build_table_pptx(cx_emu, cy_emu, tbl_xml);
    let imported = import_pptx(&bytes, "Table Test").expect("import_pptx");
    assert!(
        imported.course.is_playable(),
        "course validation failed: {:?}",
        imported.course.validate()
    );
    let slide = &imported.course.scenes[0].slides[0];
    let (obj, _) = slide.object("table1").expect("table1 imported");
    match &obj.kind {
        ObjectKind::Table {
            rows,
            borders,
            header_row,
            ..
        } => (rows.clone(), borders.clone(), *header_row),
        other => panic!("expected Table, got {other:?}"),
    }
}

fn plain_3x3_rows() -> &'static str {
    r#"
    <a:tblGrid>
      <a:gridCol w="2286000"/>
      <a:gridCol w="2286000"/>
      <a:gridCol w="2286000"/>
    </a:tblGrid>
    <a:tr h="1219200">
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>A1</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B1</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>C1</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
    </a:tr>
    <a:tr h="1219200">
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>A2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>C2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
    </a:tr>
    <a:tr h="1219200">
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>A3</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B3</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>C3</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
    </a:tr>"#
}

fn assert_default_1pt_black_grid(borders: &TableBorders, expected_width: f32) {
    let outer = borders.outer.as_ref().expect("expected outer grid border");
    assert_eq!(outer.color.0, "#000000");
    assert_eq!(outer.dash, DashStyle::Solid);
    assert!(
        (outer.width - expected_width).abs() < 0.05,
        "expected outer width ~{expected_width}, got {}",
        outer.width
    );

    let inner = borders.inner.as_ref().expect("expected inner grid border");
    assert_eq!(inner.color.0, "#000000");
    assert_eq!(inner.dash, DashStyle::Solid);
    assert!(
        (inner.width - expected_width).abs() < 0.05,
        "expected inner width ~{expected_width}, got {}",
        inner.width
    );
}

#[test]
fn c2_1_plain_table_without_style_id_gets_default_1pt_black_grid() {
    let tbl = format!(
        r#"<a:tbl><a:tblPr firstRow="0" bandRow="0"/>{}</a:tbl>"#,
        plain_3x3_rows()
    );
    let (_, borders, header_row) = import_single_table(STD_16_9_CX, STD_16_9_CY, &tbl);
    assert!(!header_row);
    let expected_w = (12_700.0 * (1280.0 / STD_16_9_CX as f64)) as f32;
    assert_default_1pt_black_grid(&borders, expected_w);
}

#[test]
fn c2_2_table_without_tbl_pr_gets_default_1pt_black_grid() {
    let tbl = format!(r#"<a:tbl>{}</a:tbl>"#, plain_3x3_rows());
    let (_, borders, _) = import_single_table(STD_16_9_CX, STD_16_9_CY, &tbl);
    let expected_w = (12_700.0 * (1280.0 / STD_16_9_CX as f64)) as f32;
    assert_default_1pt_black_grid(&borders, expected_w);
}

#[test]
fn c2_3_no_style_table_grid_guid_scales_with_layout() {
    let tbl = format!(
        r#"<a:tbl>
          <a:tblPr><a:tableStyleId>{{5940675A-B579-460E-94D1-54222C63F5DA}}</a:tableStyleId></a:tblPr>
          {}
        </a:tbl>"#,
        plain_3x3_rows()
    );

    let (_, borders_std, _) = import_single_table(STD_16_9_CX, STD_16_9_CY, &tbl);
    let expected_std = (12_700.0 * (1280.0 / STD_16_9_CX as f64)) as f32;
    assert_default_1pt_black_grid(&borders_std, expected_std);

    // 10" wide presentation has a larger layout.scale (~1.778px per 1pt).
    let small_cx = 9_144_000_u64;
    let small_cy = 5_143_500_u64;
    let (_, borders_small, _) = import_single_table(small_cx, small_cy, &tbl);
    let expected_small = (12_700.0 * (1280.0 / small_cx as f64)) as f32;
    assert_default_1pt_black_grid(&borders_small, expected_small);
}

#[test]
fn c2_4_no_style_no_grid_guid_stays_borderless() {
    let tbl = format!(
        r#"<a:tbl>
          <a:tblPr><a:tableStyleId>{{2D5ABB26-0587-4C30-8999-92F81FD0307C}}</a:tableStyleId></a:tblPr>
          {}
        </a:tbl>"#,
        plain_3x3_rows()
    );
    let (_, borders, _) = import_single_table(STD_16_9_CX, STD_16_9_CY, &tbl);
    assert!(
        borders.outer.is_none(),
        "No Style, No Grid should have outer=None, got {:?}",
        borders.outer
    );
    assert!(
        borders.inner.is_none(),
        "No Style, No Grid should have inner=None, got {:?}",
        borders.inner
    );
}

#[test]
fn c2_5_explicit_no_fill_on_all_cell_borders_stays_borderless() {
    let no_fill_tc_pr = r#"<a:tcPr>
      <a:lnL><a:noFill/></a:lnL>
      <a:lnR><a:noFill/></a:lnR>
      <a:lnT><a:noFill/></a:lnT>
      <a:lnB><a:noFill/></a:lnB>
    </a:tcPr>"#;
    let cell = format!(
        r#"<a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>X</a:t></a:r></a:p></a:txBody>{no_fill_tc_pr}</a:tc>"#
    );
    let row = format!(r#"<a:tr h="1219200">{cell}{cell}{cell}</a:tr>"#);
    let tbl = format!(
        r#"<a:tbl>
          <a:tblPr firstRow="0" bandRow="0"/>
          <a:tblGrid><a:gridCol w="2286000"/><a:gridCol w="2286000"/><a:gridCol w="2286000"/></a:tblGrid>
          {row}{row}{row}
        </a:tbl>"#
    );
    let (_, borders, _) = import_single_table(STD_16_9_CX, STD_16_9_CY, &tbl);
    assert!(
        borders.outer.is_none(),
        "explicit noFill outer should be None, got {:?}",
        borders.outer
    );
    assert!(
        borders.inner.is_none(),
        "explicit noFill inner should be None, got {:?}",
        borders.inner
    );
}

#[test]
fn c2_6_partial_explicit_outer_red_2pt_and_omitted_inner_inherits_default_grid() {
    let red_2pt = r#"w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>"#;
    let tbl = format!(
        r#"<a:tbl>
          <a:tblPr firstRow="0" bandRow="0"/>
          <a:tblGrid><a:gridCol w="2286000"/><a:gridCol w="2286000"/></a:tblGrid>
          <a:tr h="1219200">
            <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>TL</a:t></a:r></a:p></a:txBody>
              <a:tcPr><a:lnL {red_2pt}</a:lnL><a:lnT {red_2pt}</a:lnT></a:tcPr>
            </a:tc>
            <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>TR</a:t></a:r></a:p></a:txBody>
              <a:tcPr><a:lnR {red_2pt}</a:lnR><a:lnT {red_2pt}</a:lnT></a:tcPr>
            </a:tc>
          </a:tr>
          <a:tr h="1219200">
            <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>BL</a:t></a:r></a:p></a:txBody>
              <a:tcPr><a:lnL {red_2pt}</a:lnL><a:lnB {red_2pt}</a:lnB></a:tcPr>
            </a:tc>
            <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>BR</a:t></a:r></a:p></a:txBody>
              <a:tcPr><a:lnR {red_2pt}</a:lnR><a:lnB {red_2pt}</a:lnB></a:tcPr>
            </a:tc>
          </a:tr>
        </a:tbl>"#
    );
    let (_, borders, _) = import_single_table(STD_16_9_CX, STD_16_9_CY, &tbl);

    let outer = borders.outer.as_ref().expect("outer border");
    assert_eq!(outer.color.0, "#FF0000");
    let expected_2pt = (25_400.0 * (1280.0 / STD_16_9_CX as f64)) as f32;
    assert!(
        (outer.width - expected_2pt).abs() < 0.05,
        "expected outer ~{expected_2pt}, got {}",
        outer.width
    );

    let inner = borders.inner.as_ref().expect("inner border");
    assert_eq!(inner.color.0, "#000000");
    let expected_1pt = (12_700.0 * (1280.0 / STD_16_9_CX as f64)) as f32;
    assert!(
        (inner.width - expected_1pt).abs() < 0.05,
        "expected inner ~{expected_1pt}, got {}",
        inner.width
    );
}

#[test]
fn c2_7_styled_table_medium_style_2_accent_1_unaffected() {
    let tbl = format!(
        r#"<a:tbl>
          <a:tblPr firstRow="1" bandRow="1">
            <a:tableStyleId>{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}</a:tableStyleId>
          </a:tblPr>
          {}
        </a:tbl>"#,
        plain_3x3_rows()
    );
    let (rows, borders, header_row) = import_single_table(STD_16_9_CX, STD_16_9_CY, &tbl);
    assert!(header_row);
    assert_eq!(borders.outer.as_ref().unwrap().color.0, "#FFFFFF");
    assert_eq!(borders.inner.as_ref().unwrap().color.0, "#FFFFFF");
    // Header row receives accent fill and white bold text; data rows receive band fills.
    assert!(rows[0].cells[0].fill.is_some());
    assert!(rows[1].cells[0].fill.is_some());
    assert_ne!(rows[0].cells[0].fill, rows[1].cells[0].fill);
}

#[test]
fn c2_8_unrecognized_table_style_id_keeps_no_style_borders_and_reports_note() {
    let tbl_no_cell_borders = format!(
        r#"<a:tbl>
          <a:tblPr><a:tableStyleId>{{DEADBEEF-0000-0000-0000-000000000000}}</a:tableStyleId></a:tblPr>
          {}
        </a:tbl>"#,
        plain_3x3_rows()
    );
    let pptx = build_table_pptx(STD_16_9_CX, STD_16_9_CY, &tbl_no_cell_borders);
    let result = import_pptx(&pptx, "Table Test").expect("import pptx");
    let borders = result.course.scenes[0].slides[0]
        .objects
        .iter()
        .find_map(|o| match &o.kind {
            ObjectKind::Table { borders, .. } => Some(borders.clone()),
            _ => None,
        })
        .expect("table imported");
    assert_eq!(borders.outer, None);
    assert_eq!(borders.inner, None);
    assert!(result.report.items.iter().any(|item| {
        item.kind == crate::SkipKind::Table
            && item.note.contains(
                "Table style {DEADBEEF-0000-0000-0000-000000000000} not recognized; borders may differ",
            )
    }));

    // Explicit cell <a:ln*> is still honoured on an unrecognized style GUID.
    let tbl_with_explicit_outer = r#"<a:tbl>
      <a:tblPr><a:tableStyleId>{DEADBEEF-0000-0000-0000-000000000000}</a:tableStyleId></a:tblPr>
      <a:tblGrid><a:gridCol w="2286000"/><a:gridCol w="2286000"/></a:tblGrid>
      <a:tr h="914400">
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>A</a:t></a:r></a:p></a:txBody><a:tcPr>
          <a:lnL w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnL>
          <a:lnT w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnT>
        </a:tcPr></a:tc>
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B</a:t></a:r></a:p></a:txBody><a:tcPr>
          <a:lnR w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnR>
          <a:lnT w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnT>
        </a:tcPr></a:tc>
      </a:tr>
      <a:tr h="914400">
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>C</a:t></a:r></a:p></a:txBody><a:tcPr>
          <a:lnL w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnL>
          <a:lnB w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnB>
        </a:tcPr></a:tc>
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>D</a:t></a:r></a:p></a:txBody><a:tcPr>
          <a:lnR w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnR>
          <a:lnB w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnB>
        </a:tcPr></a:tc>
      </a:tr>
    </a:tbl>"#;
    let (_, explicit_borders, _) =
        import_single_table(STD_16_9_CX, STD_16_9_CY, tbl_with_explicit_outer);
    assert_eq!(explicit_borders.outer.as_ref().unwrap().color.0, "#FF0000");
    assert_eq!(explicit_borders.inner, None);
}

#[test]
fn c2_10_tc_pr_anchor_valign_and_merged_cells_with_default_grid() {
    let tbl = r#"<a:tbl>
      <a:tblPr firstRow="0" bandRow="0"/>
      <a:tblGrid>
        <a:gridCol w="2286000"/>
        <a:gridCol w="2286000"/>
        <a:gridCol w="2286000"/>
      </a:tblGrid>
      <a:tr h="1219200">
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>Mid</a:t></a:r></a:p></a:txBody><a:tcPr anchor="ctr"/></a:tc>
        <a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>Span2</a:t></a:r></a:p></a:txBody><a:tcPr anchor="b"/></a:tc>
        <a:tc hMerge="1"/>
      </a:tr>
      <a:tr h="1219200">
        <a:tc rowSpan="2"><a:txBody><a:bodyPr anchor="t"/><a:p><a:r><a:t>BodyTop</a:t></a:r></a:p></a:txBody><a:tcPr anchor="b"/></a:tc>
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>C2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      </a:tr>
      <a:tr h="1219200">
        <a:tc vMerge="1"/>
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B3</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
        <a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>C3</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
      </a:tr>
    </a:tbl>"#;
    let (rows, borders, _) = import_single_table(STD_16_9_CX, STD_16_9_CY, tbl);
    let expected_w = (12_700.0 * (1280.0 / STD_16_9_CX as f64)) as f32;
    assert_default_1pt_black_grid(&borders, expected_w);

    assert_eq!(rows[0].cells[0].valign, VAlign::Middle);
    assert_eq!(rows[0].cells[1].valign, VAlign::Bottom);
    assert_eq!(rows[0].cells[1].col_span, 2);
    // Explicit bodyPr@anchor="t" takes precedence over tcPr@anchor="b".
    assert_eq!(rows[1].cells[0].valign, VAlign::Top);
    assert_eq!(rows[1].cells[0].row_span, 2);
}
