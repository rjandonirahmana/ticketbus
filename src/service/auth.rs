//! service/auth.rs — satu mekanisme sesi untuk SEMUA peran (buyer/merchant/
//! admin). Pendaftaran buyer/merchant lewat OTP WhatsApp (WAHA); admin
//! di-seed dari env saat startup (`ensure_admin_seed`), tak pernah lewat
//! `register_request`. Login SETELAH terdaftar pakai no. HP + password
//! (bukan OTP tiap kali) — pola persis `e-ticketing`/`ppm`.

use std::sync::Arc;

use chrono::{Duration, Utc};
use rand::Rng;
use subtle::ConstantTimeEq;

use crate::repository::security::SecurityRepository;
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
    /// Baris `user_sessions` milik cookie ini. `None` = cookie lama (sebelum
    /// migrasi 006) yang belum punya sesi tercatat.
    pub session_id: Option<String>,
}

/// Asal request — dicatat di sesi & riwayat keamanan.
#[derive(Clone, Debug, Default)]
pub struct SessionMeta {
    pub user_agent: String,
    pub ip: String,
}

/// Jarak minimum antar-pembaruan `last_seen_at` satu sesi.
const TOUCH_EVERY_SECS: u64 = 300;

#[derive(Clone)]
pub struct AuthService {
    users: UserRepository,
    otp: OtpRepository,
    phone_change: PhoneChangeRepository,
    security: SecurityRepository,
    waha: WahaClient,
    rate: Arc<RateLimiter>,
    secret: String,
    /// Id sesi yang dicabut (per perangkat / keluar semua / bekukan). Dimuat
    /// saat start dari `user_sessions.revoked_at`.
    revoked_sids: Arc<std::sync::RwLock<std::collections::HashSet<String>>>,
    /// Kapan `last_seen_at` tiap sesi terakhir ditulis — throttle DB.
    touched: Arc<std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>>,
    /// user_id → detik UNIX: sesi yang terbit SEBELUM ini ditolak ("keluarkan
    /// dari semua perangkat lain"). Disalin dari `users.sessions_valid_after`
    /// saat start (`load_session_revocations`) supaya `verify_session` tetap
    /// sinkron & tanpa query DB per request. Aman karena app ini satu proses
    /// (rate limiter pun sudah in-memory).
    revoked: Arc<std::sync::RwLock<std::collections::HashMap<String, i64>>>,
}

impl AuthService {
    pub fn new(
        users: UserRepository,
        otp: OtpRepository,
        phone_change: PhoneChangeRepository,
        security: SecurityRepository,
        waha: WahaClient,
        rate: Arc<RateLimiter>,
        secret: String,
    ) -> Self {
        Self {
            users,
            otp,
            phone_change,
            security,
            waha,
            rate,
            secret,
            revoked: Default::default(),
            revoked_sids: Default::default(),
            touched: Default::default(),
        }
    }

    /// Dipanggil sekali saat start (main.rs), sesudah migrasi.
    pub async fn load_session_revocations(&self) -> anyhow::Result<()> {
        let list = self.users.session_revocations().await?;
        let n = list.len();
        if let Ok(mut map) = self.revoked.write() {
            map.extend(list);
        }
        let sids = self.security.revoked_recent().await?;
        let m = sids.len();
        if let Ok(mut set) = self.revoked_sids.write() {
            set.extend(sids);
        }
        tracing::info!(pengguna = n, sesi = m, "pencabutan sesi dimuat");
        Ok(())
    }

    /// Payload `user_id|role|iat|sid` — `iat` (detik UNIX) untuk pencabutan
    /// massal, `sid` untuk pencabutan per perangkat.
    fn token_for(&self, user_id: &str, role: &str, sid: &str) -> String {
        let iat = Utc::now().timestamp();
        token::issue(&self.secret, &format!("{user_id}|{role}|{iat}|{sid}"), SESSION_TTL_DAYS)
    }

    /// Login baru: catat sesi (perangkat) lalu terbitkan cookie-nya.
    async fn start_session(&self, user_id: &str, role: &str, meta: &SessionMeta) -> anyhow::Result<String> {
        let sid = self.security.create_session(user_id, &meta.user_agent, &meta.ip).await?;
        Ok(self.token_for(user_id, role, &sid))
    }

    pub fn verify_session(&self, cookie_value: &str) -> Option<Claims> {
        let payload = token::verify(&self.secret, cookie_value)?;
        let mut parts = payload.splitn(4, '|');
        let id = parts.next()?;
        let role = parts.next()?;
        // Token lama (sebelum 26 Sep 2026) tak punya iat → dianggap terbit di
        // awal zaman: tetap sah, KECUALI pemiliknya mencabut semua sesinya.
        let iat: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let sid = parts.next().filter(|s| !s.is_empty()).map(str::to_string);
        if let Ok(map) = self.revoked.read() {
            if map.get(id).is_some_and(|after| iat < *after) {
                return None;
            }
        }
        if let Some(sid) = &sid {
            if self.revoked_sids.read().is_ok_and(|set| set.contains(sid)) {
                return None;
            }
        }
        Some(Claims { user_id: id.to_string(), role: role.to_string(), session_id: sid })
    }

    fn mark_revoked(&self, sids: impl IntoIterator<Item = String>) {
        if let Ok(mut set) = self.revoked_sids.write() {
            set.extend(sids);
        }
    }

    async fn event(&self, user_id: &str, jenis: &str, meta: &SessionMeta) {
        if let Err(e) = self.security.record(user_id, jenis, &meta.user_agent, &meta.ip).await {
            tracing::warn!(error = %e, jenis, "gagal mencatat riwayat keamanan");
        }
    }

    /// Perbarui "terakhir aktif" sesi ini — paling sering tiap 5 menit.
    pub fn touch(&self, claims: &Claims) {
        let Some(sid) = claims.session_id.clone() else { return };
        let now = std::time::Instant::now();
        let due = match self.touched.lock() {
            Ok(mut map) => {
                let due = map.get(&sid).is_none_or(|t| now.duration_since(*t).as_secs() >= TOUCH_EVERY_SECS);
                if due {
                    map.insert(sid.clone(), now);
                }
                due
            }
            Err(_) => false,
        };
        if due {
            let repo = self.security.clone();
            tokio::spawn(async move {
                let _ = repo.touch_session(&sid).await;
            });
        }
    }

    /// Cabut semua sesi LAIN (per sid + batas waktu untuk cookie lama tanpa
    /// sid), lalu kembalikan cookie baru untuk perangkat ini.
    async fn revoke_others_and_reissue(&self, claims: &Claims, meta: &SessionMeta) -> anyhow::Result<String> {
        let now = Utc::now();
        let dicabut = self.security.revoke_sessions(&claims.user_id, claims.session_id.as_deref()).await?;
        self.mark_revoked(dicabut);
        self.users.set_sessions_valid_after(&claims.user_id, now).await?;
        if let Ok(mut map) = self.revoked.write() {
            map.insert(claims.user_id.clone(), now.timestamp());
        }
        match &claims.session_id {
            Some(sid) => Ok(self.token_for(&claims.user_id, &claims.role, sid)),
            None => self.start_session(&claims.user_id, &claims.role, meta).await,
        }
    }

    /// Tombol "Keluar dari Semua Perangkat Lain" di Pusat Keamanan.
    pub async fn logout_other_devices(&self, claims: &Claims, meta: &SessionMeta) -> anyhow::Result<String> {
        let token = self.revoke_others_and_reissue(claims, meta).await?;
        self.event(&claims.user_id, "keluar_semua", meta).await;
        Ok(token)
    }

    /// Keluarkan satu perangkat. Sesi yang sedang dipakai tak boleh dicabut
    /// dari sini — untuk itu ada tombol Keluar biasa.
    pub async fn revoke_device(&self, claims: &Claims, session_id: &str, meta: &SessionMeta) -> anyhow::Result<()> {
        if claims.session_id.as_deref() == Some(session_id) {
            anyhow::bail!("Ini perangkat yang sedang Anda pakai — gunakan tombol Keluar dari Akun");
        }
        if !self.security.revoke_session(&claims.user_id, session_id).await? {
            anyhow::bail!("Sesi tidak ditemukan atau sudah berakhir");
        }
        self.mark_revoked([session_id.to_string()]);
        self.event(&claims.user_id, "sesi_dicabut", meta).await;
        Ok(())
    }

    /// Logout biasa: cabut sesi perangkat ini (cookie curian tak lagi berguna).
    pub async fn logout(&self, claims: &Claims) {
        if let Some(sid) = &claims.session_id {
            if self.security.revoke_session(&claims.user_id, sid).await.unwrap_or(false) {
                self.mark_revoked([sid.clone()]);
            }
        }
    }

    /// Bekukan akun: SEMUA sesi (termasuk perangkat ini) dicabut dan login
    /// dengan password biasa ditolak sampai pemilik memakai Lupa Password.
    pub async fn freeze_account(&self, claims: &Claims, meta: &SessionMeta) -> anyhow::Result<()> {
        let now = Utc::now();
        self.security.set_frozen(&claims.user_id, true).await?;
        let dicabut = self.security.revoke_sessions(&claims.user_id, None).await?;
        self.mark_revoked(dicabut);
        self.users.set_sessions_valid_after(&claims.user_id, now).await?;
        if let Ok(mut map) = self.revoked.write() {
            map.insert(claims.user_id.clone(), now.timestamp() + 1);
        }
        self.event(&claims.user_id, "akun_dibekukan", meta).await;
        if let Some(u) = self.users.find_by_id(&claims.user_id).await? {
            let waha = self.waha.clone();
            let text = wa_message::account_frozen(&u.name);
            let chat = phone::to_waha_chat_id(&u.phone);
            tokio::spawn(async move {
                if let Err(e) = waha.send_text(&chat, &text).await {
                    tracing::warn!(error = %e, "WA notifikasi bekukan akun gagal");
                }
            });
        }
        Ok(())
    }

    /// Data Pusat Keamanan: skor dari fakta asli + sesi + riwayat.
    pub async fn security_overview(&self, claims: &Claims) -> anyhow::Result<crate::web::models::SecurityOverview> {
        use crate::web::models::{SecurityEvent, SecurityFactor, SecurityOverview, SessionInfo};
        use crate::web::seats::{is_mobile, nama_perangkat, samarkan_ip};

        let user = self
            .users
            .find_by_id(&claims.user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Akun tidak ditemukan"))?;
        let (_, changed) = self
            .users
            .password_state(&claims.user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Akun tidak ditemukan"))?;
        let gagal = self.security.count_since(&claims.user_id, "login_gagal", 30).await?;
        let sesi_rows = self.security.active_sessions(&claims.user_id).await?;
        let events = self.security.recent_events(&claims.user_id, 8).await?;

        let umur_sandi = (Utc::now() - changed).num_days();
        let faktor = vec![
            SecurityFactor { label: "Nomor WhatsApp terverifikasi".into(), poin: 30, maks: 30 },
            SecurityFactor {
                label: "Kata sandi diperbarui ≤ 90 hari".into(),
                poin: match umur_sandi {
                    ..=90 => 30,
                    91..=180 => 15,
                    _ => 0,
                },
                maks: 30,
            },
            SecurityFactor {
                label: "Tanpa login gagal 30 hari terakhir".into(),
                poin: match gagal {
                    0 => 20,
                    1..=3 => 10,
                    _ => 0,
                },
                maks: 20,
            },
            SecurityFactor {
                label: "Maksimal 3 perangkat aktif".into(),
                poin: if sesi_rows.len() <= 3 { 20 } else { 5 },
                maks: 20,
            },
        ];
        let skor = faktor.iter().map(|f| f.poin).sum();

        let sesi = sesi_rows
            .into_iter()
            .map(|r| SessionInfo {
                saat_ini: claims.session_id.as_deref() == Some(r.id.as_str()),
                perangkat: nama_perangkat(&r.user_agent),
                mobile: is_mobile(&r.user_agent),
                ip: samarkan_ip(&r.ip),
                created_at: r.created_at.to_rfc3339(),
                last_seen_at: r.last_seen_at.to_rfc3339(),
                id: r.id,
            })
            .collect();
        let riwayat = events
            .into_iter()
            .map(|e| SecurityEvent {
                perangkat: nama_perangkat(&e.user_agent),
                ip: samarkan_ip(&e.ip),
                created_at: e.created_at.to_rfc3339(),
                jenis: e.jenis,
            })
            .collect();

        Ok(SecurityOverview {
            skor,
            faktor,
            phone: user.phone,
            password_changed_at: changed.to_rfc3339(),
            login_gagal_30h: gagal,
            sesi,
            riwayat,
        })
    }

    /// Kapan password terakhir diubah (RFC 3339) — untuk "Terakhir diubah".
    pub async fn password_changed_at(&self, user_id: &str) -> anyhow::Result<String> {
        let (_, changed) = self
            .users
            .password_state(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Akun tidak ditemukan"))?;
        Ok(changed.to_rfc3339())
    }

    /// Ganti password dari halaman Akun. Mengembalikan cookie sesi BARU —
    /// bila `logout_others`, semua sesi lain dicabut, tapi perangkat ini tetap
    /// masuk karena sesinya diterbitkan ulang sesudah titik pencabutan.
    pub async fn change_password(
        &self,
        claims: &Claims,
        current: &str,
        new: &str,
        logout_others: bool,
        meta: &SessionMeta,
    ) -> anyhow::Result<String> {
        if !self
            .rate
            .allow(&format!("chgpw:{}", claims.user_id), LOGIN_MAX_PER_WINDOW, LOGIN_WINDOW_SECS)
        {
            anyhow::bail!("Terlalu banyak percobaan, coba lagi nanti");
        }
        let (hash, _) = self
            .users
            .password_state(&claims.user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Akun tidak ditemukan"))?;
        if !verify_password(current, &hash) {
            anyhow::bail!("Kata sandi saat ini salah");
        }
        validate_new_password(new)?;
        if current == new {
            anyhow::bail!("Kata sandi baru harus berbeda dari yang sekarang");
        }

        let new_hash = hash_password(new)?;
        let now = Utc::now();
        self.users.update_password(&claims.user_id, &new_hash, now, false).await?;
        self.event(&claims.user_id, "sandi_diubah", meta).await;
        let token = if logout_others {
            let t = self.revoke_others_and_reissue(claims, meta).await?;
            self.event(&claims.user_id, "keluar_semua", meta).await;
            t
        } else {
            match &claims.session_id {
                Some(sid) => self.token_for(&claims.user_id, &claims.role, sid),
                None => self.start_session(&claims.user_id, &claims.role, meta).await?,
            }
        };

        // Pemberitahuan keamanan ke WhatsApp pemilik — best-effort: password
        // sudah berganti, WAHA yang mati tak boleh membuatnya tampak gagal.
        if let Some(u) = self.users.find_by_id(&claims.user_id).await? {
            let waha = self.waha.clone();
            let text = wa_message::password_changed(&u.name, logout_others);
            let chat = phone::to_waha_chat_id(&u.phone);
            tokio::spawn(async move {
                if let Err(e) = waha.send_text(&chat, &text).await {
                    tracing::warn!(error = %e, "WA notifikasi ganti password gagal");
                }
            });
        }

        Ok(token)
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
    pub async fn register_verify(
        &self,
        phone_raw: &str,
        otp_input: &str,
        meta: &SessionMeta,
    ) -> anyhow::Result<(PublicUser, String)> {
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
        let session = self.start_session(&user.id, &user.role, meta).await?;
        self.event(&user.id, "login_berhasil", meta).await;
        Ok((user, session))
    }

    /// Login no. HP + password. Nomor tak terdaftar tetap menjalankan SATU
    /// verifikasi bcrypt terhadap hash tetap (`dummy_hash`) supaya waktu
    /// respons "nomor salah" tak terbedakan dari "password salah" —
    /// mencegah pencacahan nomor terdaftar lewat timing.
    pub async fn login(
        &self,
        phone_raw: &str,
        password: &str,
        meta: &SessionMeta,
    ) -> anyhow::Result<(PublicUser, String)> {
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

        let beku = self.security.is_frozen(&u.id).await?;
        if verify_password(password, &u.password_hash) {
            // Akun dibekukan: password lama mungkin sudah bocor — hanya
            // password baru dari Lupa Password (dikirim ke WA pemilik) yang
            // boleh membukanya.
            if beku {
                anyhow::bail!("Akun sedang dibekukan. Gunakan \"Lupa password?\" untuk membukanya kembali lewat WhatsApp.");
            }
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
                self.event(&u.id, "login_gagal", meta).await;
                anyhow::bail!("Nomor HP atau password salah");
            }
            self.users.promote_pending_password(&u.id).await?;
            if beku {
                self.security.set_frozen(&u.id, false).await?;
                self.event(&u.id, "akun_dipulihkan", meta).await;
            }
        }

        let session = self.start_session(&u.id, &u.role, meta).await?;
        self.event(&u.id, "login_berhasil", meta).await;
        Ok((PublicUser { id: u.id, name: u.name, phone: u.phone, role: u.role }, session))
    }

    pub async fn user_by_id(&self, id: &str) -> anyhow::Result<Option<PublicUser>> {
        self.users.find_by_id(id).await
    }

    /// Kirim password BARU via WA. Password lama TETAP berlaku; yang baru
    /// disimpan tertunda dan baru jadi resmi saat dipakai login (lihat
    /// `login`). Nomor tak terdaftar dibalas sukses yang sama persis —
    /// form ini tak boleh jadi alat mengecek nomor mana yang punya akun.
    pub async fn forgot_password(&self, phone_raw: &str, meta: &SessionMeta) -> anyhow::Result<()> {
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
        self.event(&u.id, "lupa_sandi", meta).await;
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
    pub async fn verify_phone_change(
        &self,
        user_id: &str,
        otp_input: &str,
        meta: &SessionMeta,
    ) -> anyhow::Result<PublicUser> {
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
        self.event(user_id, "nomor_diubah", meta).await;
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

/// Aturan kata sandi baru — SAMA dengan daftar syarat di halaman ganti sandi
/// (`web/pages/change_password.rs`); simbol dianjurkan tapi tidak wajib.
pub fn validate_new_password(p: &str) -> anyhow::Result<()> {
    if p.chars().count() < 8 {
        anyhow::bail!("Kata sandi baru minimal 8 karakter");
    }
    if p.chars().count() > 72 {
        // bcrypt hanya memakai 72 byte pertama — sisanya diam-diam diabaikan.
        anyhow::bail!("Kata sandi baru maksimal 72 karakter");
    }
    if !(p.chars().any(|c| c.is_uppercase()) && p.chars().any(|c| c.is_lowercase())) {
        anyhow::bail!("Kata sandi baru harus memuat huruf besar dan huruf kecil");
    }
    if !p.chars().any(|c| c.is_ascii_digit()) {
        anyhow::bail!("Kata sandi baru harus memuat minimal 1 angka");
    }
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
    #[test]
    fn aturan_kata_sandi_baru() {
        assert!(validate_new_password("Rahasia1").is_ok());
        assert!(validate_new_password("Rahasia!9x").is_ok());
        assert!(validate_new_password("Rhs1").is_err()); // terlalu pendek
        assert!(validate_new_password("rahasia123").is_err()); // tanpa huruf besar
        assert!(validate_new_password("RAHASIA123").is_err()); // tanpa huruf kecil
        assert!(validate_new_password("RahasiaSaja").is_err()); // tanpa angka
    }

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
