//! web/api/upload.rs — POST /upload/trip-photo (multipart/form-data).
//! Fields: file (required), armada_id (required), caption (optional).
//! Admin atau merchant PEMILIK armada tersebut — dicek lewat cookie sesi,
//! sama seperti server function.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use axum::{
    extract::{Extension, Multipart},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::state::AppState;

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

pub async fn trip_photo_upload(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let claims = crate::middleware::auth::current_claims(&headers, &state.auth_svc);
    let Ok(claims) = crate::middleware::auth::require_role(claims, &["admin", "merchant"]) else {
        return err(StatusCode::UNAUTHORIZED, "Perlu login admin atau merchant");
    };

    let mut data: Option<Vec<u8>> = None;
    let mut armada_id: Option<String> = None;
    let mut caption = String::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => return err(StatusCode::BAD_REQUEST, format!("Multipart error: {e}")),
        };
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => match field.bytes().await {
                Ok(bytes) => data = Some(bytes.to_vec()),
                Err(e) => return err(StatusCode::BAD_REQUEST, format!("Gagal baca file: {e}")),
            },
            "armada_id" => armada_id = field.text().await.ok().filter(|s| !s.is_empty()),
            "caption" => caption = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }

    let Some(data) = data.filter(|d| !d.is_empty()) else {
        return err(StatusCode::BAD_REQUEST, "Field 'file' tidak ada dalam request");
    };
    let Some(armada_id) = armada_id else {
        return err(StatusCode::BAD_REQUEST, "Field 'armada_id' wajib diisi");
    };

    if let Err(e) = state.armada_svc.check_ownership(&claims, &armada_id).await {
        return err(StatusCode::FORBIDDEN, e.to_string());
    }

    let url = match state.storage.upload_trip_photo(data).await {
        Ok(u) => u,
        Err(e) => return err(StatusCode::BAD_REQUEST, e.to_string()),
    };

    match state.photo_svc.create(&armada_id, &url, &caption).await {
        Ok(photo) => Json(json!({ "photo": photo })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// POST /upload/listing-photo — foto sampul paket wisata / bus sewa.
/// Field: file. Balasan `{ "url": "..." }`; URL itu lalu dikirim bersama form
/// paket/bus lewat server fn (yang memvalidasi kepemilikan).
pub async fn listing_photo_upload(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let claims = crate::middleware::auth::current_claims(&headers, &state.auth_svc);
    if crate::middleware::auth::require_role(claims, &["admin", "merchant"]).is_err() {
        return err(StatusCode::UNAUTHORIZED, "Perlu login admin atau mitra PO");
    }

    let mut data: Option<Vec<u8>> = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => return err(StatusCode::BAD_REQUEST, format!("Multipart error: {e}")),
        };
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(bytes) => data = Some(bytes.to_vec()),
                Err(e) => return err(StatusCode::BAD_REQUEST, format!("Gagal baca file: {e}")),
            }
        }
    }
    let Some(data) = data.filter(|d| !d.is_empty()) else {
        return err(StatusCode::BAD_REQUEST, "Field 'file' tidak ada dalam request");
    };
    match state.storage.upload_trip_photo(data).await {
        Ok(url) => Json(json!({ "url": url })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}
