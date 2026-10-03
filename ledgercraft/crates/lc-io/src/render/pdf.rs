//! PDF output through an embedded Typst engine (fully offline).
//! Fonts: Liberation Serif (SIL OFL) with DejaVu Serif fallback for "₹".

use lc_core::report::Report;
use std::sync::OnceLock;
use typst::foundations::{Dict, IntoValue};
use typst_as_lib::TypstEngine;

const TEMPLATE: &str = include_str!("../../templates/report.typ");
const FONTS: [&[u8]; 6] = [
    include_bytes!("../../fonts/LiberationSerif-Regular.ttf"),
    include_bytes!("../../fonts/LiberationSerif-Bold.ttf"),
    include_bytes!("../../fonts/LiberationSerif-Italic.ttf"),
    include_bytes!("../../fonts/LiberationSerif-BoldItalic.ttf"),
    include_bytes!("../../fonts/DejaVuSerif.ttf"),
    include_bytes!("../../fonts/DejaVuSerif-Bold.ttf"),
];

fn engine() -> &'static TypstEngine<typst_as_lib::TypstTemplateMainFile> {
    static ENGINE: OnceLock<TypstEngine<typst_as_lib::TypstTemplateMainFile>> = OnceLock::new();
    ENGINE.get_or_init(|| {
        TypstEngine::builder()
            .main_file(TEMPLATE)
            .fonts(FONTS)
            .build()
    })
}

/// Render the report to PDF bytes.
pub fn render(report: &Report) -> Result<Vec<u8>, String> {
    let json = serde_json::to_string(report).map_err(|e| e.to_string())?;
    let mut input = Dict::new();
    input.insert("data".into(), json.into_value());
    let compiled = engine().compile_with_input(input);
    let doc = compiled
        .output
        .map_err(|e| format!("PDF layout failed: {e:?}"))?;
    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|e| format!("PDF export failed: {e:?}"))
}
