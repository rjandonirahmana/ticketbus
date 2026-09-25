use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::Armada;

#[server(ListArmadas, "/api-fn")]
pub async fn list_armadas() -> Result<Vec<Armada>, ServerFnError> {
    let state = app_state().await?;
    state.armada_svc.list().await.map_err(map_err)
}

#[server(ListMyArmadas, "/api-fn")]
pub async fn list_my_armadas() -> Result<Vec<Armada>, ServerFnError> {
    let claims = require_role(&["merchant"]).await?;
    let state = app_state().await?;
    state.armada_svc.list_by_merchant(&claims.user_id).await.map_err(map_err)
}

#[server(CreateArmada, "/api-fn")]
pub async fn create_armada(name: String, color_hex: String) -> Result<Armada, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.armada_svc.create(&claims, &name, &color_hex).await.map_err(map_err)
}

#[server(UpdateArmada, "/api-fn")]
pub async fn update_armada(id: String, name: String, color_hex: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.armada_svc.update(&claims, &id, &name, &color_hex).await.map_err(map_err)
}

#[server(DeleteArmada, "/api-fn")]
pub async fn delete_armada(id: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.armada_svc.delete(&claims, &id).await.map_err(map_err)
}
