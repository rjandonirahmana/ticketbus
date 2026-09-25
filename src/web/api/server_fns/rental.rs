//! Server fn paket wisata (/wisata), bus charter (/sewa), dan permintaan sewa.

use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::{CharterBus, NewCharterBus, NewRentalRequest, NewTourPackage, RentalRequest, TourPackage};

#[cfg(feature = "ssr")]
use crate::repository::rental::ListingTable;

// ── Publik ───────────────────────────────────────────────────────────────────

#[server(ListTourPackages, "/api-fn")]
pub async fn list_tour_packages() -> Result<Vec<TourPackage>, ServerFnError> {
    let state = app_state().await?;
    state.rental_svc.public_packages().await.map_err(map_err)
}

#[server(ListCharterBuses, "/api-fn")]
pub async fn list_charter_buses() -> Result<Vec<CharterBus>, ServerFnError> {
    let state = app_state().await?;
    state.rental_svc.public_buses().await.map_err(map_err)
}

/// Nomor WhatsApp CS (format 62xxx) untuk tautan wa.me — kosong bila belum diatur.
#[server(GetCsContact, "/api-fn")]
pub async fn get_cs_contact() -> Result<String, ServerFnError> {
    let state = app_state().await?;
    Ok(state.rental_svc.cs_phone().to_string())
}

/// Boleh tanpa login; bila login, permintaan dikaitkan ke akunnya.
#[server(CreateRentalRequest, "/api-fn")]
pub async fn create_rental_request(input: NewRentalRequest) -> Result<RentalRequest, ServerFnError> {
    let state = app_state().await?;
    let user = current_claims().await.map(|c| c.user_id);
    state.rental_svc.create_request(user.as_deref(), input).await.map_err(map_err)
}

// ── Dashboard admin / mitra PO ───────────────────────────────────────────────

#[server(ListManagePackages, "/api-fn")]
pub async fn list_manage_packages() -> Result<Vec<TourPackage>, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.rental_svc.manage_packages(&claims).await.map_err(map_err)
}

#[server(ListManageBuses, "/api-fn")]
pub async fn list_manage_buses() -> Result<Vec<CharterBus>, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.rental_svc.manage_buses(&claims).await.map_err(map_err)
}

#[server(CreateTourPackage, "/api-fn")]
pub async fn create_tour_package(input: NewTourPackage) -> Result<String, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.rental_svc.create_package(&claims, input).await.map_err(map_err)
}

#[server(CreateCharterBus, "/api-fn")]
pub async fn create_charter_bus(input: NewCharterBus) -> Result<String, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.rental_svc.create_bus(&claims, input).await.map_err(map_err)
}

/// `kind`: "paket" | "bus".
#[server(SetListingActive, "/api-fn")]
pub async fn set_listing_active(kind: String, id: String, aktif: bool) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    let table = listing_table(&kind)?;
    state.rental_svc.set_active(&claims, table, &id, aktif).await.map_err(map_err)
}

#[server(DeleteListing, "/api-fn")]
pub async fn delete_listing(kind: String, id: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    let table = listing_table(&kind)?;
    state.rental_svc.delete(&claims, table, &id).await.map_err(map_err)
}

#[server(ListRentalRequests, "/api-fn")]
pub async fn list_rental_requests() -> Result<Vec<RentalRequest>, ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.rental_svc.list_requests(&claims).await.map_err(map_err)
}

#[server(SetRentalRequestStatus, "/api-fn")]
pub async fn set_rental_request_status(id: String, status: String) -> Result<(), ServerFnError> {
    let claims = require_role(&["admin", "merchant"]).await?;
    let state = app_state().await?;
    state.rental_svc.set_request_status(&claims, &id, &status).await.map_err(map_err)
}

#[cfg(feature = "ssr")]
fn listing_table(kind: &str) -> Result<ListingTable, ServerFnError> {
    match kind {
        "paket" => Ok(ListingTable::Package),
        "bus" => Ok(ListingTable::Bus),
        _ => Err(ServerFnError::ServerError("Jenis data tidak valid".into())),
    }
}
