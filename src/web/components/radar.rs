//! Radar bus: lokasi pengguna (browser), daftar jadwal hari ini yang punya
//! lokasi (auto-refresh 10 dtk), jarak/perkiraan tiba, dan penanda peta.
//! Dipakai kartu "Radar Bus di Sekitar" (beranda) dan halaman /peta.

use leptos::prelude::*;

use super::{format_rupiah, geo_clear, geo_watch, map_init, map_set_items, map_set_user, Icon, MapItem};
use crate::web::api::list_nearby_buses;
use crate::web::geo::{eta_menit, format_jarak, jarak_m, DEFAULT_CENTER};
use crate::web::models::NearbyBus;

/// Interval pembaruan radar (detik) — sama dengan teks di UI.
pub const REFRESH_SECS: u64 = 10;
/// Posisi lebih tua dari ini ditandai "sinyal lemah".
const STALE_SECS: i64 = 120;

/// Satu baris radar, sudah diperkaya jarak.
#[derive(Clone, PartialEq)]
pub struct RadarRow {
    pub bus: NearbyBus,
    /// Titik yang ditampilkan: posisi live bila ada, else titik jemput.
    pub lat: f64,
    pub lng: f64,
    pub live: bool,
    pub stale: bool,
    /// Jarak titik di atas ke pengguna (None bila lokasi pengguna belum ada).
    pub jarak: Option<f64>,
    /// Menit menuju titik jemput (hanya bila live & titik jemput diketahui).
    pub eta_jemput: Option<i64>,
}

pub fn build_rows(list: Vec<NearbyBus>, user: Option<(f64, f64)>) -> Vec<RadarRow> {
    let mut rows: Vec<RadarRow> = list
        .into_iter()
        .filter_map(|b| {
            let s = &b.schedule;
            let jemput = s.jemput_lat.zip(s.jemput_lng);
            let (lat, lng, live) = match &b.posisi {
                Some(p) => (p.lat, p.lng, true),
                None => {
                    let (a, c) = jemput?;
                    (a, c, false)
                }
            };
            let stale = b.posisi.as_ref().is_some_and(|p| p.umur_detik > STALE_SECS);
            let eta_jemput = match (&b.posisi, jemput) {
                (Some(p), Some((ja, jb))) => Some(eta_menit(jarak_m(p.lat, p.lng, ja, jb), p.speed_kmh)),
                _ => None,
            };
            let jarak = user.map(|(ua, ub)| jarak_m(ua, ub, lat, lng));
            Some(RadarRow { bus: b, lat, lng, live, stale, jarak, eta_jemput })
        })
        .collect();
    // Terdekat dulu; tanpa lokasi pengguna: yang live dulu, lalu jam berangkat.
    rows.sort_by(|a, b| match (a.jarak, b.jarak) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        _ => b.live.cmp(&a.live).then_with(|| a.bus.schedule.jam.cmp(&b.bus.schedule.jam)),
    });
    rows
}

pub fn map_items(rows: &[RadarRow], selected: Option<&str>) -> Vec<MapItem> {
    rows.iter()
        .map(|r| {
            let s = &r.bus.schedule;
            let badge = if r.live {
                match r.eta_jemput {
                    Some(m) => format!("{m} mnt"),
                    None => "Live".into(),
                }
            } else {
                s.jam.clone()
            };
            MapItem {
                id: s.id.clone(),
                kind: if r.live { "bus" } else { "pickup" },
                lat: r.lat,
                lng: r.lng,
                label: s.armada_name.clone(),
                badge,
                tone: if r.stale { "stale" } else if r.live { "live" } else { "" },
                active: selected == Some(s.id.as_str()),
                color: s.armada_color_hex.clone(),
                to_lat: if r.live { s.jemput_lat } else { None },
                to_lng: if r.live { s.jemput_lng } else { None },
            }
        })
        .collect()
}

/// "19:30" → menit menuju keberangkatan hari ini (WIB); None bila tak terbaca.
pub fn menit_ke_berangkat(jam: &str) -> Option<i64> {
    let t = chrono::NaiveTime::parse_from_str(jam.trim(), "%H:%M").ok()?;
    let now = (chrono::Utc::now() + chrono::Duration::hours(7)).time();
    Some((t - now).num_minutes())
}

/// Lokasi pengguna dari browser (akurasi rendah cukup untuk radar).
/// `status` berisi pesan bila izin ditolak / tidak didukung.
pub fn use_user_location() -> (RwSignal<Option<(f64, f64)>>, RwSignal<String>) {
    let pos = RwSignal::new(None::<(f64, f64)>);
    let status = RwSignal::new(String::new());
    let wid = StoredValue::new(-1i32);
    Effect::new(move |_| {
        let id = geo_watch(
            move |a, b, _, _, _| {
                pos.set(Some((a, b)));
                status.set(String::new());
            },
            move |e| status.set(e),
            false,
        );
        wid.set_value(id);
    });
    on_cleanup(move || geo_clear(wid.get_value()));
    (pos, status)
}

/// Data radar = murni sisi-klien (lokasi browser, refresh 10 dtk), jadi
/// `LocalResource`: dimuat setelah hydrate, tak pernah ikut SSR — tak ada
/// hydration mismatch walau dibaca di luar `<Suspense>`.
pub type NearbyRes = LocalResource<Result<Vec<NearbyBus>, ServerFnError>>;

/// Resource daftar bus + refetch otomatis tiap `REFRESH_SECS` (browser saja).
pub fn use_nearby() -> NearbyRes {
    let res = LocalResource::new(list_nearby_buses);
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(h) = set_interval_with_handle(move || res.refetch(), std::time::Duration::from_secs(REFRESH_SECS)) {
            on_cleanup(move || h.clear());
        }
    }
    res
}

/// Kartu "Radar Bus di Sekitar" untuk beranda.
#[component]
pub fn RadarCard() -> impl IntoView {
    const MAP_ID: &str = "radar-map";
    const RADIUS_M: f64 = 5_000.0;
    let (user, status) = use_user_location();
    let nearby = use_nearby();
    let rows = move || build_rows(nearby.get().and_then(|r| r.ok()).unwrap_or_default(), user.get());
    let ready = RwSignal::new(false);

    Effect::new(move |_| {
        let (a, b) = DEFAULT_CENTER;
        ready.set(map_init(MAP_ID, a, b, 12.0, false));
    });
    // Perbarui penanda setiap data/lokasi berubah.
    Effect::new(move |prev: Option<bool>| {
        if !ready.get() {
            return false;
        }
        if let Some((a, b)) = user.get() {
            map_set_user(MAP_ID, a, b, RADIUS_M);
        }
        let r = rows();
        // Pas-kan tampilan hanya sekali (tak "melompat" tiap refresh 10 dtk).
        let fit = prev != Some(true) && (!r.is_empty() || user.get().is_some());
        map_set_items(MAP_ID, &map_items(&r, None), fit);
        prev == Some(true) || fit
    });

    view! {
        <section class="card radar-card">
            <div class="radar-head">
                <span class="radar-icon">
                    <Icon name="radar" />
                </span>
                <div>
                    <h2>"Radar Bus di Sekitar"</h2>
                    <Suspense fallback=|| view! { <p>"Memuat radar…"</p> }>
                        <p>
                            {move || {
                                let r = rows();
                                let live = r.iter().filter(|x| x.live).count();
                                match user.get() {
                                    Some(_) => {
                                        let dekat = r.iter().filter(|x| x.jarak.is_some_and(|d| d <= RADIUS_M)).count();
                                        format!("{dekat} bus dalam radius 5 km · {live} live GPS")
                                    }
                                    None if !status.get().is_empty() => status.get(),
                                    None => format!("{} bus hari ini · {live} live GPS", r.len()),
                                }
                            }}
                        </p>
                    </Suspense>
                </div>
                <a href="/peta" class="chip-btn">
                    "Peta Penuh"
                    <Icon name="arrow_forward" />
                </a>
            </div>
            <a href="/peta" class="radar-map-link" aria-label="Buka Peta Bus">
                <div id=MAP_ID class="radar-map"></div>
            </a>
            <Suspense fallback=|| ()>
                {move || {
                    let Some(r) = rows().into_iter().next() else {
                        return view! {
                            <p class="note-card">
                                <Icon name="info" />
                                "Belum ada bus dengan lokasi hari ini. Lokasi tampil saat mitra menandai titik jemput atau driver berbagi GPS."
                            </p>
                        }
                            .into_any();
                    };
                    let s = r.bus.schedule.clone();
                    let info = match (r.live, r.jarak) {
                        (true, Some(d)) => format!("Live · {} dari Anda", format_jarak(d)),
                        (true, None) => "Live GPS aktif".into(),
                        (false, Some(d)) => format!("Titik jemput {} dari Anda", format_jarak(d)),
                        (false, None) => format!("Jemput {}", s.lokasi_jemput),
                    };
                    view! {
                        <div class="radar-top">
                            <span class="op-logo" style=format!("--op:{}", s.armada_color_hex)>
                                {crate::web::components::initials(&s.armada_name)}
                            </span>
                            <div class="radar-top-main">
                                <strong>{s.armada_name.clone()}</strong>
                                <small>
                                    {r.live.then(|| view! { <i class="live-dot"></i> })}
                                    {format!("{info} · {}", s.jam)}
                                </small>
                            </div>
                            <span class="radar-price">{format!("Rp {}", format_rupiah(s.harga))}</span>
                            <a href=format!("/pesan/{}", s.id) class="btn btn-cta btn-sm">
                                "Pesan"
                                <Icon name="arrow_forward" />
                            </a>
                        </div>
                    }
                        .into_any()
                }}
            </Suspense>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::models::{BusPosition, Schedule};

    fn sched(id: &str, lat: Option<f64>, lng: Option<f64>) -> Schedule {
        Schedule {
            id: id.into(),
            armada_id: "a".into(),
            armada_name: format!("PO {id}"),
            armada_color_hex: "#000000".into(),
            tanggal: "2026-09-26".into(),
            tujuan: "X".into(),
            lokasi_jemput: "Y".into(),
            jam: "19:00".into(),
            harga: 1,
            catatan: String::new(),
            kapasitas: 40,
            kursi_terjual: 0,
            driver_nama: String::new(),
            driver_telp: String::new(),
            konfigurasi: "2-2".into(),
            dua_dek: false,
            kursi_wanita: vec![],
            jemput_lat: lat,
            jemput_lng: lng,
        }
    }

    #[test]
    fn urut_terdekat_dan_live() {
        let dekat = NearbyBus { schedule: sched("dekat", Some(-6.21), Some(106.85)), posisi: None };
        let jauh = NearbyBus {
            schedule: sched("jauh", Some(-6.30), Some(106.90)),
            posisi: Some(BusPosition { lat: -6.40, lng: 106.95, speed_kmh: Some(60.0), heading: None, umur_detik: 5 }),
        };
        let tanpa = NearbyBus { schedule: sched("tanpa", None, None), posisi: None };
        let rows = build_rows(vec![jauh, dekat, tanpa], Some((-6.2088, 106.8456)));
        assert_eq!(rows.len(), 2, "jadwal tanpa lokasi dibuang");
        assert_eq!(rows[0].bus.schedule.id, "dekat");
        assert!(rows[1].live && rows[1].eta_jemput.is_some());
    }
}
