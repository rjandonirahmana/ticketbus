//! Pemilih koordinat titik jemput (form jadwal): klik peta atau pakai lokasi
//! perangkat. Nilai disimpan sebagai teks di dua signal (lat, lng).

use leptos::prelude::*;

use super::{geo_clear, geo_watch, map_focus, map_init, map_pick, Icon};
use crate::web::geo::DEFAULT_CENTER;

const MAP_ID: &str = "pickup-map";

#[component]
pub fn PickupPicker(
    lat: RwSignal<String>,
    lng: RwSignal<String>,
    /// Id elemen peta — beda-bedakan bila dua form berpeta bisa terbuka bersamaan.
    #[prop(optional)]
    map_id: Option<&'static str>,
) -> impl IntoView {
    let map_id = map_id.unwrap_or(MAP_ID);
    let info = RwSignal::new(String::new());
    let current = move || {
        let (a, b) = (lat.get_untracked(), lng.get_untracked());
        a.parse::<f64>().ok().zip(b.parse::<f64>().ok())
    };

    // Pasang peta setelah elemen ada di DOM (hanya di browser).
    Effect::new(move |_| {
        let (c_lat, c_lng) = current().unwrap_or(DEFAULT_CENTER);
        if map_init(map_id, c_lat, c_lng, if current().is_some() { 15.0 } else { 11.0 }, true) {
            map_pick(map_id, current(), move |a, b| {
                lat.set(format!("{a:.6}"));
                lng.set(format!("{b:.6}"));
            });
        } else {
            info.set("Peta tidak dapat dimuat — periksa koneksi internet.".into());
        }
    });

    let pakai_lokasi = move |_| {
        info.set("Mencari lokasi perangkat…".into());
        let wid = RwSignal::new(-1i32);
        let id = geo_watch(
            move |a, b, acc, _, _| {
                lat.set(format!("{a:.6}"));
                lng.set(format!("{b:.6}"));
                info.set(format!("Lokasi perangkat dipakai (akurasi ±{} m). Klik peta untuk menggeser.", acc.round()));
                map_init(map_id, a, b, 16.0, true);
                map_pick(map_id, Some((a, b)), move |x, y| {
                    lat.set(format!("{x:.6}"));
                    lng.set(format!("{y:.6}"));
                });
                map_focus(map_id, a, b, 16.0);
                geo_clear(wid.get_untracked());
            },
            move |e| info.set(e),
            true,
        );
        wid.set(id);
    };

    view! {
        <div class="field">
            <span class="field-label">"Titik Jemput di Peta (untuk Radar Bus)"</span>
            <div id=map_id class="pick-map"></div>
            <div class="pick-bar">
                <span class="pick-coord">
                    <Icon name="pin_drop" />
                    {move || {
                        let (a, b) = (lat.get(), lng.get());
                        if a.is_empty() { "Belum dipilih — klik peta".to_string() } else { format!("{a}, {b}") }
                    }}
                </span>
                <button type="button" class="btn btn-soft btn-sm" on:click=pakai_lokasi>
                    <Icon name="my_location" />
                    "Lokasi saya"
                </button>
                {move || {
                    (!lat.get().is_empty())
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-sm"
                                    on:click=move |_| {
                                        lat.set(String::new());
                                        lng.set(String::new());
                                    }
                                >
                                    "Hapus"
                                </button>
                            }
                        })
                }}
            </div>
            {move || (!info.get().is_empty()).then(|| view! { <p class="field-hint">{info.get()}</p> })}
        </div>
    }
}
