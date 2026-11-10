use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSyncConfig {
    pub enabled: bool,
    pub url: Option<String>,
}

impl Default for VaultSyncConfig {
    fn default() -> Self {
        let enabled = std::env::var("XAVIER_VAULT_SYNC_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let url = std::env::var("XAVIER_VAULT_SYNC_URL").ok();

        Self { enabled, url }
    }
}
