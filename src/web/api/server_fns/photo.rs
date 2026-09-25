//! Upload sendiri lewat route multipart `/upload/trip-photo`
//! (`web/api/upload.rs`) — server function tak cocok untuk body file besar.

use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::TripPhoto;

#[server(ListPhotos, "/api-fn")]
pub async fn list_photos(armada_id: Option<String>) -> Result<Vec<TripPhoto>, ServerFnError> {
    let state = app_state().await?;
    state.photo_svc.list(armada_id.as_deref()).await.map_err(map_err)
}

#[server(DeletePhoto, "/api-fn")]
pub async fn delete_photo(id: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    if claims.role == "merchant" {
        let armada_id = state.photo_svc.armada_id_of(&id).await.map_err(map_err)?;
        state.armada_svc.check_ownership(&claims, &armada_id).await.map_err(map_err)?;
    }
    state.photo_svc.delete(&id).await.map_err(map_err)
}

/// Foto dari semua armada milik merchant yang sedang login.
#[server(ListMyPhotos, "/api-fn")]
pub async fn list_my_photos() -> Result<Vec<TripPhoto>, ServerFnError> {
    let claims = require_role(&["merchant"]).await?;
    let state = app_state().await?;
    state.photo_svc.list_by_merchant(&claims.user_id).await.map_err(map_err)
}
