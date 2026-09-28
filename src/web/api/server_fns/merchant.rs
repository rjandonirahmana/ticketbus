//! Server fn Mitra PO: pendaftaran (/daftar-mitra), profil milik sendiri,
//! review admin, dan halaman profil PO publik (/po/:id).

use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::{MerchantProfile, NewMerchantProfile, PoPage};

/// Daftar Mitra PO: validasi profil, lalu kirim OTP + password awal lewat
/// WhatsApp (alur sama dengan pendaftaran biasa → /verify-otp).
#[server(RegisterMerchant, "/api-fn")]
pub async fn register_merchant(phone: String, name: String, profil: NewMerchantProfile) -> Result<(), ServerFnError> {
    let state = app_state().await?;
    let draft = state.merchant_svc.draft_json(profil).map_err(map_err)?;
    state.auth_svc.register_merchant(&phone, &name, &draft).await.map_err(map_err)
}

#[server(GetMyMerchantProfile, "/api-fn")]
pub async fn get_my_merchant_profile() -> Result<Option<MerchantProfile>, ServerFnError> {
    let claims = require_role(&["merchant"]).await?;
    let state = app_state().await?;
    state.merchant_svc.my_profile(&claims.user_id).await.map_err(map_err)
}

/// Simpan profil sendiri; profil yang ditolak otomatis diajukan ulang.
#[server(UpdateMyMerchantProfile, "/api-fn")]
pub async fn update_my_merchant_profile(profil: NewMerchantProfile) -> Result<(), ServerFnError> {
    let claims = require_role(&["merchant"]).await?;
    let state = app_state().await?;
    state.merchant_svc.update_mine(&claims.user_id, profil).await.map_err(map_err)
}

/// `status` kosong = semua.
#[server(ListMerchantProfiles, "/api-fn")]
pub async fn list_merchant_profiles(status: String) -> Result<Vec<MerchantProfile>, ServerFnError> {
    require_role(&["admin"]).await?;
    let state = app_state().await?;
    let status = Some(status.as_str()).filter(|s| !s.is_empty());
    state.merchant_svc.list(status).await.map_err(map_err)
}

#[server(DecideMerchant, "/api-fn")]
pub async fn decide_merchant(user_id: String, setujui: bool, catatan: String) -> Result<(), ServerFnError> {
    require_role(&["admin"]).await?;
    let state = app_state().await?;
    state.merchant_svc.decide(&user_id, setujui, &catatan).await.map_err(map_err)
}

#[server(GetPoPage, "/api-fn")]
pub async fn get_po_page(id: String) -> Result<Option<PoPage>, ServerFnError> {
    let state = app_state().await?;
    state.merchant_svc.public_page(&id).await.map_err(map_err)
}
