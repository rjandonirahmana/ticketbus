#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub(super) async fn app_state() -> Result<std::sync::Arc<crate::state::AppState>, ServerFnError> {
    use axum::Extension;
    leptos_axum::extract::<Extension<std::sync::Arc<crate::state::AppState>>>()
        .await
        .map(|ext| ext.0)
        .map_err(|e| -> ServerFnError { ServerFnError::ServerError(format!("AppState unavailable: {e}")) })
}

#[cfg(feature = "ssr")]
pub(super) async fn current_claims() -> Option<crate::service::auth::Claims> {
    use axum::http::HeaderMap;
    let headers: HeaderMap = leptos_axum::extract().await.ok()?;
    let state = app_state().await.ok()?;
    crate::middleware::auth::current_claims(&headers, &state.auth_svc)
}

/// User-Agent & IP asal request untuk sesi/riwayat keamanan. Di produksi app
/// ada di belakang pingora, jadi IP klien diambil dari header proxy
/// (X-Forwarded-For elemen pertama, lalu X-Real-IP); tanpa header = "".
#[cfg(feature = "ssr")]
pub(super) async fn request_meta() -> crate::service::auth::SessionMeta {
    use axum::http::HeaderMap;
    let Ok(headers) = leptos_axum::extract::<HeaderMap>().await else {
        return Default::default();
    };
    let get = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).map(str::trim).unwrap_or("");
    let ip = get("x-forwarded-for").split(',').next().unwrap_or("").trim();
    let ip = if ip.is_empty() { get("x-real-ip") } else { ip };
    crate::service::auth::SessionMeta {
        user_agent: get("user-agent").chars().take(300).collect(),
        ip: ip.chars().take(64).collect(),
    }
}

/// SECURITY: server function adalah batas otorisasi SEBENARNYA — guard rute
/// sisi klien (`web/app/guards.rs`) hanya menyembunyikan UI dan bisa dilewati
/// dengan memanggil `/api-fn/*` langsung. Tiap server fn yang mengubah data
/// WAJIB memanggil ini, bukan hanya mengandalkan tampilan.
#[cfg(feature = "ssr")]
pub(super) async fn require_role(roles: &[&str]) -> Result<crate::service::auth::Claims, ServerFnError> {
    let claims = current_claims().await;
    crate::middleware::auth::require_role(claims, roles).map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[cfg(feature = "ssr")]
pub(super) fn map_err(e: anyhow::Error) -> ServerFnError {
    ServerFnError::ServerError(e.to_string())
}

/// Pasang cookie. `use_context`, BUKAN `expect_context`: `ResponseOptions`
/// bisa absen pada sebagian jalur SSR — jangan panik untuk hal yang
/// sebenarnya cuma "cookie tak bisa dipasang".
#[cfg(feature = "ssr")]
pub(super) fn set_cookie(name: &str, value: &str, max_age_secs: i64) {
    use axum::http::{header::SET_COOKIE, HeaderValue};
    use leptos_axum::ResponseOptions;

    let Some(resp) = use_context::<ResponseOptions>() else {
        tracing::warn!("ResponseOptions tidak tersedia — cookie dilewati");
        return;
    };
    let formatted = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}");
    if let Ok(hv) = HeaderValue::from_str(&formatted) {
        resp.append_header(SET_COOKIE, hv);
    }
}

#[cfg(feature = "ssr")]
pub(super) fn clear_cookie(name: &str) {
    use axum::http::{header::SET_COOKIE, HeaderValue};
    use leptos_axum::ResponseOptions;

    let Some(resp) = use_context::<ResponseOptions>() else {
        return;
    };
    let formatted =
        format!("{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT");
    if let Ok(hv) = HeaderValue::from_str(&formatted) {
        resp.append_header(SET_COOKIE, hv);
    }
}
