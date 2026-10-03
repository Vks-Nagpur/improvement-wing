//! Zoho Books connection (India data centre by default).
//!
//! Balances come from Zoho's own Trial Balance export (Reports → Trial
//! Balance → Export as XLSX), read by `read::read_trial_balance_with`. The API
//! is used for what it documents reliably: the chart of accounts with each
//! account's type, so every account lands in the right group automatically.
//! OAuth: a self-client refresh token from api-console.zoho.in (read-only scope
//! `ZohoBooks.accountants.READ` / `ZohoBooks.settings.READ`).

use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ZohoClient {
    pub accounts_url: String,
    pub api_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    pub organization_id: String,
}

impl ZohoClient {
    pub fn india(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        organization_id: &str,
    ) -> ZohoClient {
        ZohoClient {
            accounts_url: "https://accounts.zoho.in".into(),
            api_url: "https://www.zohoapis.in/books/v3".into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            refresh_token: refresh_token.into(),
            organization_id: organization_id.into(),
        }
    }

    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(60))
            .build()
    }

    pub fn access_token(&self) -> Result<String, String> {
        let url = format!("{}/oauth/v2/token", self.accounts_url);
        let resp = Self::agent()
            .post(&url)
            .send_form(&[
                ("refresh_token", &self.refresh_token),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("grant_type", "refresh_token"),
            ])
            .map_err(|e| format!("Zoho sign-in failed: {e}"))?;
        let v: Value = serde_json::from_reader(resp.into_reader())
            .map_err(|e| format!("Zoho sign-in reply: {e}"))?;
        v.get("access_token")
            .and_then(|t| t.as_str())
            .map(String::from)
            .ok_or_else(|| {
                format!(
                    "Zoho sign-in refused: {}",
                    v.get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("no access token returned")
                )
            })
    }

    /// Account name → account type (e.g. "accounts_receivable").
    pub fn chart_of_accounts(&self) -> Result<HashMap<String, String>, String> {
        let token = self.access_token()?;
        let mut out = HashMap::new();
        let mut page = 1;
        loop {
            let url = format!(
                "{}/chartofaccounts?organization_id={}&page={page}&per_page=200",
                self.api_url, self.organization_id
            );
            let resp = Self::agent()
                .get(&url)
                .set("Authorization", &format!("Zoho-oauthtoken {token}"))
                .call()
                .map_err(|e| format!("Zoho chart of accounts: {e}"))?;
            let v: Value = serde_json::from_reader(resp.into_reader())
                .map_err(|e| format!("Zoho chart of accounts reply: {e}"))?;
            if v.get("code").and_then(|c| c.as_i64()).unwrap_or(0) != 0 {
                return Err(format!(
                    "Zoho: {}",
                    v.get("message").and_then(|m| m.as_str()).unwrap_or("error")
                ));
            }
            for a in v
                .get("chartofaccounts")
                .and_then(|x| x.as_array())
                .into_iter()
                .flatten()
            {
                if let (Some(n), Some(t)) = (
                    a.get("account_name").and_then(|x| x.as_str()),
                    a.get("account_type").and_then(|x| x.as_str()),
                ) {
                    out.insert(n.to_string(), t.to_string());
                }
            }
            let more = v
                .pointer("/page_context/has_more_page")
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            if !more || page > 500 {
                break;
            }
            page += 1;
        }
        Ok(out)
    }
}

/// Apply Zoho account types to a trial balance read from Zoho's export.
pub fn apply_account_types(tb: &mut lc_core::model::TrialBalance, types: &HashMap<String, String>) {
    for l in tb.ledgers.iter_mut() {
        if let Some(g) = types
            .get(&l.name)
            .and_then(|t| crate::read::zoho_type_group(t))
        {
            l.group = g.to_string();
        }
    }
}
