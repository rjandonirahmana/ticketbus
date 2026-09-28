//! Server fn trayek tetap & penugasan bus/driver harian (admin + mitra PO;
//! lingkup data ditegakkan service/route.rs).

use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::{NewRoute, Route, Schedule};

#[server(ListRoutes, "/api-fn")]
pub async fn list_routes() -> Result<Vec<Route>, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.route_svc.list(&claims).await.map_err(map_err)
}

/// `id` kosong = trayek baru (boleh sekalian rute balik); selain itu ubah.
#[server(SaveRoute, "/api-fn")]
pub async fn save_route(id: String, input: NewRoute) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    if id.is_empty() {
        state.route_svc.create(&claims, input).await.map_err(map_err)
    } else {
        state.route_svc.update(&claims, &id, input).await.map_err(map_err)
    }
}

#[server(SetRouteActive, "/api-fn")]
pub async fn set_route_active(id: String, aktif: bool) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.route_svc.set_active(&claims, &id, aktif).await.map_err(map_err)
}

#[server(DeleteRoute, "/api-fn")]
pub async fn delete_route(id: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.route_svc.delete(&claims, &id).await.map_err(map_err)
}

/// Keberangkatan trayek 14 hari ke depan (termasuk yang dibatalkan).
#[server(ListRouteSchedules, "/api-fn")]
pub async fn list_route_schedules(id: String) -> Result<Vec<Schedule>, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.route_svc.upcoming(&claims, &id).await.map_err(map_err)
}

/// Ganti bus & driver untuk satu keberangkatan.
#[server(AssignSchedule, "/api-fn")]
pub async fn assign_schedule(
    schedule_id: String,
    armada_id: String,
    driver_nama: String,
    driver_telp: String,
) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state
        .route_svc
        .assign(&claims, &schedule_id, &armada_id, &driver_nama, &driver_telp)
        .await
        .map_err(map_err)
}

#[server(SetScheduleBatal, "/api-fn")]
pub async fn set_schedule_batal(schedule_id: String, batal: bool) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.route_svc.set_batal(&claims, &schedule_id, batal).await.map_err(map_err)
}
