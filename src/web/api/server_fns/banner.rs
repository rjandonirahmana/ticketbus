//! Server fn banner promo beranda. Publik hanya membaca banner yang tayang;
//! mengelola banner khusus admin.

use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::{Banner, NewBanner};

#[server(ListBanners, "/api-fn")]
pub async fn list_banners() -> Result<Vec<Banner>, ServerFnError> {
    let state = app_state().await?;
    state.banner_svc.live().await.map_err(map_err)
}

#[server(ListManageBanners, "/api-fn")]
pub async fn list_manage_banners() -> Result<Vec<Banner>, ServerFnError> {
    require_role(&["admin"]).await?;
    let state = app_state().await?;
    state.banner_svc.all().await.map_err(map_err)
}

/// `id` kosong = banner baru.
#[server(SaveBanner, "/api-fn")]
pub async fn save_banner(id: String, input: NewBanner) -> Result<String, ServerFnError> {
    require_role(&["admin"]).await?;
    let state = app_state().await?;
    let id = Some(id.as_str()).filter(|s| !s.is_empty());
    state.banner_svc.save(id, input).await.map_err(map_err)
}

#[server(SetBannerActive, "/api-fn")]
pub async fn set_banner_active(id: String, aktif: bool) -> Result<(), ServerFnError> {
    require_role(&["admin"]).await?;
    let state = app_state().await?;
    state.banner_svc.set_active(&id, aktif).await.map_err(map_err)
}

#[server(ReleaseBannerNow, "/api-fn")]
pub async fn release_banner_now(id: String) -> Result<(), ServerFnError> {
    require_role(&["admin"]).await?;
    let state = app_state().await?;
    state.banner_svc.release_now(&id).await.map_err(map_err)
}

#[server(DeleteBanner, "/api-fn")]
pub async fn delete_banner(id: String) -> Result<(), ServerFnError> {
    require_role(&["admin"]).await?;
    let state = app_state().await?;
    state.banner_svc.delete(&id).await.map_err(map_err)
}
