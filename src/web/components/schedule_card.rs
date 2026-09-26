use leptos::prelude::*;

use super::{format_rupiah, format_tanggal, initials, Icon};
use crate::web::models::Schedule;

/// Garis rute horizontal: jam + titik jemput → tujuan. `mid` tampil di atas
/// garis putus-putus, "Langsung" di bawahnya (seperti kartu jadwal Figma).
#[component]
pub fn RouteTimeline(
    #[prop(into)] jam: String,
    #[prop(into)] from: String,
    #[prop(into)] to: String,
    #[prop(into)] mid: String,
) -> impl IntoView {
    let jam = if jam.trim().is_empty() { "--:--".to_string() } else { jam };
    view! {
        <div class="timeline">
            <div class="tl-node">
                <span class="tl-time">{jam}</span>
                <span class="tl-place">{from}</span>
                <span class="tl-sub">"Titik Jemput"</span>
            </div>
            <div class="tl-track">
                <span class="tl-mid">{mid}</span>
                <div class="tl-line">
                    <i class="tl-dot"></i>
                    <span class="tl-dash">
                        <Icon name="directions_bus" />
                    </span>
                    <i class="tl-dot tl-dot-end"></i>
                </div>
                <span class="tl-direct">"Langsung"</span>
            </div>
            <div class="tl-node tl-node-end">
                <span class="tl-dest">{to}</span>
                <span class="tl-sub">"Tujuan"</span>
            </div>
        </div>
    }
}

/// Logo inisial operator/armada (ubin netral seperti desain) dengan strip
/// kecil warna penanda armada.
#[component]
pub fn OpLogo(#[prop(into)] name: String, #[prop(into)] color: String) -> impl IntoView {
    view! {
        <span class="op-logo" style=format!("--op:{color}")>
            {initials(&name)}
        </span>
    }
}

/// Kartu jadwal untuk penumpang (header operator, garis rute, harga + tombol
/// "Pilih Kursi" → /pesan/:id). `bookable=false` untuk jadwal yang sudah lewat.
#[component]
pub fn TripCard(schedule: Schedule, #[prop(default = true)] bookable: bool) -> impl IntoView {
    let s = schedule;
    let sisa = s.sisa_kursi();
    let low = sisa > 0 && sisa <= 5;
    let href = format!("/pesan/{}", s.id);
    let (btn_label, can_pick) = if !bookable {
        ("Sudah Berangkat", false)
    } else if sisa <= 0 {
        ("Kursi Habis", false)
    } else {
        ("Pilih Kursi", true)
    };

    view! {
        <article class="card trip-card">
            <header class="trip-head">
                <OpLogo name=s.armada_name.clone() color=s.armada_color_hex.clone() />
                <div class="trip-op">
                    <h3>
                        {s.armada_name.clone()}
                        <Icon name="verified" filled=true class="verified" />
                    </h3>
                    <p>{format_tanggal(&s.tanggal)}</p>
                </div>
                <span class="class-badge">
                    <Icon name="airline_seat_recline_extra" />
                    {format!("{} Seat · {}", s.kapasitas, s.konfigurasi)}
                </span>
            </header>
            <RouteTimeline jam=s.jam.clone() from=s.lokasi_jemput.clone() to=s.tujuan.clone() mid="Via rute PO" />
            <div class="facility-row">
                <span class="facility">
                    <Icon name="confirmation_number" />
                    "E-tiket instan"
                </span>
                {(!s.driver_nama.is_empty())
                    .then(|| {
                        view! {
                            <span class="facility">
                                <Icon name="badge" />
                                {format!("Driver {}", s.driver_nama)}
                            </span>
                        }
                    })}
                {(!s.catatan.is_empty())
                    .then(|| {
                        view! {
                            <span class="facility">
                                <Icon name="info" />
                                {s.catatan.clone()}
                            </span>
                        }
                    })}
            </div>
            <footer class="trip-foot">
                <div>
                    {if sisa <= 0 {
                        view! { <span class="seat-badge">"Kursi habis"</span> }.into_any()
                    } else if low {
                        view! {
                            <span class="seat-badge pulse">
                                <Icon name="alarm" />
                                {format!("Tersisa {sisa} kursi!")}
                            </span>
                        }
                            .into_any()
                    } else {
                        view! {
                            <span class="seat-badge ok">
                                <Icon name="check_circle" />
                                {format!("Tersedia {sisa} kursi")}
                            </span>
                        }
                            .into_any()
                    }}
                    <p class="price">
                        <strong>{format!("Rp {}", format_rupiah(s.harga))}</strong>
                        <small>"/kursi"</small>
                    </p>
                </div>
                {if can_pick {
                    view! {
                        <a href=href class=if low { "btn btn-cta" } else { "btn btn-primary" }>
                            {btn_label}
                            <Icon name="arrow_forward" />
                        </a>
                    }
                        .into_any()
                } else {
                    view! {
                        <button type="button" class="btn btn-soft" disabled=true>
                            {btn_label}
                        </button>
                    }
                        .into_any()
                }}
            </footer>
        </article>
    }
}

/// Kartu keberangkatan untuk dashboard merchant/admin: okupansi + driver,
/// tombol hapus hanya bila `on_delete` diberikan.
#[component]
pub fn ScheduleCard(
    schedule: Schedule,
    #[prop(optional)] index: Option<usize>,
    #[prop(optional, into)] on_delete: Option<Callback<String>>,
) -> impl IntoView {
    let s = schedule;
    let pct = if s.kapasitas > 0 { (s.kursi_terjual * 100 / s.kapasitas).clamp(0, 100) } else { 0 };
    let status = if s.sisa_kursi() <= 0 {
        ("Penuh", "pill pill-danger")
    } else if pct >= 75 {
        ("Hampir Penuh", "pill pill-warn")
    } else {
        ("Tersedia", "pill pill-ok")
    };
    let id = s.id.clone();
    let driver = if s.driver_nama.is_empty() { "Driver belum diisi".to_string() } else { s.driver_nama.clone() };

    view! {
        <article class="card ops-card">
            <header class="ops-head">
                {match index {
                    Some(i) => view! { <span class="ops-num">{format!("{:02}", i + 1)}</span> }.into_any(),
                    None => view! { <OpLogo name=s.armada_name.clone() color=s.armada_color_hex.clone() /> }.into_any(),
                }}
                <div class="ops-title">
                    <h3>
                        <span class="swatch" style=format!("background:{}", s.armada_color_hex)></span>
                        {s.armada_name.clone()}
                    </h3>
                    <p>{format_tanggal(&s.tanggal)}</p>
                </div>
                <span class=status.1>{status.0}</span>
            </header>
            <RouteTimeline jam=s.jam.clone() from=s.lokasi_jemput.clone() to=s.tujuan.clone() mid="Rp ".to_string() + &format_rupiah(s.harga) />
            <div class="ops-meta">
                <div class="occupancy">
                    <span>
                        <Icon name="airline_seat_recline_normal" />
                        {format!("Okupansi {}/{} kursi", s.kursi_terjual, s.kapasitas)}
                    </span>
                    <div class="bar">
                        <i style=format!("width:{pct}%")></i>
                    </div>
                </div>
                <span class="ops-driver">
                    <Icon name="badge" />
                    {driver}
                    {(!s.driver_telp.is_empty()).then(|| format!(" · {}", s.driver_telp))}
                </span>
                {(!s.catatan.is_empty())
                    .then(|| {
                        view! {
                            <span class="ops-note">
                                <Icon name="sticky_note_2" />
                                {s.catatan.clone()}
                            </span>
                        }
                    })}
            </div>
            {on_delete
                .map(|cb| {
                    view! {
                        <div class="ops-actions">
                            <button type="button" class="btn btn-danger-ghost" on:click=move |_| cb.run(id.clone())>
                                <Icon name="delete" />
                                "Hapus Jadwal"
                            </button>
                        </div>
                    }
                })}
        </article>
    }
}
