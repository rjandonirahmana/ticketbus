//! service/rate_limit.rs — pembatas laju in-memory, per-proses.
//!
//! `e-ticketing` memakai Redis untuk ini karena Redis-nya sudah ada untuk hal
//! lain (WS, keranjang). `bis` tak punya Redis sama sekali — menambahkannya
//! cuma untuk rate-limit OTP tak sepadan di skala aplikasi ini. Konsekuensi:
//! penghitung reset saat proses restart/deploy, dan tak konsisten lintas
//! instance kalau suatu saat di-scale horizontal. Keduanya dapat diterima
//! untuk satu proses server tunggal.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct RateLimiter {
    hits: Mutex<HashMap<String, Vec<i64>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` bila permintaan diizinkan (dan langsung dicatat). `false` bila
    /// `key` sudah mencapai `max` permintaan dalam `window_secs` terakhir.
    pub fn allow(&self, key: &str, max: usize, window_secs: i64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut hits = self.hits.lock().expect("rate limiter mutex poisoned");
        let entry = hits.entry(key.to_string()).or_default();
        entry.retain(|&t| now - t < window_secs);

        if entry.len() >= max {
            return false;
        }
        entry.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mengizinkan_sampai_batas_lalu_menolak() {
        let rl = RateLimiter::new();
        assert!(rl.allow("k", 2, 3600));
        assert!(rl.allow("k", 2, 3600));
        assert!(!rl.allow("k", 2, 3600));
    }

    #[test]
    fn key_berbeda_tak_saling_pengaruh() {
        let rl = RateLimiter::new();
        assert!(rl.allow("a", 1, 3600));
        assert!(rl.allow("b", 1, 3600));
    }
}
