//! Excel rendering of a `Report` with the same design rules as the PDF:
//! Times-metric type, boxed or ruled tables, shaded headers, ruled totals,
//! A4 print setup with page numbers. Figures are real numbers (unit-scaled).

use lc_core::report::{Align, Block, Layout, Report, RowStyle, Section, Table};
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet};

const FONT: &str = "Times New Roman";

fn x<T>(r: Result<T, rust_xlsxwriter::XlsxError>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

struct Style {
    boxed: bool,
    decimals: u8,
}

impl Style {
    fn base(&self) -> Format {
        Format::new()
            .set_font_name(FONT)
            .set_font_size(10)
            .set_align(FormatAlign::VerticalCenter)
    }
    fn num_format(&self) -> &'static str {
        if self.decimals == 0 {
            "#,##0;(#,##0);\"-\""
        } else {
            "#,##0.00;(#,##0.00);\"-\""
        }
    }
    fn header(&self, a: Align) -> Format {
        let f = self.base().set_bold().set_text_wrap().set_align(xa(a));
        if self.boxed {
            f.set_background_color(Color::RGB(0xEEEEEE))
                .set_border(FormatBorder::Thin)
                .set_border_bottom(FormatBorder::Medium)
                .set_border_top(FormatBorder::Medium)
        } else {
            f.set_border_top(FormatBorder::Medium)
                .set_border_bottom(FormatBorder::Thin)
        }
    }
    fn cell(&self, style: RowStyle, a: Align, numeric: bool, indent: u8) -> Format {
        let mut f = self.base().set_align(xa(a));
        if numeric {
            f = f.set_num_format(self.num_format());
        } else if a == Align::Left {
            f = f.set_text_wrap();
        }
        if indent > 0 && a == Align::Left {
            f = f.set_indent(indent);
        }
        if matches!(
            style,
            RowStyle::Heading | RowStyle::Subheading | RowStyle::Subtotal | RowStyle::Total
        ) {
            f = f.set_bold();
        }
        if style == RowStyle::Remark {
            f = f.set_italic().set_font_size(9);
        }
        if self.boxed {
            f = f.set_border(FormatBorder::Thin);
            if style == RowStyle::Total {
                f = f
                    .set_border_top(FormatBorder::Medium)
                    .set_border_bottom(FormatBorder::Medium);
            }
        } else {
            match style {
                RowStyle::Subtotal => f = f.set_border_top(FormatBorder::Thin),
                RowStyle::Total => {
                    f = f.set_border_top(FormatBorder::Medium);
                    if numeric {
                        f = f.set_border_bottom(FormatBorder::Double);
                    }
                }
                _ => {}
            }
        }
        f
    }
}

fn xa(a: Align) -> FormatAlign {
    match a {
        Align::Left => FormatAlign::Left,
        Align::Center => FormatAlign::Center,
        Align::Right => FormatAlign::Right,
    }
}

fn sheet_name(s: &Section) -> String {
    match s.id.as_str() {
        "balance_sheet" => "Balance Sheet".into(),
        "profit_and_loss" => "Profit and Loss".into(),
        "cash_flow" => "Cash Flow".into(),
        "notes" => "Notes".into(),
        "tax_depreciation" => "Tax Depreciation".into(),
        other => other.chars().take(31).collect(),
    }
}

fn width_chars(w: &str, fr: f64) -> f64 {
    if let Some(mm) = w.strip_suffix("mm") {
        mm.parse::<f64>().unwrap_or(30.0) / 1.9
    } else {
        fr
    }
}

fn table(
    ws: &mut Worksheet,
    st: &Style,
    t: &Table,
    row: &mut u32,
    ncols: u16,
) -> Result<(), String> {
    if let Some(title) = &t.title {
        x(ws.write_string_with_format(*row, 0, title, &st.base().set_bold().set_italic()))?;
        *row += 1;
    }
    // Header with merged cells (colspan / rowspan).
    let hstart = *row;
    let mut occupied: Vec<Vec<bool>> = vec![vec![false; ncols as usize + 1]; t.header.len()];
    for (hi, hr) in t.header.iter().enumerate() {
        let mut c: u16 = 0;
        for h in hr {
            while (c as usize) < occupied[hi].len() && occupied[hi][c as usize] {
                c += 1;
            }
            let r0 = hstart + hi as u32;
            let r1 = r0 + h.rowspan.max(1) as u32 - 1;
            let c1 = c + h.colspan.max(1) - 1;
            let f = st.header(h.align);
            if r1 > r0 || c1 > c {
                x(ws.merge_range(r0, c, r1, c1, &h.text, &f))?;
            } else {
                x(ws.write_string_with_format(r0, c, &h.text, &f))?;
            }
            let span_end = (hi + h.rowspan.max(1) as usize).min(t.header.len());
            for occ in occupied.iter_mut().take(span_end).skip(hi) {
                for cc in c..=c1 {
                    if let Some(slot) = occ.get_mut(cc as usize) {
                        *slot = true;
                    }
                }
            }
            c = c1 + 1;
        }
    }
    *row += t.header.len() as u32;
    let n = t.columns.len() as u16;
    for r in &t.rows {
        if r.style == RowStyle::Remark {
            let f = st.cell(r.style, Align::Left, false, 0);
            if n > 1 {
                x(ws.merge_range(*row, 0, *row, n - 1, &r.cells[0], &f))?;
            } else {
                x(ws.write_string_with_format(*row, 0, &r.cells[0], &f))?;
            }
            *row += 1;
            continue;
        }
        for (i, cell) in r.cells.iter().enumerate() {
            let a = t.columns[i].align;
            let v = r.values.get(i).copied().flatten();
            let numeric = v.is_some() && !cell.ends_with('%') && cell != "N.A.";
            let f = st.cell(r.style, a, numeric, if i == 0 { r.indent } else { 0 });
            match (numeric, v) {
                (true, Some(v)) => x(ws.write_number_with_format(*row, i as u16, v, &f))?,
                _ => x(ws.write_string_with_format(*row, i as u16, cell, &f))?,
            };
        }
        *row += 1;
    }
    *row += 1;
    Ok(())
}

pub fn render(r: &Report) -> Result<Vec<u8>, String> {
    let st = Style {
        boxed: r.meta.layout == Layout::Boxed,
        decimals: r.meta.decimals,
    };
    let mut wb = Workbook::new();
    for s in &r.sections {
        let ws = wb.add_worksheet();
        x(ws.set_name(sheet_name(s)))?;
        let tables: Vec<&Table> = s
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Table(t) = b {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();
        let ncols = tables
            .iter()
            .map(|t| t.columns.len())
            .max()
            .unwrap_or(2)
            .max(2) as u16;
        let wide = tables.iter().any(|t| t.landscape);
        // Column widths: first column wide, others from the widest table.
        x(ws.set_column_width(0, if ncols > 6 { 34 } else { 56 }))?;
        for c in 1..ncols {
            let w = tables
                .iter()
                .filter_map(|t| t.columns.get(c as usize))
                .map(|col| width_chars(&col.width, 30.0))
                .fold(14.0_f64, f64::max);
            x(ws.set_column_width(c, w.min(32.0)))?;
        }
        let mut row = 0u32;
        let center = |f: Format| f.set_align(FormatAlign::Center);
        x(ws.merge_range(
            row,
            0,
            row,
            ncols - 1,
            &r.meta.entity.to_uppercase(),
            &center(st.base().set_bold().set_font_size(13)),
        ))?;
        row += 1;
        if !r.meta.details.is_empty() {
            x(ws.merge_range(
                row,
                0,
                row,
                ncols - 1,
                &r.meta.details.join(" | "),
                &center(st.base().set_font_size(9)),
            ))?;
            row += 1;
        }
        x(ws.merge_range(
            row,
            0,
            row,
            ncols - 1,
            &s.title,
            &center(st.base().set_bold().set_font_size(11)),
        ))?;
        row += 1;
        x(ws.merge_range(
            row,
            0,
            row,
            ncols - 1,
            &r.meta.unit_note,
            &st.base()
                .set_italic()
                .set_font_size(9)
                .set_align(FormatAlign::Right),
        ))?;
        row += 2;
        for b in &s.blocks {
            match b {
                Block::Heading { text, level } => {
                    let f = if *level == 2 {
                        st.base().set_bold().set_font_size(11)
                    } else {
                        st.base().set_bold().set_italic()
                    };
                    x(ws.write_string_with_format(row, 0, text, &f))?;
                    row += 1;
                }
                Block::Para { text } => {
                    let f = st
                        .base()
                        .set_text_wrap()
                        .set_align(FormatAlign::Top)
                        .set_font_size(9);
                    x(ws.merge_range(row, 0, row, ncols - 1, text, &f))?;
                    let lines = (text.len() as f64 / (if ncols > 6 { 160.0 } else { 105.0 }))
                        .ceil()
                        .max(1.0);
                    x(ws.set_row_height(row, 13.0 * lines))?;
                    row += 2;
                }
                Block::Table(t) => table(ws, &st, t, &mut row, ncols)?,
                Block::Chart { title, .. } => {
                    x(ws.write_string(row, 0, format!("{title} (chart: see the PDF)")))?;
                    row += 2;
                }
            }
        }
        if let Some(n) = &s.closing_note {
            x(ws.write_string_with_format(row, 0, n, &st.base().set_italic().set_font_size(9)))?;
            row += 2;
        }
        if let Some(sig) = &s.signature {
            let rc = ncols.saturating_sub(2).max(1);
            for (i, l) in sig.left.iter().enumerate() {
                x(ws.write_string_with_format(row + i as u32, 0, l, &st.base()))?;
            }
            for (i, l) in sig.right.iter().enumerate() {
                x(ws.write_string_with_format(row + i as u32, rc, l, &st.base()))?;
            }
        }
        ws.set_paper_size(9);
        if wide || ncols > 6 {
            ws.set_landscape();
        }
        ws.set_print_fit_to_pages(1, 0);
        ws.set_margins(0.6, 0.5, 0.8, 0.7, 0.3, 0.3);
        ws.set_header(format!(
            "&L&\"{FONT}\"&8{}&R&\"{FONT}\"&8{}",
            r.meta.entity.replace('&', "&&"),
            if r.meta.draft {
                if r.meta.draft_note.is_empty() {
                    "DRAFT".to_string()
                } else {
                    format!("DRAFT ({})", r.meta.draft_note)
                }
            } else {
                String::new()
            }
        ));
        ws.set_footer(format!("&C&\"{FONT}\"&8Page &P of &N"));
        ws.set_screen_gridlines(false);
    }
    x(wb.save_to_buffer())
}
