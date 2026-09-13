#!/usr/bin/env python3
"""Generator (g4-binary F14 validation): office document packages (OOXML, ODF, EPUB).  GENERATED DATA.

Contract: python gen_office_docs.py --out <staging> --seed N --params <JSON>
params: profile ("office-v1")

Writes minimal but structurally valid packages directly with zipfile (no third-party libraries, so
output is deterministic): fixed zip timestamps, fixed docProps dates, sorted parts except where the
format mandates order (ODF/EPUB 'mimetype' first and stored).
  docs/report-NN.docx   paragraphs + tables; some embed an .xlsx part (word/embeddings/) and PNG media
  sheets/data-NN.xlsx   shared strings + numeric sheets
  slides/deck-NN.pptx   slides with text and an embedded .docx part
  odf/text-NN.odt       content.xml/styles.xml/meta.xml/manifest + an embedded ODS object (sub-directory)
  odf/calc-NN.ods
  books/book.epub       XHTML chapters
PNG media are generated with zlib (deflate inside a zip = double compression), which is typical of
real office files.  Content text comes from random.Random(seed).
"""

import argparse
import io
import json
import os
import random
import struct
import zipfile
import zlib
from xml.sax.saxutils import escape

ZDATE = (2026, 1, 1, 0, 0, 0)
WORDS = ("quarterly revenue forecast region margin customer pipeline renewal churn capacity storage archive "
         "latency throughput budget headcount roadmap milestone risk mitigation vendor contract audit").split()


def zpack(parts, first_stored=None):
    bio = io.BytesIO()
    with zipfile.ZipFile(bio, "w") as z:
        names = sorted(parts)
        if first_stored:
            names.remove(first_stored)
            zi = zipfile.ZipInfo(first_stored, date_time=ZDATE)
            zi.compress_type = zipfile.ZIP_STORED
            z.writestr(zi, parts[first_stored])
        for n in names:
            zi = zipfile.ZipInfo(n, date_time=ZDATE)
            zi.compress_type = zipfile.ZIP_DEFLATED
            zi.external_attr = 0o100644 << 16
            z.writestr(zi, parts[n])
    return bio.getvalue()


def png(rng, w, h):
    raw = bytearray()
    base = [rng.randrange(256) for _ in range(3)]
    for y in range(h):
        raw.append(0)
        for x in range(w):
            raw += bytes(((base[0] + x) % 256, (base[1] + y) % 256, (base[2] + (x ^ y)) % 256))

    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + \
        chunk(b"IDAT", zlib.compress(bytes(raw), 6)) + chunk(b"IEND", b"")


def sentence(rng, n):
    return " ".join(rng.choice(WORDS) for _ in range(n)).capitalize() + "."


CORE = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<cp:coreProperties '
        'xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
        'xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" '
        'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{t}</dc:title><dc:creator>ebrc-gen</dc:creator>'
        '<dcterms:created xsi:type="dcterms:W3CDTF">2026-01-01T00:00:00Z</dcterms:created>'
        '<dcterms:modified xsi:type="dcterms:W3CDTF">2026-01-01T00:00:00Z</dcterms:modified></cp:coreProperties>')
RELS_ROOT = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
             '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="{main}"/>'
             '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>'
             '</Relationships>')


def xlsx(rng, rows, cols):
    strings = [sentence(rng, 3) for _ in range(40)]
    sst = "".join(f"<si><t>{escape(s)}</t></si>" for s in strings)
    data = []
    for r in range(1, rows + 1):
        cells = [f'<c r="A{r}" t="s"><v>{rng.randrange(len(strings))}</v></c>']
        for c in range(1, cols):
            col = chr(ord("A") + c)
            cells.append(f'<c r="{col}{r}"><v>{rng.randrange(10**6) / 100}</v></c>')
        data.append(f'<row r="{r}">{"".join(cells)}</row>')
    parts = {
        "[Content_Types].xml": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                                '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                                '<Default Extension="xml" ContentType="application/xml"/>'
                                '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
                                '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
                                '<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>'
                                '<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>').encode(),
        "_rels/.rels": RELS_ROOT.format(main="xl/workbook.xml").encode(),
        "docProps/core.xml": CORE.format(t="workbook").encode(),
        "xl/workbook.xml": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
                            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>').encode(),
        "xl/_rels/workbook.xml.rels": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                                       '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>'
                                       '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>'
                                       '</Relationships>').encode(),
        "xl/sharedStrings.xml": (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
                                 f'count="{len(strings)}" uniqueCount="{len(strings)}">{sst}</sst>').encode(),
        "xl/worksheets/sheet1.xml": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
                                     f'<sheetData>{"".join(data)}</sheetData></worksheet>').encode(),
    }
    return zpack(parts)


def docx(rng, paras, embed_xlsx=None, images=0):
    body = []
    for i in range(paras):
        body.append(f"<w:p><w:r><w:t>{escape(sentence(rng, rng.randrange(8, 40)))}</w:t></w:r></w:p>")
        if i % 25 == 24:
            rows = "".join("<w:tr>" + "".join(f"<w:tc><w:p><w:r><w:t>{rng.randrange(10**5)}</w:t></w:r></w:p></w:tc>"
                                              for _ in range(4)) + "</w:tr>" for _ in range(6))
            body.append(f"<w:tbl>{rows}</w:tbl>")
    rels = ['<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>']
    parts = {}
    for k in range(images):
        parts[f"word/media/image{k + 1}.png"] = png(rng, 160 + 40 * k, 120)
        rels.append(f'<Relationship Id="rIdImg{k + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image{k + 1}.png"/>')
    if embed_xlsx:
        parts["word/embeddings/Microsoft_Excel_Worksheet.xlsx"] = embed_xlsx
        rels.append('<Relationship Id="rIdPkg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="embeddings/Microsoft_Excel_Worksheet.xlsx"/>')
    parts.update({
        "[Content_Types].xml": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                                '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                                '<Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/>'
                                '<Default Extension="xlsx" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"/>'
                                '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
                                '<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>'
                                '<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>').encode(),
        "_rels/.rels": RELS_ROOT.format(main="word/document.xml").encode(),
        "docProps/core.xml": CORE.format(t="report").encode(),
        "word/document.xml": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
                              f'<w:body>{"".join(body)}</w:body></w:document>').encode(),
        "word/styles.xml": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
                            + "".join(f'<w:style w:type="paragraph" w:styleId="S{i}"><w:name w:val="Style {i}"/></w:style>' for i in range(60))
                            + "</w:styles>").encode(),
        "word/_rels/document.xml.rels": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                                         + "".join(rels) + "</Relationships>").encode(),
    })
    return zpack(parts)


def pptx(rng, slides, embed_docx):
    parts = {"[Content_Types].xml": ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                                     '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                                     '<Default Extension="xml" ContentType="application/xml"/>'
                                     '<Default Extension="docx" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document"/>'
                                     '<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>'
                                     + "".join(f'<Override PartName="/ppt/slides/slide{i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>' for i in range(slides))
                                     + "</Types>").encode(),
             "_rels/.rels": RELS_ROOT.format(main="ppt/presentation.xml").encode(),
             "docProps/core.xml": CORE.format(t="deck").encode(),
             "ppt/embeddings/Microsoft_Word_Document.docx": embed_docx}
    sld_ids = "".join(f'<p:sldId id="{256 + i}" r:id="rId{i + 1}"/>' for i in range(slides))
    parts["ppt/presentation.xml"] = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
                                     f'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst>{sld_ids}</p:sldIdLst></p:presentation>').encode()
    parts["ppt/_rels/presentation.xml.rels"] = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                                                + "".join(f'<Relationship Id="rId{i + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{i + 1}.xml"/>' for i in range(slides))
                                                + "</Relationships>").encode()
    for i in range(slides):
        texts = "".join(f"<a:p><a:r><a:t>{escape(sentence(rng, 7))}</a:t></a:r></a:p>" for _ in range(6))
        parts[f"ppt/slides/slide{i + 1}.xml"] = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
                                                 'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:sp><p:txBody>'
                                                 f"{texts}</p:txBody></p:sp></p:spTree></p:cSld></p:sld>").encode()
    return zpack(parts)


def odf(rng, kind, embed_ods=None):
    mt = {"odt": "application/vnd.oasis.opendocument.text", "ods": "application/vnd.oasis.opendocument.spreadsheet"}[kind]
    if kind == "odt":
        body = "".join(f"<text:p>{escape(sentence(rng, rng.randrange(8, 30)))}</text:p>" for _ in range(300))
        inner = f"<office:text>{body}</office:text>"
    else:
        rows = "".join("<table:table-row>" + "".join(f'<table:table-cell office:value-type="float" office:value="{rng.randrange(10**6)}"/>'
                                                      for _ in range(8)) + "</table:table-row>" for _ in range(400))
        inner = f'<office:spreadsheet><table:table table:name="Sheet1">{rows}</table:table></office:spreadsheet>'
    ns = ('xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" '
          'xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3"')
    parts = {"mimetype": mt.encode(),
             "content.xml": f'<?xml version="1.0" encoding="UTF-8"?>\n<office:document-content {ns}><office:body>{inner}</office:body></office:document-content>'.encode(),
             "styles.xml": f'<?xml version="1.0" encoding="UTF-8"?>\n<office:document-styles {ns}/>'.encode(),
             "meta.xml": (f'<?xml version="1.0" encoding="UTF-8"?>\n<office:document-meta {ns} xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0">'
                          '<office:meta><meta:creation-date>2026-01-01T00:00:00</meta:creation-date><meta:generator>ebrc-gen</meta:generator></office:meta></office:document-meta>').encode()}
    entries = [f'<manifest:file-entry manifest:full-path="/" manifest:media-type="{mt}"/>']
    for n in ("content.xml", "styles.xml", "meta.xml"):
        entries.append(f'<manifest:file-entry manifest:full-path="{n}" manifest:media-type="text/xml"/>')
    if embed_ods:
        with zipfile.ZipFile(io.BytesIO(embed_ods)) as z:
            for n in z.namelist():
                if n != "mimetype" and not n.startswith("META-INF"):
                    parts[f"Object 1/{n}"] = z.read(n)
        entries.append('<manifest:file-entry manifest:full-path="Object 1/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>')
        parts["Pictures/thumb.png"] = png(rng, 64, 64)
    parts["META-INF/manifest.xml"] = ('<?xml version="1.0" encoding="UTF-8"?>\n<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3">'
                                      + "".join(entries) + "</manifest:manifest>").encode()
    return zpack(parts, first_stored="mimetype")


def epub(rng, chapters):
    parts = {"mimetype": b"application/epub+zip",
             "META-INF/container.xml": b'<?xml version="1.0"?>\n<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>'}
    items, spine = [], []
    for i in range(chapters):
        paras = "".join(f"<p>{escape(sentence(rng, rng.randrange(10, 50)))}</p>" for _ in range(120))
        parts[f"OEBPS/ch{i:02d}.xhtml"] = f'<?xml version="1.0" encoding="utf-8"?>\n<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Chapter {i}</title></head><body>{paras}</body></html>'.encode()
        items.append(f'<item id="c{i}" href="ch{i:02d}.xhtml" media-type="application/xhtml+xml"/>')
        spine.append(f'<itemref idref="c{i}"/>')
    parts["OEBPS/cover.png"] = png(rng, 300, 400)
    parts["OEBPS/content.opf"] = ('<?xml version="1.0" encoding="utf-8"?>\n<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">'
                                  '<metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="id">urn:uuid:00000000-0000-4000-8000-000000000001</dc:identifier>'
                                  '<dc:title>Generated Book</dc:title><dc:language>en</dc:language><meta property="dcterms:modified">2026-01-01T00:00:00Z</meta></metadata>'
                                  f'<manifest>{"".join(items)}<item id="cover" href="cover.png" media-type="image/png"/></manifest><spine>{"".join(spine)}</spine></package>').encode()
    return zpack(parts, first_stored="mimetype")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    a = ap.parse_args()
    p = json.loads(a.params)
    if p.get("profile", "office-v1") != "office-v1":
        raise SystemExit("unknown profile")
    rng = random.Random(a.seed)

    def write(rel, data):
        path = os.path.join(a.out, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)
        os.chmod(path, 0o644)

    sheets = []
    for i in range(6):
        x = xlsx(rng, rng.randrange(200, 1500), 8)
        sheets.append(x)
        write(f"sheets/data-{i:02d}.xlsx", x)
    docs = []
    for i in range(10):
        d = docx(rng, rng.randrange(100, 900), embed_xlsx=sheets[i % len(sheets)] if i % 3 == 0 else None, images=i % 4)
        docs.append(d)
        write(f"docs/report-{i:02d}.docx", d)
    for i in range(4):
        write(f"slides/deck-{i:02d}.pptx", pptx(rng, rng.randrange(8, 30), docs[i]))
    for i in range(3):
        ods = odf(rng, "ods")
        write(f"odf/calc-{i:02d}.ods", ods)
        write(f"odf/text-{i:02d}.odt", odf(rng, "odt", embed_ods=ods))
    write("books/book.epub", epub(rng, 12))


if __name__ == "__main__":
    main()
