//! service/waha.rs — klien WAHA (WhatsApp HTTP API) minimal: kirim teks.
//! Pola disalin dari `e-ticketing/src/service/auth.rs::send_wa_otp`.

use std::time::Duration;

use crate::config::config::WahaConfig;

/// Tanpa batas eksplisit, `reqwest::Client` default tak punya timeout sama
/// sekali — endpoint yang tak terjangkau (salah alamat, WAHA belum jalan)
/// bisa menggantung sampai batas TCP OS (puluhan detik hingga menit),
/// menahan seluruh request pendaftaran/resend OTP. Pelajaran yang sama
/// dengan timeout RustFS di `main.rs`.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct WahaClient {
    http: reqwest::Client,
    base_url: String,
    session: String,
    api_key: String,
}

impl WahaClient {
    pub fn new(cfg: &WahaConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            http,
            base_url: cfg.base_url.clone(),
            session: cfg.session.clone(),
            api_key: cfg.api_key.clone(),
        }
    }

    /// `chat_id` format WAHA: `"62xxxxxxxxxx@c.us"` (lihat `utils::phone`).
    pub async fn send_text(&self, chat_id: &str, text: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/sendText", self.base_url);
        let mut req = self.http.post(&url).json(&serde_json::json!({
            "chatId": chat_id,
            "text": text,
            "session": self.session,
        }));
        if !self.api_key.is_empty() {
            req = req.header("X-Api-Key", &self.api_key);
        }

        let resp = req.send().await.map_err(|e| {
            // `{e}` saja cuma "error sending request" — penyebab sebenarnya
            // (connection refused, TLS, DNS) ada di rantai `source()`.
            tracing::warn!(url = %url, error = ?e, "WAHA: gagal mengirim request");
            anyhow::anyhow!("Gagal menghubungi WAHA: {e}")
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("WAHA membalas {status}: {body}");
        }
        Ok(())
    }
}
