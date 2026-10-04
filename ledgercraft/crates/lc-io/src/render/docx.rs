//! Word (.docx) renderer. Same document model and design rules as the PDF:
//! A4, Times New Roman, boxed or ruled tables, merged multi-level headers,
//! wide schedules on landscape pages, page numbers, signature block.
//! It lays out; it never computes (every figure arrives formatted).

use docx_rs::*;
use lc_core::report::{Align, Block, Layout, Report, RowStyle, Section, Signature, Table};

const FONT: &str = "Times New Roman";
/// Twips per millimetre.
const MM: f64 = 56.693;
const A4_W: u32 = 11906;
const A4_H: u32 = 16838;
const M_TOP: i32 = 1247; // 22 mm
const M_BOTTOM: i32 = 1134; // 20 mm
const M_LEFT: i32 = 1134; // 20 mm (binding)
const M_RIGHT: i32 = 907; // 16 mm

fn fonts() -> RunFonts {
    RunFonts::new()
        .ascii(FONT)
        .hi_ansi(FONT)
        .cs(FONT)
        .east_asia(FONT)
}

fn run(text: &str, size_pt: f32) -> Run {
    Run::new()
        .add_text(text)
        .fonts(fonts())
        .size((size_pt * 2.0).round() as usize)
}

fn para(text: &str, size_pt: f32) -> Paragraph {
    Paragraph::new().add_run(run(text, size_pt))
}

fn align(a: Align) -> AlignmentType {
    match a {
        Align::Left => AlignmentType::Left,
        Align::Center => AlignmentType::Center,
        Align::Right => AlignmentType::Right,
    }
}

fn page(landscape: bool) -> SectionProperty {
    let (w, h, o) = if landscape {
        (A4_H, A4_W, PageOrientationType::Landscape)
    } else {
        (A4_W, A4_H, PageOrientationType::Portrait)
    };
    SectionProperty::new()
        .page_size(PageSize::new().width(w).height(h).orient(o))
        .page_orient(o)
        .page_margin(
            PageMargin::new()
                .top(M_TOP)
                .bottom(M_BOTTOM)
                .left(M_LEFT)
                .right(M_RIGHT)
                .header(567)
                .footer(567),
        )
}

fn content_width(landscape: bool) -> f64 {
    let w = if landscape { A4_H } else { A4_W } as f64;
    w - (M_LEFT + M_RIGHT) as f64
}

/// Column widths in twips from "1fr" / "34mm" specs.
fn widths(t: &Table) -> Vec<usize> {
    let total = content_width(t.landscape);
    let mut fixed = 0.0;
    let mut fr = 0.0;
    for c in &t.columns {
        if let Some(mm) = c.width.strip_suffix("mm") {
            fixed += mm.trim().parse::<f64>().unwrap_or(30.0) * MM;
        } else {
            fr += c.width.trim_end_matches("fr").parse::<f64>().unwrap_or(1.0);
        }
    }
    let rest = (total - fixed).max(total * 0.2);
    t.columns
        .iter()
        .map(|c| {
            if let Some(mm) = c.width.strip_suffix("mm") {
                (mm.trim().parse::<f64>().unwrap_or(30.0) * MM) as usize
            } else {
                let f = c.width.trim_end_matches("fr").parse::<f64>().unwrap_or(1.0);
                (rest * f / fr.max(1.0)) as usize
            }
        })
        .collect()
}

fn border(pos: TableCellBorderPosition, size: usize, kind: BorderType) -> TableCellBorder {
    TableCellBorder::new(pos)
        .size(size)
        .border_type(kind)
        .color("000000")
}

fn none(pos: TableCellBorderPosition) -> TableCellBorder {
    TableCellBorder::new(pos).border_type(BorderType::Nil)
}

/// Borders of one cell: Boxed = full grid; Ruled = lines only at the frame,
/// under the header and at subtotals / totals.
fn cell_borders(
    layout: Layout,
    style: Option<RowStyle>,
    header: bool,
    last_header_row: bool,
    first_row: bool,
    last_row: bool,
) -> TableCellBorders {
    use TableCellBorderPosition as P;
    let thin = |p| border(p, 4, BorderType::Single);
    let mut b = TableCellBorders::with_empty();
    match layout {
        Layout::Boxed => {
            for p in [P::Left, P::Right, P::Top, P::Bottom] {
                b = b.set(thin(p));
            }
        }
        Layout::Ruled => {
            b = b.set(none(P::Left)).set(none(P::Right));
            b = b.set(if first_row && header {
                thin(P::Top)
            } else {
                none(P::Top)
            });
            b = b.set(if (header && last_header_row) || last_row {
                thin(P::Bottom)
            } else {
                none(P::Bottom)
            });
        }
    }
    match style {
        Some(RowStyle::Subtotal) => b = b.set(border(P::Top, 6, BorderType::Single)),
        Some(RowStyle::Total) => {
            b = b.set(border(P::Top, 8, BorderType::Single)).set(border(
                P::Bottom,
                6,
                BorderType::Double,
            ))
        }
        _ => {}
    }
    b
}

fn table(t: &Table, layout: Layout) -> docx_rs::Table {
    let w = widths(t);
    let ncols = t.columns.len();
    let size: f32 = if t.dense { 8.5 } else { 10.0 };
    let mut rows = Vec::new();
    // Header rows with merged cells (colspan → gridSpan, rowspan → vMerge).
    let mut covered: Vec<usize> = vec![0; ncols]; // rows still covered by a rowspan
    let hn = t.header.len();
    for (ri, hrow) in t.header.iter().enumerate() {
        let mut cells = Vec::new();
        let mut col = 0;
        let mut it = hrow.iter();
        while col < ncols {
            if covered[col] > 0 {
                covered[col] -= 1;
                cells.push(
                    TableCell::new()
                        .vertical_merge(VMergeType::Continue)
                        .width(w[col], WidthType::Dxa)
                        .set_borders(cell_borders(
                            layout,
                            None,
                            true,
                            ri + 1 == hn,
                            ri == 0,
                            false,
                        ))
                        .add_paragraph(Paragraph::new()),
                );
                col += 1;
                continue;
            }
            let Some(h) = it.next() else { break };
            let span = (h.colspan.max(1) as usize).min(ncols - col);
            let width: usize = w[col..col + span].iter().sum();
            let mut cell = TableCell::new()
                .width(width, WidthType::Dxa)
                .shading(if layout == Layout::Boxed {
                    Shading::new().fill("EDEDED")
                } else {
                    Shading::new().fill("FFFFFF")
                })
                .set_borders(cell_borders(
                    layout,
                    None,
                    true,
                    ri + 1 == hn,
                    ri == 0,
                    false,
                ))
                .add_paragraph(
                    Paragraph::new()
                        .align(align(h.align))
                        .add_run(run(&h.text, size).bold()),
                );
            if span > 1 {
                cell = cell.grid_span(span);
            }
            if h.rowspan > 1 {
                cell = cell.vertical_merge(VMergeType::Restart);
                for c in covered.iter_mut().skip(col).take(span) {
                    *c = h.rowspan as usize - 1;
                }
            }
            cells.push(cell);
            col += span;
        }
        rows.push(TableRow::new(cells).cant_split());
    }
    let n = t.rows.len();
    for (ri, r) in t.rows.iter().enumerate() {
        let bold = matches!(
            r.style,
            RowStyle::Heading | RowStyle::Subheading | RowStyle::Subtotal | RowStyle::Total
        );
        let cells = r
            .cells
            .iter()
            .enumerate()
            .take(ncols)
            .map(|(ci, text)| {
                let text = if ci == 0 && r.style == RowStyle::Heading {
                    text.to_uppercase()
                } else {
                    text.clone()
                };
                let mut rn = run(
                    &text,
                    if r.style == RowStyle::Remark {
                        size - 1.0
                    } else {
                        size
                    },
                );
                if bold {
                    rn = rn.bold();
                }
                if r.style == RowStyle::Remark {
                    rn = rn.italic();
                }
                let mut p = Paragraph::new()
                    .align(align(t.columns[ci].align))
                    .add_run(rn);
                if ci == 0 && r.indent > 0 {
                    p = p.indent(Some((r.indent as f64 * 4.0 * MM) as i32), None, None, None);
                }
                let figure_style = if ci == 0 { None } else { Some(r.style) };
                TableCell::new()
                    .width(w[ci], WidthType::Dxa)
                    .set_borders(cell_borders(
                        layout,
                        figure_style
                            .filter(|s| matches!(s, RowStyle::Subtotal | RowStyle::Total))
                            .or(if layout == Layout::Boxed {
                                Some(r.style)
                            } else {
                                None
                            }),
                        false,
                        false,
                        false,
                        ri + 1 == n,
                    ))
                    .add_paragraph(p)
            })
            .collect();
        let mut row = TableRow::new(cells);
        if t.keep_together {
            row = row.cant_split();
        }
        rows.push(row);
    }
    docx_rs::Table::new(rows)
        .set_grid(w.clone())
        .layout(TableLayoutType::Fixed)
        .width(w.iter().sum(), WidthType::Dxa)
}

fn signature_table(s: &Signature) -> docx_rs::Table {
    let half = (content_width(false) / 2.0) as usize;
    let cell = |lines: &[String]| {
        let mut c = TableCell::new()
            .width(half, WidthType::Dxa)
            .set_borders(TableCellBorders::with_empty());
        for (i, l) in lines.iter().enumerate() {
            let mut r = run(l, 10.0);
            if i == 0 {
                r = r.bold();
            }
            c = c.add_paragraph(Paragraph::new().add_run(r));
        }
        c
    };
    docx_rs::Table::without_borders(vec![
        TableRow::new(vec![cell(&s.left), cell(&s.right)]).cant_split()
    ])
    .set_grid(vec![half, half])
    .width(half * 2, WidthType::Dxa)
}

fn title_block(d: Docx, r: &Report, s: &Section, first: bool) -> Docx {
    let m = &r.meta;
    let mut p = Paragraph::new()
        .align(AlignmentType::Center)
        .add_run(run(&m.entity.to_uppercase(), 13.0).bold());
    if !first {
        p = p.page_break_before(true);
    }
    let mut d = d.add_paragraph(p);
    if !m.details.is_empty() {
        d = d.add_paragraph(para(&m.details.join(" | "), 8.5).align(AlignmentType::Center));
    }
    d.add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .add_run(run(&s.title, 11.5).bold()),
    )
    .add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Right)
            .add_run(run(&m.unit_note, 9.0).italic()),
    )
}

/// The report as a Word document.
pub fn render(r: &Report) -> Result<Vec<u8>, String> {
    let m = &r.meta;
    let mut d = Docx::new()
        .default_fonts(fonts())
        .default_size(20)
        .page_size(A4_W, A4_H)
        .page_margin(
            PageMargin::new()
                .top(M_TOP)
                .bottom(M_BOTTOM)
                .left(M_LEFT)
                .right(M_RIGHT)
                .header(567)
                .footer(567),
        );
    let mut foot = Paragraph::new().align(AlignmentType::Center);
    if m.draft {
        let note = if m.draft_note.is_empty() {
            "Draft – for discussion only    ".to_string()
        } else {
            format!("Draft – for discussion only · {}    ", m.draft_note)
        };
        foot = foot.add_run(run(&note, 8.0));
    }
    foot = foot
        .add_run(run("Page ", 8.0))
        .add_page_num(PageNum::new())
        .add_run(run(" of ", 8.0))
        .add_num_pages(NumPages::new());
    d = d.footer(Footer::new().add_paragraph(foot));
    if m.draft {
        d = d.header(
            Header::new().add_paragraph(para("DRAFT", 9.0).align(AlignmentType::Right).bold()),
        );
    }

    let mut first = true;
    if m.cover {
        d = d
            .add_paragraph(para("", 12.0))
            .add_paragraph(para("", 12.0))
            .add_paragraph(
                Paragraph::new()
                    .align(AlignmentType::Center)
                    .add_run(run(&m.entity.to_uppercase(), 18.0).bold()),
            );
        if !m.details.is_empty() {
            d = d.add_paragraph(para(&m.details.join(" | "), 10.0).align(AlignmentType::Center));
        }
        d = d
            .add_paragraph(para(&m.title, 13.0).align(AlignmentType::Center))
            .add_paragraph(
                para(&format!("Prepared in the format: {}", m.format_name), 9.0)
                    .align(AlignmentType::Center),
            )
            .add_paragraph(
                Paragraph::new()
                    .page_break_before(true)
                    .add_run(run("Contents", 12.0).bold()),
            );
        for s in r.sections.iter().filter(|s| s.contents) {
            d = d.add_paragraph(para(&s.title, 10.0));
        }
        first = false;
    }

    for s in &r.sections {
        d = title_block(d, r, s, first);
        first = false;
        for b in &s.blocks {
            match b {
                Block::Heading { text, level } => {
                    let size = if *level <= 2 { 10.5 } else { 10.0 };
                    d = d.add_paragraph(
                        Paragraph::new()
                            .keep_next(true)
                            .add_run(run(text, size).bold()),
                    );
                }
                Block::Para { text } => {
                    d = d.add_paragraph(para(text, 10.0));
                }
                Block::Table(t) if t.landscape => {
                    // Close the portrait section, put the schedule on landscape pages.
                    d = d.add_paragraph(Paragraph::new().section_property(page(false)));
                    if let Some(title) = &t.title {
                        d = d.add_paragraph(
                            Paragraph::new()
                                .keep_next(true)
                                .add_run(run(title, 10.0).bold()),
                        );
                    }
                    d = d.add_table(table(t, m.layout));
                    d = d.add_paragraph(Paragraph::new().section_property(page(true)));
                }
                Block::Table(t) => {
                    if let Some(title) = &t.title {
                        d = d.add_paragraph(
                            Paragraph::new()
                                .keep_next(true)
                                .add_run(run(title, 10.0).bold()),
                        );
                    }
                    d = d.add_table(table(t, m.layout)).add_paragraph(para("", 6.0));
                }
            }
        }
        if let Some(n) = &s.closing_note {
            d = d.add_paragraph(Paragraph::new().add_run(run(n, 9.0).italic()));
        }
        if let Some(sig) = &s.signature {
            d = d
                .add_paragraph(para("", 10.0))
                .add_table(signature_table(sig));
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    d.build().pack(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}
