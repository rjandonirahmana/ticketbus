//! service/auth.rs — satu mekanisme sesi untuk SEMUA peran (buyer/merchant/
//! admin). Pendaftaran buyer/merchant lewat OTP WhatsApp (WAHA); admin
//! di-seed dari env saat startup (`ensure_admin_seed`), tak pernah lewat
//! `register_request`. Login SETELAH terdaftar pakai no. HP + password
//! (bukan OTP tiap kali) — pola persis `e-ticketing`/`ppm`.

use std::sync::Arc;

use chrono::{Duration, Utc};
use rand::Rng;
use subtle::ConstantTimeEq;

use crate::repository::{OtpRepository, PhoneChangeRepository, UserRepository};
use crate::service::rate_limit::RateLimiter;
use crate::service::wa_message;
use crate::service::waha::WahaClient;
use crate::utils::{phone, token};
use crate::web::models::PublicUser;

pub const SESSION_COOKIE: &str = "bis_session";
const SESSION_TTL_DAYS: i64 = 30;
const OTP_TTL_MINUTES: i64 = 10;
const OTP_MAX_ATTEMPTS: i32 = 5;
const REGISTER_MAX_PER_HOUR: usize = 5;
const RESEND_MAX_PER_WINDOW: usize = 1;
const RESEND_COOLDOWN_SECS: i64 = 60;
const VERIFY_MAX_PER_WINDOW: usize = 15;
const VERIFY_WINDOW_SECS: i64 = 600;
const LOGIN_MAX_PER_WINDOW: usize = 10;
const LOGIN_WINDOW_SECS: i64 = 600;
const FORGOT_MAX_PER_HOUR: usize = 3;
const PENDING_PASSWORD_TTL_HOURS: i64 = 3;
const PHONE_CHANGE_MAX_PER_HOUR: usize = 5;

#[derive(Clone, Debug)]
pub struct Claims {
    pub user_id: String,
    pub role: String,
}

#[derive(Clone)]
pub struct AuthService {
    users: UserRepository,
    otp: OtpRepository,
    phone_change: PhoneChangeRepository,
    waha: WahaClient,
    rate: Arc<RateLimiter>,
    secret: String,
}

impl AuthService {
    pub fn new(
        users: UserRepository,
        otp: OtpRepository,
        phone_change: PhoneChangeRepository,
        waha: WahaClient,
        rate: Arc<RateLimiter>,
        secret: String,
    ) -> Self {
        Self { users, otp, phone_change, waha, rate, secret }
    }

    pub fn issue_session(&self, user_id: &str, role: &str) -> String {
        token::issue(&self.secret, &format!("{user_id}|{role}"), SESSION_TTL_DAYS)
    }

    pub fn verify_session(&self, cookie_value: &str) -> Option<Claims> {
        let payload = token::verify(&self.secret, cookie_value)?;
        let (id, role) = payload.split_once('|')?;
        Some(Claims { user_id: id.to_string(), role: role.to_string() })
    }

    /// Tahap 1 pendaftaran: validasi → generate password+OTP acak → **kirim
    /// WA dulu** → baru simpan draf. Kalau kirim WA gagal, draf TIDAK
    /// tersimpan — tak ada sesi pendaftaran menggantung untuk pesan yang tak
    /// pernah sampai (pola `send_wa_otp` di e-ticketing).
    pub async fn register_request(&self, phone_raw: &str, name: &str, role: &str) -> anyhow::Result<()> {
        let phone_norm = phone::normalize(phone_raw).ok_or_else(|| anyhow::anyhow!("Nomor HP tidak valid"))?;
        let name = name.trim();
        if name.is_empty() {
            anyhow::bail!("Nama tidak boleh kosong");
        }
        if role != "buyer" && role != "merchant" {
            anyhow::bail!("Peran tidak valid");
        }
        if self.users.exists_by_phone(&phone_norm).await? {
            anyhow::bail!("Nomor HP sudah terdaftar — silakan login");
        }
        if !self.rate.allow(&format!("register:{phone_norm}"), REGISTER_MAX_PER_HOUR, 3600) {
            anyhow::bail!("Terlalu banyak percobaan daftar dari nomor ini, coba lagi nanti");
        }

        let password = generate_password();
        let otp_code = generate_otp();
        let password_hash = hash_password(&password)?;

        let chat_id = phone::to_waha_chat_id(&phone_norm);
        let text = wa_message::register(name, role, &otp_code, OTP_TTL_MINUTES, &password);
        self.waha.send_text(&chat_id, &text).await?;

        let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
        self.otp
            .upsert(&phone_norm, name, role, &password_hash, &otp_code, expires_at)
            .await?;
        Ok(())
    }

    pub async fn resend_otp(&self, phone_raw: &str) -> anyhow::Result<()> {
        let phone_norm = phone::normalize(phone_raw).ok_or_else(|| anyhow::anyhow!("Nomor HP tidak valid"))?;
        let pending = self
            .otp
            .get(&phone_norm)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Tidak ada pendaftaran tertunda untuk nomor ini"))?;
        if !self
            .rate
            .allow(&format!("resend:{phone_norm}"), RESEND_MAX_PER_WINDOW, RESEND_COOLDOWN_SECS)
        {
            anyhow::bail!("Tunggu sebentar sebelum minta kode baru");
        }

        let otp_code = generate_otp();
        let chat_id = phone::to_waha_chat_id(&phone_norm);
        let text = wa_message::resend_otp(&pending.name, &otp_code, OTP_TTL_MINUTES);
        self.waha.send_text(&chat_id, &text).await?;

        let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
        self.otp
            .upsert(&phone_norm, &pending.name, &pending.role, &pending.password_hash, &otp_code, expires_at)
            .await?;
        Ok(())
    }

    /// Tahap 2: cocokkan OTP (constant-time) → INSERT user sungguhan →
    /// hapus draf → terbitkan sesi.
    pub async fn register_verify(&self, phone_raw: &str, otp_input: &str) -> anyhow::Result<(PublicUser, String)> {
        let phone_norm = phone::normalize(phone_raw).ok_or_else(|| anyhow::anyhow!("Nomor HP tidak valid"))?;
        if !self
            .rate
            .allow(&format!("verify:{phone_norm}"), VERIFY_MAX_PER_WINDOW, VERIFY_WINDOW_SECS)
        {
            anyhow::bail!("Terlalu banyak percobaan verifikasi, coba lagi nanti");
        }

        let pending = self
            .otp
            .get(&phone_norm)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Tidak ada pendaftaran tertunda untuk nomor ini"))?;

        if pending.attempts >= OTP_MAX_ATTEMPTS {
            self.otp.delete(&phone_norm).await?;
            anyhow::bail!("Percobaan OTP habis — silakan daftar ulang");
        }
        if Utc::now() > pending.expires_at {
            self.otp.delete(&phone_norm).await?;
            anyhow::bail!("Kode OTP sudah kedaluwarsa — silakan daftar ulang");
        }
        if !constant_time_eq(&pending.otp_code, otp_input) {
            self.otp.increment_attempts(&phone_norm).await?;
            anyhow::bail!("Kode OTP salah");
        }

        let user = self
            .users
            .create(&phone_norm, &pending.name, &pending.role, &pending.password_hash)
            .await?;
        self.otp.delete(&phone_norm).await?;
        let session = self.issue_session(&user.id, &user.role);
        Ok((user, session))
    }

    /// Login no. HP + password. Nomor tak terdaftar tetap menjalankan SATU
    /// verifikasi bcrypt terhadap hash tetap (`dummy_hash`) supaya waktu
    /// respons "nomor salah" tak terbedakan dari "password salah" —
    /// mencegah pencacahan nomor terdaftar lewat timing.
    pub async fn login(&self, phone_raw: &str, password: &str) -> anyhow::Result<(PublicUser, String)> {
        let phone_norm = phone::normalize(phone_raw).ok_or_else(|| anyhow::anyhow!("Nomor HP tidak valid"))?;
        if !self
            .rate
            .allow(&format!("login:{phone_norm}"), LOGIN_MAX_PER_WINDOW, LOGIN_WINDOW_SECS)
        {
            anyhow::bail!("Terlalu banyak percobaan login, coba lagi nanti");
        }

        let found = self.users.find_by_phone_with_hash(&phone_norm).await?;
        let Some(u) = found else {
            verify_password(password, dummy_hash());
            anyhow::bail!("Nomor HP atau password salah");
        };

        if verify_password(password, &u.password_hash) {
            // Pemilik masih ingat password lamanya → permintaan "lupa
            // password" yang tertunda (mungkin dibuat orang lain) dibatalkan.
            if u.pending_password_hash.is_some() {
                self.users.clear_pending_password(&u.id).await?;
            }
        } else {
            let pending_ok = match (&u.pending_password_hash, u.pending_password_expires_at) {
                (Some(h), Some(exp)) if Utc::now() < exp => verify_password(password, h),
                _ => false,
            };
            if !pending_ok {
                anyhow::bail!("Nomor HP atau password salah");
            }
            self.users.promote_pending_password(&u.id).await?;
        }

        let session = self.issue_session(&u.id, &u.role);
        Ok((PublicUser { id: u.id, name: u.name, phone: u.phone, role: u.role }, session))
    }

    pub async fn user_by_id(&self, id: &str) -> anyhow::Result<Option<PublicUser>> {
        self.users.find_by_id(id).await
    }

    /// Kirim password BARU via WA. Password lama TETAP berlaku; yang baru
    /// disimpan tertunda dan baru jadi resmi saat dipakai login (lihat
    /// `login`). Nomor tak terdaftar dibalas sukses yang sama persis —
    /// form ini tak boleh jadi alat mengecek nomor mana yang punya akun.
    pub async fn forgot_password(&self, phone_raw: &str) -> anyhow::Result<()> {
        let phone_norm = phone::normalize(phone_raw).ok_or_else(|| anyhow::anyhow!("Nomor HP tidak valid"))?;
        if !self.rate.allow(&format!("forgot:{phone_norm}"), FORGOT_MAX_PER_HOUR, 3600) {
            anyhow::bail!("Terlalu banyak permintaan, coba lagi nanti");
        }
        let Some(u) = self.users.find_by_phone_with_hash(&phone_norm).await? else {
            return Ok(());
        };

        let password = generate_password();
        let hash = hash_password(&password)?;
        let chat_id = phone::to_waha_chat_id(&phone_norm);
        let text = wa_message::forgot_password(&u.name, &password, PENDING_PASSWORD_TTL_HOURS);
        self.waha.send_text(&chat_id, &text).await?;

        let expires_at = Utc::now() + Duration::hours(PENDING_PASSWORD_TTL_HOURS);
        self.users.set_pending_password(&u.id, &hash, expires_at).await?;
        Ok(())
    }

    /// Tahap 1 ganti nomor: OTP dikirim ke nomor BARU — membuktikan nomor
    /// itu benar-benar milik si pengguna sebelum jadi identitas login-nya.
    pub async fn request_phone_change(&self, user_id: &str, new_phone_raw: &str) -> anyhow::Result<()> {
        let new_phone = phone::normalize(new_phone_raw).ok_or_else(|| anyhow::anyhow!("Nomor HP tidak valid"))?;
        if self.users.exists_by_phone(&new_phone).await? {
            anyhow::bail!("Nomor HP sudah dipakai akun lain");
        }
        if !self.rate.allow(&format!("phonechg:{user_id}"), PHONE_CHANGE_MAX_PER_HOUR, 3600) {
            anyhow::bail!("Terlalu banyak permintaan, coba lagi nanti");
        }

        let otp_code = generate_otp();
        let chat_id = phone::to_waha_chat_id(&new_phone);
        let text = wa_message::phone_change(&otp_code, OTP_TTL_MINUTES);
        self.waha.send_text(&chat_id, &text).await?;

        let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
        self.phone_change.upsert(user_id, &new_phone, &otp_code, expires_at).await?;
        Ok(())
    }

    /// Tahap 2 ganti nomor. Sesi tetap sah (token memuat user_id, bukan nomor).
    pub async fn verify_phone_change(&self, user_id: &str, otp_input: &str) -> anyhow::Result<PublicUser> {
        if !self
            .rate
            .allow(&format!("phonechg-verify:{user_id}"), VERIFY_MAX_PER_WINDOW, VERIFY_WINDOW_SECS)
        {
            anyhow::bail!("Terlalu banyak percobaan verifikasi, coba lagi nanti");
        }
        let pending = self
            .phone_change
            .get(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Tidak ada permintaan ganti nomor"))?;

        if pending.attempts >= OTP_MAX_ATTEMPTS {
            self.phone_change.delete(user_id).await?;
            anyhow::bail!("Percobaan OTP habis — minta kode baru");
        }
        if Utc::now() > pending.expires_at {
            self.phone_change.delete(user_id).await?;
            anyhow::bail!("Kode OTP sudah kedaluwarsa — minta kode baru");
        }
        if !constant_time_eq(&pending.otp_code, otp_input) {
            self.phone_change.increment_attempts(user_id).await?;
            anyhow::bail!("Kode OTP salah");
        }

        let user = self.users.update_phone(user_id, &pending.new_phone).await?;
        self.phone_change.delete(user_id).await?;
        Ok(user)
    }
}

/// Seed SATU akun admin dari env, idempoten — aman dipanggil tiap startup.
/// Admin TAK PERNAH lewat `register_request` (route itu hanya menerima
/// role buyer/merchant, lihat CHECK constraint `otp_pending.role`).
pub async fn ensure_admin_seed(users: &UserRepository, phone_raw: &str, name: &str, password: &str) -> anyhow::Result<()> {
    if users.admin_exists().await? {
        return Ok(());
    }
    let phone_norm = phone::normalize(phone_raw).unwrap_or_else(|| phone_raw.to_string());
    let hash = hash_password(password)?;
    users.create_admin(&phone_norm, name, &hash).await?;
    tracing::info!(phone = %phone_norm, "seed: akun admin dibuat");
    Ok(())
}

fn generate_otp() -> String {
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(0..1_000_000u32))
}

fn generate_password() -> String {
    const CHARS: &[u8] = b"abcdefghjkmnpqrstuvwxyzABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..8).map(|_| CHARS[rng.random_range(0..CHARS.len())] as char).collect()
}

fn hash_password(plain: &str) -> anyhow::Result<String> {
    bcrypt::hash(plain, bcrypt::DEFAULT_COST).map_err(|e| anyhow::anyhow!("gagal hash password: {e}"))
}

fn verify_password(plain: &str, hash: &str) -> bool {
    bcrypt::verify(plain, hash).unwrap_or(false)
}

fn dummy_hash() -> &'static str {
    use std::sync::OnceLock;
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| bcrypt::hash("dummy-password-untuk-timing", 10).unwrap_or_default())
        .as_str()
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_bisa_diverifikasi() {
        let h = hash_password("rahasia123").unwrap();
        assert!(verify_password("rahasia123", &h));
        assert!(!verify_password("salah", &h));
    }

    #[test]
    fn otp_enam_digit() {
        for _ in 0..20 {
            let o = generate_otp();
            assert_eq!(o.len(), 6);
            assert!(o.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn constant_time_eq_benar() {
        assert!(constant_time_eq("123456", "123456"));
        assert!(!constant_time_eq("123456", "654321"));
        assert!(!constant_time_eq("123", "123456"));
    }
}
