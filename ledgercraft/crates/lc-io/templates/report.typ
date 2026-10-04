// LedgerCraft – design system for printed financial statements.
//
// This file holds ALL visual decisions (page, type, rules, spacing). Content
// arrives as JSON (the `Report` model) and is laid out here unchanged:
// figures are pre-formatted and already cast, so the template never computes.

#let R = json(bytes(sys.inputs.data))
#let M = R.meta
#let boxed = M.layout == "boxed"

// ---- Tokens -------------------------------------------------------------
#let ink = black
#let hair = 0.4pt + ink
#let firm = 0.9pt + ink
#let header-fill = if boxed { luma(238) } else { none }
#let body-size = 10pt
#let dense-size = 8.5pt
#let small = 8.5pt
#let indent-step = 4mm

#set document(title: M.entity + " – " + M.title, author: M.generator)
#set text(font: ("Liberation Serif", "DejaVu Serif"), size: body-size, lang: "en", hyphenate: false, fill: ink)
#set par(justify: true, leading: 0.6em, spacing: 0.9em)

// ---- Page furniture -----------------------------------------------------
#let section-marks() = query(<lc-section>)
#let running-header = context {
  let p = here().page()
  let marks = section-marks().filter(m => m.location().page() <= p)
  if marks.len() == 0 { return }
  let m = marks.last()
  if m.location().page() == p { return }   // first page of a section shows the full title block
  set text(size: 8pt)
  grid(columns: (1fr, auto), align: (left, right), M.entity, [#m.value (continued)])
  v(-6pt)
  line(length: 100%, stroke: 0.3pt)
}
#let footer = context {
  set text(size: 8pt)
  let p = counter(page).display()
  let total = counter(page).final().first()
  grid(columns: (1fr, auto, 1fr), align: (left, center, right),
    if M.draft [Draft – for discussion only#if M.at("draft_note", default: "") != "" [ · #M.draft_note]] else [],
    [Page #p of #total],
    [])
}
#set page(
  paper: "a4",
  margin: (top: 22mm, bottom: 20mm, left: 20mm, right: 16mm),
  header: running-header,
  footer: footer,
  background: if M.draft { rotate(-38deg, text(96pt, weight: "bold", fill: luma(236))[DRAFT]) },
)

// ---- Helpers ------------------------------------------------------------
#let width(w) = {
  if w.ends-with("fr") { float(w.slice(0, -2)) * 1fr }
  else if w.ends-with("mm") { float(w.slice(0, -2)) * 1mm }
  else { auto }
}
#let al(a) = if a == "right" { right + horizon } else if a == "center" { center + horizon } else { left + horizon }
#let double-rule(body) = underline(offset: 2.2pt, stroke: 0.5pt, underline(offset: 0.9pt, stroke: 0.5pt, body))

#let entity-block(title) = {
  align(center)[
    #text(13pt, weight: "bold")[#upper(M.entity)]
    #if M.details.len() > 0 [ \ #text(8.5pt)[#M.details.join(" | ")] ]
    #v(2pt)
    #text(11.5pt, weight: "bold")[#title]
  ]
  v(1pt)
  align(right, text(small, style: "italic", M.unit_note))
}

#let render-table(t) = {
  set par(justify: false)
  let ncols = t.columns.len()
  let size = if t.dense { dense-size } else { body-size }
  let ins = if t.dense { (x: 3pt, y: 3pt) } else { (x: 4pt, y: 3.6pt) }
  let header-rows = t.header.len()
  let cells = ()
  // Header (repeats on every page the table runs to).
  let hcells = ()
  for hr in t.header {
    for h in hr {
      hcells.push(table.cell(colspan: h.colspan, rowspan: h.rowspan, align: al(h.align), fill: header-fill,
        text(weight: "bold", size: size - 0.5pt, h.text)))
    }
  }
  let body = ()
  let hlines = ()
  let y = header-rows
  for r in t.rows {
    let st = r.style
    if st == "remark" {
      body.push(table.cell(colspan: ncols, align: left, text(size: size - 1pt, style: "italic", r.cells.at(0))))
    } else {
      for (i, c) in r.cells.enumerate() {
        let a = al(t.columns.at(i).align)
        let content = if i == 0 and r.indent > 0 { pad(left: r.indent * indent-step, c) } else { c }
        let w = if st in ("heading", "subheading", "subtotal", "total") { "bold" } else { "regular" }
        let shown = if st == "total" and not boxed and i > 0 and c != "" { double-rule(content) } else { content }
        body.push(table.cell(align: a, text(weight: w, size: size, shown)))
      }
    }
    if st == "subtotal" { hlines.push(table.hline(y: y, stroke: hair)) }
    if st == "total" {
      hlines.push(table.hline(y: y, stroke: firm))
      if boxed { hlines.push(table.hline(y: y + 1, stroke: firm)) }
    }
    y += 1
  }
  let stroke = if boxed {
    (x, y) => (left: hair, right: hair, top: if y == 0 { firm } else { hair }, bottom: if y == header-rows - 1 { firm } else { hair })
  } else {
    (x, y) => (top: if y == 0 { firm } else { none }, bottom: if y == header-rows - 1 { hair } else { none })
  }
  let tbl = table(
    columns: t.columns.map(c => width(c.width)),
    stroke: stroke,
    inset: ins,
    table.header(repeat: true, ..hcells),
    ..hlines,
    ..body,
    ..if not boxed { (table.hline(stroke: firm),) } else { () },
  )
  let titled = if t.title != none {
    stack(spacing: 3pt, text(size: size, weight: "bold", style: "italic", t.title), tbl)
  } else { tbl }
  if t.keep_together { block(breakable: false, width: 100%, titled) } else { titled }
}

#let render-signature(s) = block(breakable: false, above: 14pt, {
  set text(size: 9.5pt)
  set par(justify: false, spacing: 0pt, leading: 0.45em)
  grid(columns: (1fr, 1fr), column-gutter: 16mm,
    align(left, s.left.map(l => if l == "" [#v(9pt)] else [#l \ ]).join()),
    align(left, s.right.map(l => if l == "" [#v(9pt)] else [#l \ ]).join()))
})

#let render-block(b) = {
  if b.kind == "heading" {
    if b.level == 2 {
      block(sticky: true, above: 12pt, below: 5pt, text(10.5pt, weight: "bold", b.text))
    } else {
      block(sticky: true, above: 7pt, below: 3pt, text(10pt, weight: "bold", style: "italic", b.text))
    }
  } else if b.kind == "para" {
    block(above: 4pt, below: 6pt, text(9.5pt, b.text))
  } else {
    block(above: 4pt, below: 8pt, render-table(b))
  }
}

// Group blocks by note heading so a note that needs a landscape page moves
// there together with its title.
#let render-blocks(blocks, tail) = {
  let groups = ()
  let cur = ()
  for b in blocks {
    if b.kind == "heading" and b.level == 2 and cur.len() > 0 {
      groups.push(cur)
      cur = ()
    }
    cur.push(b)
  }
  if cur.len() > 0 { groups.push(cur) }
  // The closing note and signature travel with the last group (no orphan page).
  for (gi, g) in groups.enumerate() {
    let last = gi == groups.len() - 1
    let wide = g.any(b => b.kind == "table" and b.landscape)
    let body = { for b in g { render-block(b) }; if last { tail } }
    if wide { page(flipped: true, body) } else { body }
  }
  if groups.len() == 0 { tail }
}

// ---- Cover and contents ---------------------------------------------------
#if M.cover {
  page(header: none, footer: none, {
    v(1fr)
    align(center)[
      #text(20pt, weight: "bold")[#upper(M.entity)]
      #if M.details.len() > 0 [ #v(4pt) #text(10pt)[#M.details.join(" | ")] ]
      #v(10pt)
      #line(length: 40%, stroke: firm)
      #v(10pt)
      #text(14pt)[#M.title]
    ]
    v(1.4fr)
    align(center, text(8.5pt, fill: luma(90))[Prepared in the format: #M.format_name])
  })
  page({
    align(center, text(13pt, weight: "bold")[Contents])
    v(8pt)
    let secs = R.sections.filter(s => s.contents)
    context {
      let marks = query(<lc-section>)
      table(
        columns: (1fr, 20mm),
        stroke: if boxed { hair } else { (x, y) => (bottom: hair) },
        inset: (x: 5pt, y: 5pt),
        table.header(table.cell(fill: header-fill)[*Particulars*], table.cell(fill: header-fill, align: right)[*Page*]),
        ..secs.map(s => {
          let m = marks.filter(x => x.value == s.title)
          let pg = if m.len() > 0 { str(counter(page).at(m.first().location()).first()) } else { "" }
          (s.title, align(right, pg))
        }).flatten()
      )
    }
  })
}

// ---- Sections ------------------------------------------------------------
#for (i, s) in R.sections.enumerate() {
  if i > 0 or M.cover { pagebreak(weak: true) }
  [#metadata(s.title) <lc-section>]
  entity-block(s.title)
  v(4pt)
  render-blocks(s.blocks, {
    if s.closing_note != none {
      block(above: 6pt, text(9pt, style: "italic", s.closing_note))
    }
    if s.signature != none { render-signature(s.signature) }
  })
}
