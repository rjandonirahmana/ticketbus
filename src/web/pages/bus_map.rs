//! web/pages/bus_map.rs — "/peta": peta bus hari ini (posisi live dari HP
//! driver + titik jemput), jarak dari penumpang, pencarian, filter, radius.
//! Data radar murni sisi-klien (`LocalResource`), jadi aman dibaca di mana saja.

use leptos::prelude::*;

use crate::web::components::{
    build_rows, format_rupiah, map_fit_all, map_focus, map_init, map_items, map_on_select, map_set_items,
    map_set_user_label, map_zoom, menit_ke_berangkat, use_nearby, use_user_location, Icon, RadarRow, RouteTimeline,
};
use crate::web::geo::{format_jarak, jarak_m, DEFAULT_CENTER};

const MAP_ID: &str = "bus-map-full";

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    Semua,
    Live,
    Sleeper,
    Vip,
    Eksekutif,
}

impl Filter {
    fn cocok(self, r: &RadarRow) -> bool {
        match self {
            Filter::Semua => true,
            Filter::Live => r.live,
            Filter::Sleeper => r.bus.schedule.konfigurasi == "1-1",
            Filter::Vip => r.bus.schedule.konfigurasi == "2-1",
            Filter::Eksekutif => r.bus.schedule.konfigurasi == "2-2",
        }
    }
}

/// Radius pilihan (meter); 0 = semua.
const RADII: [(f64, &str); 4] = [(2_000.0, "2 km"), (5_000.0, "5 km"), (10_000.0, "10 km"), (0.0, "Semua")];

fn kelas(konfigurasi: &str) -> (&'static str, &'static str) {
    // (label, kelas warna badge)
    match konfigurasi {
        "1-1" => ("Sleeper Suite", "cls-orange"),
        "2-1" => ("VIP Legrest", "cls-violet"),
        _ => ("Eksekutif", "cls-blue"),
    }
}

#[component]
pub fn BusMapPage() -> impl IntoView {
    let (user, status) = use_user_location();
    let nearby = use_nearby();
    let q = RwSignal::new(String::new());
    let filter = RwSignal::new(Filter::Semua);
    let radius = RwSignal::new(0.0f64);
    let show_radius = RwSignal::new(false);
    let selected = RwSignal::new(None::<String>);
    let ready = RwSignal::new(false);

    let all_rows = move || build_rows(nearby.get().and_then(|r| r.ok()).unwrap_or_default(), user.get());
    let rows = move || {
        let needle = q.get().trim().to_lowercase();
        let f = filter.get();
        let rad = radius.get();
        all_rows()
            .into_iter()
            .filter(|r| f.cocok(r))
            .filter(|r| rad <= 0.0 || r.jarak.is_none_or(|d| d <= rad))
            .filter(|r| {
                let s = &r.bus.schedule;
                needle.is_empty()
                    || s.armada_name.to_lowercase().contains(&needle)
                    || s.tujuan.to_lowercase().contains(&needle)
                    || s.lokasi_jemput.to_lowercase().contains(&needle)
                    || s.asal.to_lowercase().contains(&needle)
            })
            .collect::<Vec<_>>()
    };
    // Titik jemput terdekat dari pengguna (untuk kartu "shelter" + chip lokasi).
    let jemput_terdekat = move || {
        let (ua, ub) = user.get()?;
        all_rows()
            .into_iter()
            .filter_map(|r| {
                let s = r.bus.schedule;
                let (a, b) = s.jemput_lat.zip(s.jemput_lng)?;
                Some((jarak_m(ua, ub, a, b), s.lokasi_jemput, a, b))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
    };

    Effect::new(move |_| {
        let (a, b) = DEFAULT_CENTER;
        if map_init(MAP_ID, a, b, 12.0, true) {
            map_on_select(MAP_ID, move |id| selected.set(Some(id)));
            ready.set(true);
        }
    });
    Effect::new(move |prev: Option<bool>| {
        if !ready.get() {
            return false;
        }
        if let Some((a, b)) = user.get() {
            // Lingkaran oranye = radius pilihan (bawaan 2 km seperti desain).
            let rad = if radius.get() > 0.0 { radius.get() } else { 2_000.0 };
            map_set_user_label(MAP_ID, a, b, rad, "Anda di sini");
        }
        let r = rows();
        let fit = prev != Some(true) && (!r.is_empty() || user.get().is_some());
        map_set_items(MAP_ID, &map_items(&r, selected.get().as_deref()), fit);
        prev == Some(true) || fit
    });

    let lacak = move |r: &RadarRow| {
        selected.set(Some(r.bus.schedule.id.clone()));
        map_focus(MAP_ID, r.lat, r.lng, 16.0);
        #[cfg(target_arch = "wasm32")]
        if let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id(MAP_ID)) {
            el.scroll_into_view();
        }
    };
    let ke_saya = move |_| match user.get_untracked() {
        Some((a, b)) => map_focus(MAP_ID, a, b, 15.0),
        None => status.set("Izinkan akses lokasi di browser untuk memusatkan peta.".into()),
    };

    let chip = move |f: Filter, label: &'static str, icon: &'static str| {
        view! {
            <button
                type="button"
                class=move || if filter.get() == f { "map-chip active" } else { "map-chip" }
                on:click=move |_| filter.set(f)
            >
                <Icon name=icon />
                {label}
                {move || (f == Filter::Semua).then(|| format!(" ({})", all_rows().len()))}
            </button>
        }
    };

    view! {
        <div class="page page-map">
            <div class="map-top">
                <span class="map-status">
                    <i class=move || if all_rows().iter().any(|r| r.live && !r.stale) { "status-dot" } else { "status-dot off" }></i>
                    {move || {
                        let live = all_rows().iter().filter(|r| r.live).count();
                        if live > 0 { format!("Live GPS · {live} bus") } else { "Menunggu GPS driver".to_string() }
                    }}
                </span>
                <span class="map-status">
                    <Icon name="my_location" />
                    {move || {
                        match (jemput_terdekat(), user.get()) {
                            (Some((_, nama, _, _)), _) => format!("{nama} & Sekitar"),
                            (None, Some(_)) => "Lokasi Anda aktif".to_string(),
                            (None, None) if !status.get().is_empty() => "Lokasi belum diizinkan".to_string(),
                            _ => "Mencari lokasi…".to_string(),
                        }
                    }}
                </span>
            </div>

            <div class="card map-search">
                <Icon name="explore" />
                <input
                    type="search"
                    placeholder="Cari halte, armada, atau PO terdekat…"
                    prop:value=move || q.get()
                    on:input=move |ev| q.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    class=move || if show_radius.get() { "tune-btn active" } else { "tune-btn" }
                    title="Atur radius"
                    on:click=move |_| show_radius.update(|v| *v = !*v)
                >
                    <Icon name="tune" />
                </button>
            </div>
            {move || {
                show_radius
                    .get()
                    .then(|| {
                        view! {
                            <div class="radius-panel">
                                <span class="label-caps">"Radius dari lokasi Anda"</span>
                                <div class="radius-chips">
                                    {RADII
                                        .into_iter()
                                        .map(|(m, label)| {
                                            view! {
                                                <button
                                                    type="button"
                                                    class=move || if radius.get() == m { "time-chip active" } else { "time-chip" }
                                                    on:click=move |_| radius.set(m)
                                                >
                                                    {label}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        }
                    })
            }}
            <div class="chip-row">
                {chip(Filter::Semua, "Semua Bus", "directions_bus")}
                {chip(Filter::Live, "Live GPS", "satellite_alt")}
                {chip(Filter::Sleeper, "Sleeper Bus", "airline_seat_flat")}
                {chip(Filter::Vip, "VIP 2-1", "airline_seat_recline_extra")}
                {chip(Filter::Eksekutif, "Eksekutif AC", "airline_seat_recline_normal")}
            </div>

            <div class="map-frame">
                <div id=MAP_ID class="full-map"></div>
                <div class="map-ctrl top">
                    <button type="button" title="Tampilkan semua bus" on:click=move |_| map_fit_all(MAP_ID)>
                        <Icon name="explore" />
                    </button>
                    <button type="button" title="Ke lokasi saya" on:click=ke_saya>
                        <Icon name="my_location" />
                    </button>
                </div>
                <div class="map-ctrl bottom">
                    <button type="button" title="Perbesar" on:click=move |_| map_zoom(MAP_ID, 1.0)>
                        <Icon name="add" />
                    </button>
                    <button type="button" title="Perkecil" on:click=move |_| map_zoom(MAP_ID, -1.0)>
                        <Icon name="remove" />
                    </button>
                    <button type="button" title="Segarkan radar" on:click=move |_| nearby.refetch()>
                        <Icon name="autorenew" />
                    </button>
                </div>
            </div>

            <section class="map-sheet">
                <div class="sheet-grip"></div>
                <div class="section-head">
                    <div>
                        <h2>"Armada Bus di Sekitar"</h2>
                        <p>
                            {move || {
                                let n = rows().len();
                                match (user.get(), radius.get()) {
                                    (Some(_), r) if r > 0.0 => {
                                        format!("{n} bus siap berangkat dalam radius {}", format_jarak(r))
                                    }
                                    _ => format!("{n} bus berangkat hari ini"),
                                }
                            }}
                        </p>
                    </div>
                    <span class="pill pill-quick">
                        <Icon name="bolt" />
                        "Pesan Cepat"
                    </span>
                </div>

                {move || {
                    jemput_terdekat()
                        .map(|(d, nama, a, b)| {
                            let menit = (d / 80.0).ceil().max(1.0) as i64; // ±4,8 km/j jalan kaki
                            view! {
                                <div class="shelter-card">
                                    <span class="shelter-icon">
                                        <Icon name="directions_walk" />
                                    </span>
                                    <div>
                                        <strong>{format!("Titik Jemput {nama}")}</strong>
                                        <small>{format!("Jarak {} · {menit} menit jalan kaki", format_jarak(d))}</small>
                                    </div>
                                    <a
                                        class="btn btn-soft btn-sm"
                                        href=format!("https://www.google.com/maps/dir/?api=1&destination={a},{b}&travelmode=walking")
                                        target="_blank"
                                        rel="noopener"
                                    >
                                        "Petunjuk"
                                        <Icon name="arrow_forward" />
                                    </a>
                                </div>
                            }
                        })
                }}

                {move || {
                    let list = rows();
                    if list.is_empty() {
                        return view! {
                            <div class="card empty-state">
                                <Icon name="location_off" />
                                <p>
                                    {if nearby.get().is_none() {
                                        "Memuat radar bus…"
                                    } else {
                                        "Belum ada bus dengan lokasi yang cocok. Coba perbesar radius atau hapus filter."
                                    }}
                                </p>
                            </div>
                        }
                            .into_any();
                    }
                    let sel = selected.get();
                    list.into_iter()
                        .map(|r| {
                            let active = sel.as_deref() == Some(r.bus.schedule.id.as_str());
                            let rr = r.clone();
                            view! { <BusRow r=r active=active on_track=move |_| lacak(&rr) /> }
                        })
                        .collect_view()
                        .into_any()
                }}

                <div class="radar-foot">
                    <span>
                        <Icon name="sensors" />
                        "Update radar otomatis setiap " <b>"10 detik"</b>
                    </span>
                    <button type="button" class="link-btn accent" on:click=move |_| show_radius.update(|v| *v = !*v)>
                        "Atur Radius"
                    </button>
                </div>
            </section>
        </div>
    }
}

#[component]
fn BusRow(r: RadarRow, active: bool, #[prop(into)] on_track: Callback<()>) -> impl IntoView {
    let s = r.bus.schedule.clone();
    let sisa = s.sisa_kursi();
    let (kelas_label, kelas_cls) = kelas(&s.konfigurasi);
    let berangkat = menit_ke_berangkat(&s.jam);

    // Baris status: kiri = keadaan bus, kanan = waktu.
    let (status_icon, status_cls, kiri) = match (r.live, r.stale, r.jarak) {
        (true, true, _) => (
            "signal_cellular_connected_no_internet_0_bar",
            "bus-status stale",
            format!("Sinyal terakhir {} mnt lalu", r.bus.posisi.as_ref().map(|p| p.umur_detik / 60).unwrap_or(0)),
        ),
        (true, false, Some(d)) => ("", "bus-status live", format!("Mendekati · {} dari Anda", format_jarak(d))),
        (true, false, None) => ("", "bus-status live", "Live GPS aktif".to_string()),
        (false, _, Some(d)) => ("location_on", "bus-status", format!("Standby di {} · {}", s.lokasi_jemput, format_jarak(d))),
        (false, _, None) => ("location_on", "bus-status", format!("Standby di {}", s.lokasi_jemput)),
    };
    let (kanan, kanan_cls) = match (r.eta_jemput, berangkat) {
        (Some(m), _) => (format!("Tiba dlm {m} mnt"), ""),
        (None, Some(m)) if (1..=90).contains(&m) => (format!("Berangkat {m} Menit Lagi"), "urgent"),
        (None, Some(m)) if m <= 0 && m > -90 => ("Jam berangkat".to_string(), "urgent"),
        _ => (format!("Berangkat {}", s.jam), ""),
    };
    // Tombol utama mengikuti keadaan (seperti desain): live → Lacak + Pilih
    // Kursi; berangkat ≤ 60 mnt → Pesan Kilat; selain itu → Lihat Denah.
    let segera = berangkat.is_some_and(|m| (0..=60).contains(&m));
    let pesan = format!("/pesan/{}", s.id);

    view! {
        <article class=if active { "card bus-row active" } else { "card bus-row" }>
            <header class="trip-head">
                <span class="op-logo" style=format!("--op:{}", s.armada_color_hex)>
                    {crate::web::components::initials(&s.armada_name)}
                </span>
                <div class="trip-op">
                    <h3>
                        {s.armada_name.clone()}
                        <Icon name="verified" filled=true class="verified" />
                    </h3>
                    <p>{format!("{kelas_label} · {} kursi · {}", s.kapasitas, s.konfigurasi)}</p>
                </div>
                <span class=format!("cls-badge {kelas_cls}")>{kelas_label}</span>
            </header>
            <div class=status_cls>
                <span>
                    {if status_icon.is_empty() {
                        view! { <i class="live-dot"></i> }.into_any()
                    } else {
                        view! { <Icon name=status_icon /> }.into_any()
                    }}
                    {kiri}
                </span>
                <b class=kanan_cls>{kanan}</b>
            </div>
            <RouteTimeline
                jam=s.jam.clone()
                tiba=s.jam_tiba.clone()
                asal=s.asal.clone()
                from=s.lokasi_jemput.clone()
                to=s.tujuan.clone()
                mid="Langsung"
            />
            <footer class="trip-foot">
                <div>
                    <span class=if sisa <= 5 { "label-caps seat-left low" } else { "label-caps seat-left" }>
                        {format!("Sisa {sisa} kursi")}
                    </span>
                    <p class="price">
                        <strong>{format!("Rp {}", format_rupiah(s.harga))}</strong>
                        <small>"/kursi"</small>
                    </p>
                </div>
                <div class="bus-actions">
                    {if sisa <= 0 {
                        view! {
                            <button type="button" class="btn btn-soft btn-sm" disabled=true>
                                "Kursi Penuh"
                            </button>
                        }
                            .into_any()
                    } else if r.live {
                        view! {
                            <button type="button" class="btn btn-outline btn-sm" on:click=move |_| on_track.run(())>
                                <Icon name="map" />
                                "Lacak"
                            </button>
                            <a href=pesan class="btn btn-brown btn-sm">
                                "Pilih Kursi"
                                <Icon name="chevron_right" />
                            </a>
                        }
                            .into_any()
                    } else if segera {
                        view! {
                            <a href=pesan class="btn btn-navy btn-sm">
                                "Pesan Kilat"
                                <Icon name="bolt" />
                            </a>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button type="button" class="btn btn-outline btn-sm" on:click=move |_| on_track.run(())>
                                <Icon name="map" />
                            </button>
                            <a href=pesan class="btn btn-soft btn-sm">
                                "Lihat Denah"
                                <Icon name="event_seat" />
                            </a>
                        }
                            .into_any()
                    }}
                </div>
            </footer>
        </article>
    }
}
