//! Local AI through Ollama (<https://ollama.com>, runs on the user's own PC).
//!
//! Rules (enforced here, not just promised):
//! * LedgerCraft works fully without AI; every call can fail gracefully.
//! * AI only suggests or explains. It never changes a figure: a suggested
//!   mapping must name one of the allowed heads or it is rejected.
//! * Every answer is labelled as an AI suggestion and carries the model name,
//!   so the app can record it in the audit trail.

pub mod assist;
pub mod ollama;

pub use assist::{Assistant, Language, MappingSuggestion, Suggestion};
pub use ollama::{ModelInfo, OllamaClient, OllamaError};

/// Small multilingual models that run on an ordinary office PC (CPU only).
pub const RECOMMENDED_MODELS: &[(&str, &str)] = &[
    (
        "qwen2.5:3b",
        "Recommended – good English, Hindi and Marathi; about 2 GB download, 8 GB RAM PC",
    ),
    (
        "llama3.2:3b",
        "Alternative – good English; about 2 GB download",
    ),
    (
        "qwen2.5:1.5b",
        "For older PCs – faster, less accurate; about 1 GB download",
    ),
];
