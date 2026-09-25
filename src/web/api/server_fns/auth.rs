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
    let (user, session) = state.auth_svc.register_verify(&phone, &otp).await.map_err(map_err)?;
    set_cookie(crate::service::auth::SESSION_COOKIE, &session, SESSION_COOKIE_MAX_AGE);
    Ok(user)
}

#[server(LoginUser, "/api-fn")]
pub async fn login_user(phone: String, password: String) -> Result<PublicUser, ServerFnError> {
    let state = app_state().await?;
    let (user, session) = state.auth_svc.login(&phone, &password).await.map_err(map_err)?;
    set_cookie(crate::service::auth::SESSION_COOKIE, &session, SESSION_COOKIE_MAX_AGE);
    Ok(user)
}

#[server(LogoutUser, "/api-fn")]
pub async fn logout_user() -> Result<(), ServerFnError> {
    clear_cookie(crate::service::auth::SESSION_COOKIE);
    Ok(())
}

#[server(CurrentSession, "/api-fn")]
pub async fn current_session() -> Result<Option<PublicUser>, ServerFnError> {
    let Some(claims) = current_claims().await else {
        return Ok(None);
    };
    let state = app_state().await?;
    state.auth_svc.user_by_id(&claims.user_id).await.map_err(map_err)
}

/// Selalu sukses untuk nomor tak terdaftar juga — lihat `AuthService::forgot_password`.
#[server(ForgotPassword, "/api-fn")]
pub async fn forgot_password(phone: String) -> Result<(), ServerFnError> {
    let state = app_state().await?;
    state.auth_svc.forgot_password(&phone).await.map_err(map_err)
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
    state.auth_svc.verify_phone_change(&claims.user_id, &otp).await.map_err(map_err)
}
