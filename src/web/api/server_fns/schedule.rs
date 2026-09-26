use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::{NewSchedule, Schedule};

#[server(ListSchedulesMonth, "/api-fn")]
pub async fn list_schedules_month(year: i32, month: u32) -> Result<Vec<Schedule>, ServerFnError> {
    let state = app_state().await?;
    state.schedule_svc.list_by_month(year, month).await.map_err(map_err)
}

/// Jadwal akan datang lintas armada — halaman browse publik (`/`).
#[server(ListSchedulesUpcoming, "/api-fn")]
pub async fn list_schedules_upcoming() -> Result<Vec<Schedule>, ServerFnError> {
    let state = app_state().await?;
    state.schedule_svc.list_upcoming().await.map_err(map_err)
}

#[server(ListSchedulesByArmada, "/api-fn")]
pub async fn list_schedules_by_armada(armada_id: Option<String>) -> Result<Vec<Schedule>, ServerFnError> {
    let state = app_state().await?;
    state
        .schedule_svc
        .list_by_armada(armada_id.as_deref())
        .await
        .map_err(map_err)
}

#[server(ListMySchedules, "/api-fn")]
pub async fn list_my_schedules() -> Result<Vec<Schedule>, ServerFnError> {
    let claims = require_role(&["merchant"]).await?;
    let state = app_state().await?;
    state.schedule_svc.list_by_merchant(&claims.user_id).await.map_err(map_err)
}

#[server(GetSchedule, "/api-fn")]
pub async fn get_schedule(id: String) -> Result<Option<Schedule>, ServerFnError> {
    let state = app_state().await?;
    state.schedule_svc.get(&id).await.map_err(map_err)
}

#[server(CreateSchedule, "/api-fn")]
pub async fn create_schedule(input: NewSchedule) -> Result<Schedule, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.schedule_svc.create(&claims, input).await.map_err(map_err)
}

#[server(DeleteSchedule, "/api-fn")]
pub async fn delete_schedule(id: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.schedule_svc.delete(&claims, &id).await.map_err(map_err)
}

/// Denah kursi + kursi terjual — halaman publik /pesan/:id.
#[server(GetSeatMap, "/api-fn")]
pub async fn get_seat_map(schedule_id: String) -> Result<Option<crate::web::models::SeatMap>, ServerFnError> {
    let state = app_state().await?;
    state.schedule_svc.seat_map(&schedule_id).await.map_err(map_err)
}

// ── Peta & GPS driver ────────────────────────────────────────────────────────

/// Jadwal hari ini yang punya lokasi (radar beranda & /peta). Publik; jarak
/// ke penumpang dihitung di browser.
#[server(ListNearbyBuses, "/api-fn")]
pub async fn list_nearby_buses() -> Result<Vec<crate::web::models::NearbyBus>, ServerFnError> {
    let state = app_state().await?;
    state.schedule_svc.nearby_today().await.map_err(map_err)
}

/// Token tautan driver — hanya pemilik jadwal / admin.
#[server(GetDriverToken, "/api-fn")]
pub async fn get_driver_token(schedule_id: String) -> Result<String, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.schedule_svc.driver_token(&claims, &schedule_id).await.map_err(map_err)
}

/// Halaman driver: token = otorisasinya (tanpa login).
#[server(GetDriverTrip, "/api-fn")]
pub async fn get_driver_trip(token: String) -> Result<Option<crate::web::models::DriverTrip>, ServerFnError> {
    let state = app_state().await?;
    state.schedule_svc.driver_trip(&token).await.map_err(map_err)
}

/// Kiriman posisi dari HP driver (±10 dtk). Dibatasi 1 kiriman / 3 dtk per token.
#[server(ReportPosition, "/api-fn")]
pub async fn report_position(
    token: String,
    lat: f64,
    lng: f64,
    speed_kmh: Option<f64>,
    heading: Option<f64>,
    accuracy_m: Option<f64>,
) -> Result<(), ServerFnError> {
    let state = app_state().await?;
    if !state.rate.allow(&format!("gps:{token}"), 1, 3) {
        return Ok(()); // kiriman terlalu rapat: abaikan diam-diam, bukan galat
    }
    state
        .schedule_svc
        .report_position(&token, lat, lng, speed_kmh, heading, accuracy_m)
        .await
        .map_err(map_err)
}

#[server(StopTracking, "/api-fn")]
pub async fn stop_tracking(token: String) -> Result<(), ServerFnError> {
    let state = app_state().await?;
    state.schedule_svc.stop_tracking(&token).await.map_err(map_err)
}
