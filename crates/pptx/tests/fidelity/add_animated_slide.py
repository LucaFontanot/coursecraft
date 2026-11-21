#!/usr/bin/env python3
"""Append Slide 20 (Animations & Motion Paths) to fidelity.pptx.

Uses only Python standard library modules (zipfile, xml.etree.ElementTree).
All existing Slides 1-19 and unrelated zip entries are copied byte-for-byte.
"""

from __future__ import annotations

import io
from pathlib import Path
import xml.etree.ElementTree as ET
import zipfile

NS_CT = "http://schemas.openxmlformats.org/package/2006/content-types"
NS_REL = "http://schemas.openxmlformats.org/package/2006/relationships"
NS_P = "http://schemas.openxmlformats.org/presentationml/2006/main"
NS_A = "http://schemas.openxmlformats.org/drawingml/2006/main"
NS_R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"

SLIDE_PART = "ppt/slides/slide20.xml"
SLIDE_RELS_PART = "ppt/slides/_rels/slide20.xml.rels"
SLIDE_REL_TARGET = "slides/slide20.xml"
SLIDE_CONTENT_TYPE = (
    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml"
)
SLIDE_REL_TYPE = (
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide"
)

SLIDE20_XML = """<?xml version='1.0' encoding='UTF-8' standalone='yes'?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill>
        <a:effectLst/>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>

      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Slide 20 Title"/>
          <p:cNvSpPr txBox="1"/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="457200" y="228600"/><a:ext cx="11277600" cy="548640"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
          <a:noFill/>
        </p:spPr>
        <p:txBody>
          <a:bodyPr wrap="square"><a:spAutoFit/></a:bodyPr>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr sz="2400" b="1"><a:solidFill><a:srgbClr val="111827"/></a:solidFill></a:rPr>
              <a:t>Slide 20: Animations &amp; Motion Paths</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>

      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Blue Rect"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="685800" y="1371600"/><a:ext cx="1371600" cy="1371600"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="2980B9"/></a:solidFill>
          <a:ln w="12700"><a:solidFill><a:srgbClr val="1B4F72"/></a:solidFill></a:ln>
        </p:spPr>
      </p:sp>

      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="4" name="Green Rounded Rect"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="2743200" y="1371600"/><a:ext cx="1371600" cy="1371600"/></a:xfrm>
          <a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="27AE60"/></a:solidFill>
          <a:ln w="12700"><a:solidFill><a:srgbClr val="196F3D"/></a:solidFill></a:ln>
        </p:spPr>
      </p:sp>

      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="5" name="Orange Shape"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="4800600" y="1371600"/><a:ext cx="1371600" cy="1371600"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="E67E22"/></a:solidFill>
          <a:ln w="12700"><a:solidFill><a:srgbClr val="A04000"/></a:solidFill></a:ln>
        </p:spPr>
      </p:sp>

      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="6" name="Purple Circle"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="685800" y="3429000"/><a:ext cx="1371600" cy="1371600"/></a:xfrm>
          <a:prstGeom prst="ellipse"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="8E44AD"/></a:solidFill>
          <a:ln w="12700"><a:solidFill><a:srgbClr val="5B2C6F"/></a:solidFill></a:ln>
        </p:spPr>
      </p:sp>

      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="7" name="Red Star"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="8229600" y="1371600"/><a:ext cx="1371600" cy="1371600"/></a:xfrm>
          <a:prstGeom prst="star5"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="E74C3C"/></a:solidFill>
          <a:ln w="12700"><a:solidFill><a:srgbClr val="922B21"/></a:solidFill></a:ln>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
  <p:timing>
    <p:tnLst>
      <p:par>
        <p:cTn id="1" nodeType="tmRoot">
          <p:childTnLst>
            <p:seq>
              <p:cTn id="2" nodeType="mainSeq">
                <p:childTnLst>
                  <p:par>
                    <p:cTn id="3">
                      <p:childTnLst>
                        <p:par>
                          <p:cTn id="10" presetID="10" presetClass="entr" presetSubtype="0" nodeType="withEffect" dur="600">
                            <p:stCondLst><p:cond delay="0"/></p:stCondLst>
                            <p:childTnLst>
                              <p:set><p:cBhvr><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:set>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                        <p:par>
                          <p:cTn id="11" presetID="2" presetClass="entr" presetSubtype="8" nodeType="afterEffect" dur="500">
                            <p:stCondLst><p:cond delay="200"/></p:stCondLst>
                            <p:childTnLst>
                              <p:set><p:cBhvr><p:tgtEl><p:spTgt spid="4"/></p:tgtEl></p:cBhvr></p:set>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                        <p:par>
                          <p:cTn id="12" presetID="22" presetClass="entr" presetSubtype="4" nodeType="withEffect" dur="400">
                            <p:stCondLst><p:cond delay="100"/></p:stCondLst>
                            <p:childTnLst>
                              <p:set><p:cBhvr><p:tgtEl><p:spTgt spid="5"/></p:tgtEl></p:cBhvr></p:set>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                        <p:par>
                          <p:cTn id="13" presetID="2" presetClass="exit" presetSubtype="2" nodeType="afterEffect" dur="500">
                            <p:stCondLst><p:cond delay="700"/></p:stCondLst>
                            <p:childTnLst>
                              <p:set><p:cBhvr><p:tgtEl><p:spTgt spid="5"/></p:tgtEl></p:cBhvr></p:set>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                        <p:par>
                          <p:cTn id="14" presetID="23" presetClass="entr" presetSubtype="0" nodeType="clickEffect" dur="500">
                            <p:childTnLst>
                              <p:set><p:cBhvr><p:tgtEl><p:spTgt spid="6"/></p:tgtEl></p:cBhvr></p:set>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                        <p:par>
                          <p:cTn id="15" presetID="0" presetClass="path" nodeType="afterEffect" dur="1000">
                            <p:childTnLst>
                              <p:animMotion path="M 0 0 L 0.25 0.0 L 0.25 0.20 E" origin="layout" pathEditMode="relative">
                                <p:cBhvr>
                                  <p:cTn id="16" dur="1000"/>
                                  <p:tgtEl><p:spTgt spid="6"/></p:tgtEl>
                                </p:cBhvr>
                              </p:animMotion>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                        <p:par>
                          <p:cTn id="17" presetID="42" presetClass="entr" presetSubtype="4" nodeType="afterEffect" dur="500">
                            <p:childTnLst>
                              <p:set><p:cBhvr><p:tgtEl><p:spTgt spid="7"/></p:tgtEl></p:cBhvr></p:set>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                        <p:par>
                          <p:cTn id="18" presetID="8" presetClass="emph" nodeType="afterEffect" dur="800" repeatCount="2000">
                            <p:childTnLst>
                              <p:set><p:cBhvr><p:tgtEl><p:spTgt spid="7"/></p:tgtEl></p:cBhvr></p:set>
                            </p:childTnLst>
                          </p:cTn>
                        </p:par>
                      </p:childTnLst>
                    </p:cTn>
                  </p:par>
                </p:childTnLst>
              </p:cTn>
            </p:seq>
          </p:childTnLst>
        </p:cTn>
      </p:par>
    </p:tnLst>
  </p:timing>
</p:sld>
"""

SLIDE20_RELS_XML = """<?xml version='1.0' encoding='UTF-8' standalone='yes'?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout7.xml"/>
</Relationships>
"""


def update_content_types(raw: bytes) -> bytes:
    ET.register_namespace("", NS_CT)
    root = ET.fromstring(raw)
    part_name = "/" + SLIDE_PART
    for elem in root.findall(f"{{{NS_CT}}}Override"):
        if elem.attrib.get("PartName") == part_name:
            return raw
    override = ET.SubElement(root, f"{{{NS_CT}}}Override")
    override.set("PartName", part_name)
    override.set("ContentType", SLIDE_CONTENT_TYPE)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def update_presentation_rels(raw: bytes) -> tuple[bytes, str]:
    ET.register_namespace("", NS_REL)
    root = ET.fromstring(raw)
    for rel in root.findall(f"{{{NS_REL}}}Relationship"):
        if rel.attrib.get("Target") == SLIDE_REL_TARGET:
            return raw, rel.attrib["Id"]
    used = {
        int(rel.attrib["Id"][3:])
        for rel in root.findall(f"{{{NS_REL}}}Relationship")
        if rel.attrib.get("Id", "").startswith("rId")
        and rel.attrib["Id"][3:].isdigit()
    }
    next_id = max(used, default=0) + 1
    rid = f"rId{next_id}"
    rel = ET.SubElement(root, f"{{{NS_REL}}}Relationship")
    rel.set("Id", rid)
    rel.set("Type", SLIDE_REL_TYPE)
    rel.set("Target", SLIDE_REL_TARGET)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True), rid


def update_presentation_xml(raw: bytes, rid: str) -> bytes:
    ET.register_namespace("a", NS_A)
    ET.register_namespace("p", NS_P)
    ET.register_namespace("r", NS_R)
    root = ET.fromstring(raw)
    sld_id_lst = root.find(f"{{{NS_P}}}sldIdLst")
    if sld_id_lst is None:
        raise RuntimeError("ppt/presentation.xml missing p:sldIdLst")
    for sld in sld_id_lst.findall(f"{{{NS_P}}}sldId"):
        if sld.attrib.get(f"{{{NS_R}}}id") == rid:
            return raw
    used_ids = [
        int(sld.attrib["id"])
        for sld in sld_id_lst.findall(f"{{{NS_P}}}sldId")
        if sld.attrib.get("id", "").isdigit()
    ]
    next_sld_id = max(used_ids, default=255) + 1
    sld_el = ET.SubElement(sld_id_lst, f"{{{NS_P}}}sldId")
    sld_el.set("id", str(next_sld_id))
    sld_el.set(f"{{{NS_R}}}id", rid)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def patch_fidelity_pptx(pptx_path: Path) -> None:
    original = pptx_path.read_bytes()
    out_buf = io.BytesIO()
    with (
        zipfile.ZipFile(io.BytesIO(original), "r") as zin,
        zipfile.ZipFile(out_buf, "w", compression=zipfile.ZIP_DEFLATED) as zout,
    ):
        pres_rels_bytes = zin.read("ppt/_rels/presentation.xml.rels")
        new_pres_rels_bytes, slide20_rid = update_presentation_rels(pres_rels_bytes)
        new_ct_bytes = update_content_types(zin.read("[Content_Types].xml"))
        new_pres_bytes = update_presentation_xml(
            zin.read("ppt/presentation.xml"), slide20_rid
        )

        replaced = {
            "[Content_Types].xml": new_ct_bytes,
            "ppt/_rels/presentation.xml.rels": new_pres_rels_bytes,
            "ppt/presentation.xml": new_pres_bytes,
            SLIDE_PART: SLIDE20_XML.encode("utf-8"),
            SLIDE_RELS_PART: SLIDE20_RELS_XML.encode("utf-8"),
        }

        written = set()
        for info in zin.infolist():
            data = replaced.get(info.filename)
            if data is not None:
                zout.writestr(info, data)
                written.add(info.filename)
            else:
                zout.writestr(info, zin.read(info.filename))

        for name, data in replaced.items():
            if name not in written:
                zout.writestr(name, data)

    pptx_path.write_bytes(out_buf.getvalue())


if __name__ == "__main__":
    target = Path(__file__).resolve().parent / "fidelity.pptx"
    patch_fidelity_pptx(target)
    print(f"Updated {target} with {SLIDE_PART}")
