//! middleware/auth.rs — gerbang sesi untuk SEMUA peran (buyer/merchant/
//! admin), satu mekanisme untuk ketiganya. Server function adalah batas
//! otorisasi SEBENARNYA (guard UI di `web/app/guards.rs` cuma menyembunyikan
//! tombol, bisa dilewati dengan memanggil `/api-fn/*` langsung) — jadi setiap
//! server fn yang mengubah data WAJIB memanggil `require_role` di sini.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

use crate::service::auth::{AuthService, Claims, SESSION_COOKIE};
use crate::state::AppState;

/// Peran yang boleh membuka halaman di bawah awalan path tertentu.
const PAGE_RULES: &[(&str, &[&str])] = &[
    ("/admin", &["admin"]),
    ("/merchant", &["merchant"]),
    ("/orders", &["buyer"]),
    ("/akun", &["buyer", "merchant", "admin"]),
];

/// Alihkan halaman terlindung ke `/login` SEBELUM Leptos mulai merender.
///
/// Guard di `web/app/guards.rs` tak cukup untuk ini: HTML SSR di-stream, jadi
/// saat guard tahu sesinya tak cocok, status 200 sudah terkirim dan
/// `<Redirect/>` tak bisa lagi menjadi 302 — yang keluar cuma kerangka
/// "Memuat…" yang menunggu WASM untuk berpindah halaman. Di sini keputusan
/// diambil dari cookie sebelum satu byte pun dikirim. (Data tetap aman tanpa
/// ini — server fn memeriksa peran sendiri; ini soal UX & perayap.)
pub async fn page_guard(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let rule = PAGE_RULES
        .iter()
        .find(|(prefix, _)| path == *prefix || path.starts_with(&format!("{prefix}/")));
    if let Some((_, roles)) = rule {
        let claims = current_claims(req.headers(), &state.auth_svc);
        if require_role(claims, roles).is_err() {
            return Redirect::to("/login").into_response();
        }
    }
    next.run(req).await
}

pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{name}=");
    raw.split(';').map(|p| p.trim()).find_map(|part| {
        part.strip_prefix(&prefix)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(String::from)
    })
}

pub fn current_claims(headers: &HeaderMap, auth: &AuthService) -> Option<Claims> {
    let value = cookie_value(headers, SESSION_COOKIE)?;
    auth.verify_session(&value)
}

/// `claims` harus `Some` DAN perannya ada di `roles`, atau ditolak dengan
/// pesan yang jelas soal alasannya (bukan sekadar "unauthorized").
pub fn require_role(claims: Option<Claims>, roles: &[&str]) -> anyhow::Result<Claims> {
    let claims = claims.ok_or_else(|| anyhow::anyhow!("Tidak terautentikasi — silakan login"))?;
    if !roles.contains(&claims.role.as_str()) {
        anyhow::bail!("Akses ditolak: perlu peran {roles:?}");
    }
    Ok(claims)
}
