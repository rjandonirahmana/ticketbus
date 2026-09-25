use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::{NewOrder, OrderDetail, OrderSummary};

#[server(CreateOrder, "/api-fn")]
pub async fn create_order(input: NewOrder) -> Result<OrderDetail, ServerFnError> {
    let claims = require_role(&["buyer"]).await?;
    let state = app_state().await?;
    state.order_svc.checkout(&claims.user_id, input).await.map_err(map_err)
}

#[server(ListMyOrders, "/api-fn")]
pub async fn list_my_orders() -> Result<Vec<OrderSummary>, ServerFnError> {
    let claims = require_role(&["buyer"]).await?;
    let state = app_state().await?;
    state.order_svc.list_by_buyer(&claims.user_id).await.map_err(map_err)
}

#[server(GetOrderDetail, "/api-fn")]
pub async fn get_order_detail(id: String) -> Result<Option<OrderDetail>, ServerFnError> {
    let claims = require_role(&["buyer"]).await?;
    let state = app_state().await?;
    state.order_svc.get_detail(&id, &claims.user_id).await.map_err(map_err)
}
