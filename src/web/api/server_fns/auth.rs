//! web/api/server_fns/auth.rs — pendaftaran (OTP WhatsApp), login, sesi.

use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::PublicUser;

#[cfg(feature = "ssr")]
const SESSION_COOKIE_MAX_AGE: i64 = 30 * 24 * 3600;

#[server(RegisterRequest, "/api-fn")]
pub async fn register_request(phone: String, name: String, role: String) -> Result<(), ServerFnError> {
    let state = app_state().await?;
    state.auth_svc.register_request(&phone, &name, &role).await.map_err(map_err)
}

#[server(ResendOtp, "/api-fn")]
pub async fn resend_otp(phone: String) -> Result<(), ServerFnError> {
    let state = app_state().await?;
    state.auth_svc.resend_otp(&phone).await.map_err(map_err)
}

#[server(RegisterVerify, "/api-fn")]
pub async fn register_verify(phone: String, otp: String) -> Result<PublicUser, ServerFnError> {
    let state = app_state().await?;
    let meta = request_meta().await;
    let (user, session) = state.auth_svc.register_verify(&phone, &otp, &meta).await.map_err(map_err)?;
    set_cookie(crate::service::auth::SESSION_COOKIE, &session, SESSION_COOKIE_MAX_AGE);
    Ok(user)
}

#[server(LoginUser, "/api-fn")]
pub async fn login_user(phone: String, password: String) -> Result<PublicUser, ServerFnError> {
    let state = app_state().await?;
    let meta = request_meta().await;
    let (user, session) = state.auth_svc.login(&phone, &password, &meta).await.map_err(map_err)?;
    set_cookie(crate::service::auth::SESSION_COOKIE, &session, SESSION_COOKIE_MAX_AGE);
    Ok(user)
}

#[server(LogoutUser, "/api-fn")]
pub async fn logout_user() -> Result<(), ServerFnError> {
    if let (Some(claims), Ok(state)) = (current_claims().await, app_state().await) {
        state.auth_svc.logout(&claims).await;
    }
    clear_cookie(crate::service::auth::SESSION_COOKIE);
    Ok(())
}

#[server(CurrentSession, "/api-fn")]
pub async fn current_session() -> Result<Option<PublicUser>, ServerFnError> {
    let Some(claims) = current_claims().await else {
        return Ok(None);
    };
    let state = app_state().await?;
    // Dipanggil tiap halaman dimuat — tempat murah memperbarui "terakhir aktif".
    state.auth_svc.touch(&claims);
    state.auth_svc.user_by_id(&claims.user_id).await.map_err(map_err)
}

/// Selalu sukses untuk nomor tak terdaftar juga — lihat `AuthService::forgot_password`.
#[server(ForgotPassword, "/api-fn")]
pub async fn forgot_password(phone: String) -> Result<(), ServerFnError> {
    let state = app_state().await?;
    let meta = request_meta().await;
    state.auth_svc.forgot_password(&phone, &meta).await.map_err(map_err)
}

#[server(RequestPhoneChange, "/api-fn")]
pub async fn request_phone_change(new_phone: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    state.auth_svc.request_phone_change(&claims.user_id, &new_phone).await.map_err(map_err)
}

#[server(VerifyPhoneChange, "/api-fn")]
pub async fn verify_phone_change(otp: String) -> Result<PublicUser, ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    let meta = request_meta().await;
    state.auth_svc.verify_phone_change(&claims.user_id, &otp, &meta).await.map_err(map_err)
}

/// Kapan kata sandi terakhir diubah (RFC 3339) — halaman /akun/password.
#[server(PasswordChangedAt, "/api-fn")]
pub async fn password_changed_at() -> Result<String, ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    state.auth_svc.password_changed_at(&claims.user_id).await.map_err(map_err)
}

/// Ganti kata sandi. Cookie sesi diterbitkan ULANG: bila `logout_others`,
/// semua sesi lama dicabut dan hanya perangkat ini yang tetap masuk.
#[server(ChangePassword, "/api-fn")]
pub async fn change_password(current: String, new_password: String, logout_others: bool) -> Result<(), ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    let session = state
        .auth_svc
        .change_password(&claims, &current, &new_password, logout_others, &request_meta().await)
        .await
        .map_err(map_err)?;
    set_cookie(crate::service::auth::SESSION_COOKIE, &session, SESSION_COOKIE_MAX_AGE);
    Ok(())
}

// ── Pusat Keamanan (/akun/keamanan) ──────────────────────────────────────────

#[server(GetSecurityOverview, "/api-fn")]
pub async fn get_security_overview() -> Result<crate::web::models::SecurityOverview, ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    state.auth_svc.security_overview(&claims).await.map_err(map_err)
}

#[server(RevokeDevice, "/api-fn")]
pub async fn revoke_device(session_id: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    let meta = request_meta().await;
    state.auth_svc.revoke_device(&claims, &session_id, &meta).await.map_err(map_err)
}

/// Perangkat ini tetap masuk (cookie diterbitkan ulang), sisanya dikeluarkan.
#[server(LogoutOtherDevices, "/api-fn")]
pub async fn logout_other_devices() -> Result<(), ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    let meta = request_meta().await;
    let session = state.auth_svc.logout_other_devices(&claims, &meta).await.map_err(map_err)?;
    set_cookie(crate::service::auth::SESSION_COOKIE, &session, SESSION_COOKIE_MAX_AGE);
    Ok(())
}

/// Bekukan akun: semua sesi (termasuk ini) dicabut; cookie dihapus.
#[server(FreezeAccount, "/api-fn")]
pub async fn freeze_account() -> Result<(), ServerFnError> {
    let claims = require_role(&["buyer", "merchant", "admin"]).await?;
    let state = app_state().await?;
    let meta = request_meta().await;
    state.auth_svc.freeze_account(&claims, &meta).await.map_err(map_err)?;
    clear_cookie(crate::service::auth::SESSION_COOKIE);
    Ok(())
}
