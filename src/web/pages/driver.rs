//! web/pages/driver.rs — "/driver/:token": HP driver berbagi lokasi bus.
//! Token di URL = otorisasinya (dibagikan mitra lewat WhatsApp), jadi driver
//! tak perlu akun. Posisi dikirim paling sering tiap 10 detik.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::web::api::{get_driver_trip, report_position, stop_tracking};
use crate::web::components::{
    clean_error, format_tanggal, geo_clear, geo_watch, keep_awake, map_focus, map_init, map_set_items,
    map_set_user, now_ms, spawn_client, BrandLogo, Icon, MapItem, RouteTimeline,
};

const MAP_ID: &str = "driver-map";
const KIRIM_TIAP_MS: f64 = 10_000.0;

#[component]
pub fn DriverPage() -> impl IntoView {
    let params = use_params_map();
    let token = move || params.get().get("token").unwrap_or_default();
    let trip = Resource::new(token, |t| async move { get_driver_trip(t).await });

    let aktif = RwSignal::new(false);
    let wid = RwSignal::new(-1i32);
    let terakhir_kirim = RwSignal::new(0.0f64);
    let terkirim = RwSignal::new(0u32);
    let pos = RwSignal::new(None::<(f64, f64, f64, f64)>); // lat, lng, akurasi, km/j
    let pesan = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let map_ok = RwSignal::new(false);

    let mulai = move |_| {
        error.set(String::new());
        pesan.set("Mencari sinyal GPS…".into());
        let t = token();
        let id = geo_watch(
            move |lat, lng, acc, kmh, arah| {
                pos.set(Some((lat, lng, acc, kmh)));
                if !map_ok.get_untracked() {
                    map_ok.set(map_init(MAP_ID, lat, lng, 16.0, true));
                }
                map_set_user(MAP_ID, lat, lng, acc.min(300.0));
                map_focus(MAP_ID, lat, lng, 16.0);
                let now = now_ms();
                if now - terakhir_kirim.get_untracked() < KIRIM_TIAP_MS {
                    return;
                }
                terakhir_kirim.set(now);
                let t = t.clone();
                spawn_client(async move {
                    let opt = |v: f64| (v >= 0.0).then_some(v);
                    match report_position(t, lat, lng, opt(kmh), opt(arah), Some(acc)).await {
                        Ok(()) => {
                            terkirim.update(|n| *n += 1);
                            pesan.set("Lokasi terkirim — penumpang melihat bus Anda di peta.".into());
                            error.set(String::new());
                        }
                        Err(e) => error.set(clean_error(&e.to_string())),
                    }
                });
            },
            move |e| error.set(e),
            true,
        );
        if id >= 0 {
            wid.set(id);
            aktif.set(true);
            keep_awake(true);
        }
    };

    let berhenti = move |_| {
        geo_clear(wid.get_untracked());
        wid.set(-1);
        aktif.set(false);
        keep_awake(false);
        let t = token();
        spawn_client(async move {
            match stop_tracking(t).await {
                Ok(()) => pesan.set("Berbagi lokasi dihentikan. Bus tak lagi tampil live di peta.".into()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
        });
    };
    on_cleanup(move || {
        geo_clear(wid.get_untracked());
        keep_awake(false);
    });

    view! {
        <div class="page page-narrow driver-page">
            <div class="driver-brand">
                <BrandLogo />
                <span class="pill pill-cta">"Mode Driver"</span>
            </div>
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    match trip.get().and_then(|r| r.ok()).flatten() {
                        None => {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="link_off" />
                                    <p>"Tautan driver tidak dikenal. Minta tautan terbaru ke mitra PO / admin."</p>
                                </div>
                            }
                                .into_any()
                        }
                        Some(d) => {
                            let s = d.schedule;
                            // Titik jemput ditandai supaya driver tahu tujuannya.
                            if let (Some(a), Some(b)) = (s.jemput_lat, s.jemput_lng) {
                                let item = MapItem {
                                    id: s.id.clone(),
                                    kind: "pickup",
                                    lat: a,
                                    lng: b,
                                    label: "Titik jemput".into(),
                                    badge: s.jam.clone(),
                                    tone: "",
                                    active: false,
                                    color: String::new(),
                                    to_lat: None,
                                    to_lng: None,
                                };
                                Effect::new(move |_| {
                                    if !map_ok.get() {
                                        map_ok.set(map_init(MAP_ID, a, b, 15.0, true));
                                    }
                                    map_set_items(MAP_ID, std::slice::from_ref(&item), false);
                                });
                            }
                            view! {
                                <section class="card trip-card">
                                    <header class="trip-head">
                                        <span class="op-logo" style=format!("--op:{}", s.armada_color_hex)>
                                            {crate::web::components::initials(&s.armada_name)}
                                        </span>
                                        <div class="trip-op">
                                            <h3>{s.armada_name.clone()}</h3>
                                            <p>{format!("{} · {} kursi", format_tanggal(&s.tanggal), s.kapasitas)}</p>
                                        </div>
                                    </header>
                                    <RouteTimeline jam=s.jam.clone() from=s.lokasi_jemput.clone() to=s.tujuan.clone() mid="Trip hari ini" />
                                </section>
                            }
                                .into_any()
                        }
                    }
                }}
            </Suspense>

            <section class=move || if aktif.get() { "gps-card on" } else { "gps-card" }>
                <div class="gps-head">
                    <span class="gps-icon">
                        <Icon name="satellite_alt" />
                    </span>
                    <div>
                        <strong>{move || if aktif.get() { "Berbagi Lokasi AKTIF" } else { "Berbagi Lokasi Mati" }}</strong>
                        <small>
                            {move || {
                                match pos.get() {
                                    Some((_, _, acc, kmh)) => {
                                        let v = if kmh >= 0.0 { format!(" · {:.0} km/j", kmh) } else { String::new() };
                                        format!("Akurasi ±{:.0} m{v} · {} kiriman", acc, terkirim.get())
                                    }
                                    None => "Posisi dikirim tiap 10 detik selama halaman ini terbuka".into(),
                                }
                            }}
                        </small>
                    </div>
                </div>
                <div id=MAP_ID class="driver-map"></div>
                {move || {
                    if aktif.get() {
                        view! {
                            <button type="button" class="btn btn-danger-ghost btn-block btn-lg" on:click=berhenti>
                                <Icon name="stop_circle" />
                                "Hentikan & Selesaikan Trip"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button type="button" class="btn btn-cta btn-block btn-lg" on:click=mulai>
                                <Icon name="play_circle" />
                                "Mulai Bagikan Lokasi"
                            </button>
                        }
                            .into_any()
                    }
                }}
            </section>

            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
            {move || (!pesan.get().is_empty()).then(|| view! { <p class="alert alert-ok">{pesan.get()}</p> })}

            <section class="card tips-card">
                <div class="tips-head">
                    <Icon name="lightbulb" />
                    <div>
                        <strong>"Agar lokasi terus terkirim"</strong>
                        <p>
                            "Biarkan halaman ini TERBUKA dan layar menyala (kami mencoba menjaganya tetap menyala). Browser berhenti mengirim lokasi saat layar dikunci atau aplikasi lain menutupi halaman. Colokkan pengisi daya untuk perjalanan jauh."
                        </p>
                    </div>
                </div>
            </section>
        </div>
    }
}
