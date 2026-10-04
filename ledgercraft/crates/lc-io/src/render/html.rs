//! Self-contained HTML rendering of a `Report` (desktop preview / browser print).
//! Mirrors the PDF design tokens: same fonts, rules, spacing and two layouts.

use lc_core::report::{Align, Block, Layout, Report, RowStyle, Table};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn al(a: Align) -> &'static str {
    match a {
        Align::Left => "l",
        Align::Center => "c",
        Align::Right => "r",
    }
}

fn table(t: &Table, h: &mut String) {
    h.push_str(&format!(
        "<figure class=\"tbl{}{}\">",
        if t.dense { " dense" } else { "" },
        if t.landscape { " wide" } else { "" }
    ));
    if let Some(title) = &t.title {
        h.push_str(&format!("<figcaption>{}</figcaption>", esc(title)));
    }
    h.push_str("<table><colgroup>");
    for c in &t.columns {
        if c.width.ends_with("mm") {
            h.push_str(&format!("<col style=\"width:{}\">", c.width));
        } else {
            h.push_str("<col>");
        }
    }
    h.push_str("</colgroup><thead>");
    for hr in &t.header {
        h.push_str("<tr>");
        for c in hr {
            h.push_str(&format!(
                "<th class=\"{}\" colspan=\"{}\" rowspan=\"{}\">{}</th>",
                al(c.align),
                c.colspan,
                c.rowspan,
                esc(&c.text)
            ));
        }
        h.push_str("</tr>");
    }
    h.push_str("</thead><tbody>");
    let n = t.columns.len();
    for r in &t.rows {
        let cls = match r.style {
            RowStyle::Heading => "heading",
            RowStyle::Subheading => "sub",
            RowStyle::Item => "item",
            RowStyle::Subtotal => "subtotal",
            RowStyle::Total => "total",
            RowStyle::Remark => "remark",
        };
        h.push_str(&format!("<tr class=\"{cls}\">"));
        if r.style == RowStyle::Remark {
            h.push_str(&format!("<td colspan=\"{n}\">{}</td>", esc(&r.cells[0])));
        } else {
            for (i, c) in r.cells.iter().enumerate() {
                let pad = if i == 0 && r.indent > 0 {
                    format!(" style=\"padding-left:{}mm\"", 1.4 + 4.0 * r.indent as f32)
                } else {
                    String::new()
                };
                h.push_str(&format!(
                    "<td class=\"{}\"{pad}><span>{}</span></td>",
                    al(t.columns[i].align),
                    esc(c)
                ));
            }
        }
        h.push_str("</tr>");
    }
    h.push_str("</tbody></table></figure>");
}

pub fn render(r: &Report) -> String {
    let m = &r.meta;
    let mut h = String::new();
    h.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    h.push_str(&format!(
        "<title>{} – {}</title><style>{}</style></head>",
        esc(&m.entity),
        esc(&m.title),
        CSS
    ));
    h.push_str(&format!(
        "<body class=\"{}{}\">",
        if m.layout == Layout::Boxed {
            "boxed"
        } else {
            "ruled"
        },
        if m.draft { " draft" } else { "" }
    ));
    if m.draft && !m.draft_note.is_empty() {
        h.push_str(&format!(
            "<p class=\"draftnote\">Draft: {}. A final copy is available once they are resolved.</p>",
            esc(&m.draft_note)
        ));
    }
    if m.cover {
        h.push_str(&format!(
            "<section class=\"page cover\"><h1>{}</h1>{}<hr><p class=\"title\">{}</p><p class=\"fmt\">Prepared in the format: {}</p></section>",
            esc(&m.entity.to_uppercase()),
            if m.details.is_empty() { String::new() } else { format!("<p class=\"details\">{}</p>", esc(&m.details.join(" | "))) },
            esc(&m.title),
            esc(&m.format_name)
        ));
    }
    for s in &r.sections {
        h.push_str(&format!(
            "<section class=\"page\" id=\"{}\"><header><p class=\"entity\">{}</p>",
            esc(&s.id),
            esc(&m.entity.to_uppercase())
        ));
        if !m.details.is_empty() {
            h.push_str(&format!(
                "<p class=\"details\">{}</p>",
                esc(&m.details.join(" | "))
            ));
        }
        h.push_str(&format!(
            "<h2>{}</h2><p class=\"unit\">{}</p></header>",
            esc(&s.title),
            esc(&m.unit_note)
        ));
        for b in &s.blocks {
            match b {
                Block::Heading { text, level } => h.push_str(&format!(
                    "<h{} class=\"note\">{}</h{}>",
                    level + 1,
                    esc(text),
                    level + 1
                )),
                Block::Para { text } => h.push_str(&format!("<p class=\"para\">{}</p>", esc(text))),
                Block::Table(t) => table(t, &mut h),
                Block::Chart { svg, .. } => {
                    h.push_str("<div class=\"chart\">");
                    h.push_str(svg);
                    h.push_str("</div>");
                }
            }
        }
        if let Some(n) = &s.closing_note {
            h.push_str(&format!("<p class=\"closing\">{}</p>", esc(n)));
        }
        if let Some(sig) = &s.signature {
            let col = |v: &Vec<String>| {
                v.iter()
                    .map(|l| {
                        if l.is_empty() {
                            "<br>".to_string()
                        } else {
                            format!("{}<br>", esc(l))
                        }
                    })
                    .collect::<String>()
            };
            h.push_str(&format!(
                "<div class=\"sig\"><div>{}</div><div>{}</div></div>",
                col(&sig.left),
                col(&sig.right)
            ));
        }
        h.push_str("</section>");
    }
    h.push_str("</body></html>");
    h
}

const CSS: &str = r#"
:root{--ink:#000;--paper:#fff;--rule:#000;--head:#eeeeee;--muted:#555}
@page{size:A4;margin:22mm 16mm 20mm 20mm}
*{box-sizing:border-box}
body{margin:0;background:#d9d9d9;color:var(--ink);font-family:"Liberation Serif","Times New Roman",Times,"DejaVu Serif",serif;font-size:10pt;line-height:1.35}
.page{background:var(--paper);width:210mm;min-height:297mm;margin:12px auto;padding:22mm 16mm 20mm 20mm;position:relative}
.cover{display:flex;flex-direction:column;justify-content:center;align-items:center;text-align:center}
.cover h1{font-size:20pt;margin:0}.cover hr{width:40%;border:0;border-top:.9pt solid var(--rule);margin:14px auto}
.cover .title{font-size:14pt;margin:0}.cover .fmt{position:absolute;bottom:30mm;font-size:8.5pt;color:var(--muted)}
header{text-align:center;margin-bottom:6px}.entity{font-weight:bold;font-size:13pt;margin:0}
.details{font-size:8.5pt;margin:2px 0}h2{font-size:11.5pt;margin:3px 0 0}
.unit{text-align:right;font-style:italic;font-size:8.5pt;margin:2px 0 0}
h3.note{font-size:10.5pt;margin:14px 0 5px;break-after:avoid}h4.note{font-size:10pt;font-style:italic;margin:8px 0 3px;break-after:avoid}
.para{text-align:justify;font-size:9.5pt;margin:3px 0 7px}
figure.tbl{margin:4px 0 9px;break-inside:avoid}figure figcaption{font-weight:bold;font-style:italic;margin-bottom:3px}
table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}
th{font-weight:bold;font-size:9.5pt;padding:3.6pt 4pt;vertical-align:middle}
td{padding:3.6pt 4pt;vertical-align:middle}
.dense th,.dense td{font-size:8.5pt;padding:3pt}
.l{text-align:left}.c{text-align:center}.r{text-align:right;white-space:nowrap}
tr.heading td,tr.sub td,tr.subtotal td,tr.total td{font-weight:bold}
tr.remark td{font-style:italic;font-size:9pt}
.boxed table{border:.9pt solid var(--rule)}.boxed th{background:var(--head);border:.4pt solid var(--rule);border-bottom:.9pt solid var(--rule)}
.boxed td{border:.4pt solid var(--rule)}.boxed tr.subtotal td{border-top:.4pt solid var(--rule)}
.boxed tr.total td{border-top:.9pt solid var(--rule);border-bottom:.9pt solid var(--rule)}
.ruled table{border-top:.9pt solid var(--rule);border-bottom:.9pt solid var(--rule)}.ruled thead tr:last-child th{border-bottom:.4pt solid var(--rule)}
.ruled tr.subtotal td{border-top:.4pt solid var(--rule)}.ruled tr.total td{border-top:.9pt solid var(--rule)}
.ruled tr.total td.r span{border-bottom:3px double var(--rule)}
.closing{font-style:italic;font-size:9pt;margin-top:6px}
.sig{display:grid;grid-template-columns:1fr 1fr;gap:16mm;margin-top:16px;font-size:9.5pt;break-inside:avoid}
.draftnote{max-width:210mm;margin:12px auto 0;padding:8px 12px;background:#fff4dc;border:1px solid #e8c77a;border-radius:6px;font:13px/1.4 sans-serif;color:#6b4a00}
.chart{margin:8px 0 16px}.chart svg{max-width:100%;height:auto}
.draft .page::before{content:"DRAFT";position:absolute;top:40%;left:18%;font-size:90pt;font-weight:bold;color:rgba(0,0,0,.06);transform:rotate(-38deg);pointer-events:none}
@media print{body{background:#fff}.page{margin:0;width:auto;min-height:auto;padding:0;break-after:page}.wide{}}
@media (max-width:820px){.page{width:auto;padding:16px}.sig{grid-template-columns:1fr}}
"#;
