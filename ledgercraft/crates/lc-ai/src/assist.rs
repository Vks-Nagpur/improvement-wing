//! What the AI may do inside LedgerCraft – each with strict output checks.

use crate::ollama::{OllamaClient, OllamaError};
use lc_core::checks::Finding;
use lc_core::mapping::Head;
use lc_core::ratios::Ratio;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    English,
    Hindi,
    Marathi,
}

impl Language {
    fn name(self) -> &'static str {
        match self {
            Language::English => "simple English",
            Language::Hindi => "simple Hindi (Devanagari script)",
            Language::Marathi => "simple Marathi (Devanagari script)",
        }
    }
}

/// An AI answer, labelled for display and for the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Suggestion {
    pub text: String,
    pub model: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MappingSuggestion {
    pub head: Head,
    pub reason: String,
    pub model: String,
}

const SYSTEM: &str =
    "You assist Chartered Accountants in India preparing financial statements and audits. \
Use only the facts given to you. Never invent amounts, dates, section numbers or case law. \
If something cannot be decided from the facts, say what information is needed. \
Be brief, practical and correct.";

pub struct Assistant {
    pub client: OllamaClient,
}

impl Assistant {
    pub fn new(client: OllamaClient) -> Self {
        Assistant { client }
    }

    fn label(&self, text: String) -> Suggestion {
        Suggestion {
            text,
            model: self.client.model.clone(),
            label: "AI suggestion – review before use".into(),
        }
    }

    /// Explain a finding to a non-expert in the chosen language.
    pub fn explain_finding(&self, f: &Finding, lang: Language) -> Result<Suggestion, OllamaError> {
        let facts = json!({
            "check": f.title, "what_was_found": f.message, "suggested_action": f.suggestion,
            "ledger": f.ledger, "voucher": f.voucher, "amount": f.amount.map(|m| m.fmt_indian()), "legal_reference": f.legal_ref,
        });
        let user = format!(
            "Explain this finding to a business owner in {} in at most 5 short sentences: what it means, why it matters, and what to do next. Facts: {facts}",
            lang.name()
        );
        self.client.chat(SYSTEM, &user, None).map(|t| self.label(t))
    }

    /// Suggest where an unusual ledger belongs. The answer must be one of `allowed`.
    pub fn suggest_mapping(
        &self,
        ledger: &str,
        group: &str,
        balance: &str,
        allowed: &[Head],
    ) -> Result<MappingSuggestion, OllamaError> {
        let ids: Vec<String> = allowed.iter().map(|h| h.id()).collect();
        let schema = json!({
            "type": "object",
            "properties": {"head": {"type": "string", "enum": ids}, "reason": {"type": "string"}},
            "required": ["head", "reason"]
        });
        let user = format!(
            "Ledger name: '{ledger}'. Group in the accounting software: '{group}'. Closing balance: {balance}. \
Choose the financial-statement head (Schedule III / ICAI format) where this ledger belongs, from: {}. \
Give a one-sentence reason. Reply as JSON.",
            ids.join(", ")
        );
        let raw = self.client.chat(SYSTEM, &user, Some(&schema))?;
        #[derive(Deserialize)]
        struct R {
            head: String,
            reason: String,
        }
        let r: R = serde_json::from_str(&raw)
            .map_err(|_| OllamaError::Failed(format!("AI reply was not valid JSON: {raw}")))?;
        let head = Head::from_id(&r.head)
            .filter(|h| allowed.contains(h))
            .ok_or_else(|| {
                OllamaError::Failed(format!(
                    "AI suggested '{}', which is not an allowed head – ignored",
                    r.head
                ))
            })?;
        Ok(MappingSuggestion {
            head,
            reason: r.reason,
            model: self.client.model.clone(),
        })
    }

    /// Draft the "reason for variance" text for a ratio that moved more than 25%.
    pub fn draft_ratio_reason(&self, r: &Ratio, context: &str) -> Result<Suggestion, OllamaError> {
        let user = format!(
            "Draft a one-sentence reason (formal, for notes to accounts) for the change in the {} from {} to {} ({}). \
Numerator: {}. Denominator: {}. Known facts about the year: {context}. If the facts do not explain it, write a neutral sentence and add '[confirm with management]'.",
            r.name,
            r.py.map(|v| format!("{v:.2}")).unwrap_or("N.A.".into()),
            r.cy.map(|v| format!("{v:.2}")).unwrap_or("N.A.".into()),
            r.variance_pct.map(|v| format!("{v:+.1}%")).unwrap_or_default(),
            r.numerator,
            r.denominator
        );
        self.client.chat(SYSTEM, &user, None).map(|t| self.label(t))
    }

    /// Free question about the current engagement, answered only from `context`.
    pub fn ask(
        &self,
        question: &str,
        context: &str,
        lang: Language,
    ) -> Result<Suggestion, OllamaError> {
        let user = format!("Answer in {}. Context (figures and findings of this engagement): {context}\n\nQuestion: {question}", lang.name());
        self.client.chat(SYSTEM, &user, None).map(|t| self.label(t))
    }
}
