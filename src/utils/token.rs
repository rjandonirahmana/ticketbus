//! utils/token.rs — token sesi HMAC generik: `{exp}.{payload}.{sig}`,
//! base64url. Dipakai satu-satunya mekanisme sesi (buyer/merchant/admin
//! semua lewat sini) — sebelumnya `service/auth.rs` (admin-only) punya
//! salinan logika ini sendiri; sekarang diekstrak supaya cuma ada satu
//! tempat yang tahu cara menanda-tangani & memverifikasi token.

use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Terbitkan token baru untuk `payload` (string bebas, JANGAN mengandung
/// karakter `.` — dipakai sebagai pemisah), berlaku `ttl_days` hari.
pub fn issue(secret: &str, payload: &str, ttl_days: i64) -> String {
    let exp = (Utc::now() + chrono::Duration::days(ttl_days)).timestamp();
    let body = format!("{exp}.{payload}");
    let sig = sign(secret, &body);
    format!("{body}.{sig}")
}

/// Verifikasi token, kembalikan `payload` bila tanda tangan cocok DAN belum
/// kedaluwarsa. `None` untuk token rusak, tanda tangan salah, atau basi.
pub fn verify(secret: &str, token: &str) -> Option<String> {
    let mut parts = token.splitn(3, '.');
    let exp_str = parts.next()?;
    let payload = parts.next()?;
    let sig = parts.next()?;

    let body = format!("{exp_str}.{payload}");
    let expected = sign(secret, &body);
    if !bool::from(expected.as_bytes().ct_eq(sig.as_bytes())) {
        return None;
    }

    let exp: i64 = exp_str.parse().ok()?;
    if Utc::now().timestamp() >= exp {
        return None;
    }

    Some(payload.to_string())
}

fn sign(secret: &str, body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC menerima kunci panjang berapa pun");
    mac.update(body.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terbit_dan_terverifikasi() {
        let t = issue("secret", "user-123|buyer", 7);
        assert_eq!(verify("secret", &t).as_deref(), Some("user-123|buyer"));
    }

    #[test]
    fn secret_beda_ditolak() {
        let t = issue("secret-a", "payload", 7);
        assert_eq!(verify("secret-b", &t), None);
    }

    #[test]
    fn sudah_kedaluwarsa_ditolak() {
        let t = issue("secret", "payload", -1);
        assert_eq!(verify("secret", &t), None);
    }

    #[test]
    fn rusak_ditolak() {
        assert_eq!(verify("secret", "bukan-token-yang-benar"), None);
        assert_eq!(verify("secret", ""), None);
    }
}
