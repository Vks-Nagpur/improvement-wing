//! Minimal Ollama HTTP client (/api/version, /api/tags, /api/pull, /api/chat).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::BufRead;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum OllamaError {
    /// Ollama is not installed or not running.
    NotRunning(String),
    /// The model has not been downloaded yet.
    ModelMissing(String),
    /// Anything else Ollama reported.
    Failed(String),
}

impl std::fmt::Display for OllamaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OllamaError::NotRunning(u) => write!(f, "Local AI is not running at {u}. Install Ollama from ollama.com (free) and start it; LedgerCraft works without it."),
            OllamaError::ModelMissing(m) => write!(f, "The AI model '{m}' is not downloaded yet. Use 'Set up AI' to download it once (needs internet one time)."),
            OllamaError::Failed(e) => write!(f, "Local AI error: {e}"),
        }
    }
}

impl std::error::Error for OllamaError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct OllamaClient {
    pub base_url: String,
    pub model: String,
    pub timeout: Duration,
}

impl Default for OllamaClient {
    fn default() -> Self {
        OllamaClient {
            base_url: "http://127.0.0.1:11434".into(),
            model: crate::RECOMMENDED_MODELS[0].0.into(),
            timeout: Duration::from_secs(300),
        }
    }
}

fn err_text(v: &Value) -> Option<String> {
    v.get("error").and_then(|e| e.as_str()).map(String::from)
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        OllamaClient {
            base_url: base_url.trim_end_matches('/').into(),
            model: model.into(),
            ..Default::default()
        }
    }

    fn agent(&self) -> ureq::Agent {
        // Local service: never send through an internet proxy.
        ureq::AgentBuilder::new()
            .timeout(self.timeout)
            .try_proxy_from_env(false)
            .build()
    }

    fn get(&self, path: &str) -> Result<Value, OllamaError> {
        let r = self.agent().get(&format!("{}{path}", self.base_url)).call();
        match r {
            Ok(resp) => serde_json::from_reader(resp.into_reader())
                .map_err(|e| OllamaError::Failed(e.to_string())),
            Err(ureq::Error::Status(_, resp)) => {
                let v: Value = serde_json::from_reader(resp.into_reader()).unwrap_or(Value::Null);
                Err(OllamaError::Failed(
                    err_text(&v).unwrap_or_else(|| "request refused".into()),
                ))
            }
            Err(_) => Err(OllamaError::NotRunning(self.base_url.clone())),
        }
    }

    fn post(&self, path: &str, body: &Value) -> Result<Value, OllamaError> {
        let r = self
            .agent()
            .post(&format!("{}{path}", self.base_url))
            .send_json(body.clone());
        match r {
            Ok(resp) => {
                let v: Value = serde_json::from_reader(resp.into_reader())
                    .map_err(|e| OllamaError::Failed(e.to_string()))?;
                match err_text(&v) {
                    Some(e) => Err(self.classify(e)),
                    None => Ok(v),
                }
            }
            Err(ureq::Error::Status(_, resp)) => {
                let v: Value = serde_json::from_reader(resp.into_reader()).unwrap_or(Value::Null);
                Err(self.classify(err_text(&v).unwrap_or_else(|| "request refused".into())))
            }
            Err(_) => Err(OllamaError::NotRunning(self.base_url.clone())),
        }
    }

    fn classify(&self, e: String) -> OllamaError {
        if e.contains("not found") && e.contains("model") {
            OllamaError::ModelMissing(self.model.clone())
        } else {
            OllamaError::Failed(e)
        }
    }

    pub fn version(&self) -> Result<String, OllamaError> {
        let v = self.get("/api/version")?;
        Ok(v.get("version")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    pub fn models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let v = self.get("/api/tags")?;
        Ok(v.get("models")
            .and_then(|m| m.as_array())
            .into_iter()
            .flatten()
            .map(|m| ModelInfo {
                name: m.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
                size_bytes: m.get("size").and_then(|x| x.as_u64()).unwrap_or(0),
            })
            .collect())
    }

    pub fn has_model(&self) -> Result<bool, OllamaError> {
        let want = self.model.trim_end_matches(":latest");
        Ok(self
            .models()?
            .iter()
            .any(|m| m.name == self.model || m.name.trim_end_matches(":latest") == want))
    }

    /// Download the model (one time). `progress(status, completed, total)`.
    pub fn pull(&self, mut progress: impl FnMut(&str, u64, u64)) -> Result<(), OllamaError> {
        let resp = self
            .agent()
            .post(&format!("{}/api/pull", self.base_url))
            .send_json(json!({"model": self.model, "stream": true}))
            .map_err(|e| match e {
                ureq::Error::Status(_, r) => {
                    OllamaError::Failed(r.into_string().unwrap_or_default())
                }
                _ => OllamaError::NotRunning(self.base_url.clone()),
            })?;
        let mut last = String::new();
        for line in std::io::BufReader::new(resp.into_reader()).lines() {
            let line = line.map_err(|e| OllamaError::Failed(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            let v: Value =
                serde_json::from_str(&line).map_err(|e| OllamaError::Failed(e.to_string()))?;
            if let Some(e) = err_text(&v) {
                return Err(OllamaError::Failed(format!(
                    "download of '{}' failed: {e}",
                    self.model
                )));
            }
            last = v
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            progress(
                &last,
                v.get("completed").and_then(|x| x.as_u64()).unwrap_or(0),
                v.get("total").and_then(|x| x.as_u64()).unwrap_or(0),
            );
        }
        if last == "success" {
            Ok(())
        } else {
            Err(OllamaError::Failed(format!(
                "download of '{}' ended without success (last status: {last})",
                self.model
            )))
        }
    }

    /// One chat turn. `json_schema` asks Ollama for structured JSON output.
    pub fn chat(
        &self,
        system: &str,
        user: &str,
        json_schema: Option<&Value>,
    ) -> Result<String, OllamaError> {
        let mut body = json!({
            "model": self.model,
            "stream": false,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
            "options": {"temperature": 0.1}
        });
        if let Some(s) = json_schema {
            body["format"] = s.clone();
        }
        let v = self.post("/api/chat", &body)?;
        v.pointer("/message/content")
            .and_then(|c| c.as_str())
            .map(|s| s.trim().to_string())
            .ok_or_else(|| OllamaError::Failed("empty reply".into()))
    }
}
