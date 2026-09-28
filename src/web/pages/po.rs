//! web/pages/po.rs — "/po/:id" publik: profil PO terverifikasi, trayek tetap,
//! keberangkatan terdekat per trayek, dan armada beserta rating penumpang.

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::web::api::get_po_page;
use crate::web::components::{format_rupiah, Icon, PoCard, RatingStars, RouteTimeline, TripCard};

#[component]
pub fn PoPublicPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.get().get("id").unwrap_or_default();
    let page = Resource::new(id, get_po_page);

    view! {
        <div class="page po-page">
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    let Some(data) = page.get().and_then(|r| r.ok()).flatten() else {
                        return view! {
                            <div class="card empty-state">
                                <Icon name="storefront" />
                                <p>"PO tidak ditemukan atau belum terverifikasi."</p>
                                <a href="/" class="btn btn-primary">"Kembali ke Beranda"</a>
                            </div>
                        }
                            .into_any();
                    };
                    let nama = data.profil.nama_po.clone();
                    let alamat = data.profil.alamat.clone();
                    let rating: Vec<_> = data.armadas.iter().filter(|a| a.jumlah_rating > 0).collect();
                    let n_rating: i32 = rating.iter().map(|a| a.jumlah_rating).sum();
                    let avg = if n_rating > 0 {
                        rating.iter().map(|a| a.avg_rating_armada * a.jumlah_rating as f64).sum::<f64>() / n_rating as f64
                    } else {
                        0.0
                    };
                    // Satu kartu per trayek (keberangkatan terdekat), jadwal sekali jalan apa adanya.
                    let mut seen = std::collections::HashSet::new();
                    let berangkat: Vec<_> = data
                        .keberangkatan
                        .into_iter()
                        .filter(|s| s.route_id.as_ref().is_none_or(|r| seen.insert(r.clone())))
                        .take(8)
                        .collect();
                    view! {
                        <Title text=format!("{nama} — LajuBus") />
                        <PoCard profil=data.profil />
                        {(!alamat.is_empty())
                            .then(|| {
                                view! {
                                    <p class="note-card">
                                        <Icon name="home_work" />
                                        {alamat}
                                    </p>
                                }
                            })}
                        {(n_rating > 0)
                            .then(|| {
                                view! {
                                    <section class="card po-rating">
                                        <strong>{format!("{avg:.1}")}</strong>
                                        <RatingStars value=avg />
                                        <small>{format!("dari {n_rating} ulasan penumpang")}</small>
                                    </section>
                                }
                            })}

                        <div class="section-head">
                            <h2>
                                <Icon name="swap_horiz" />
                                "Trayek Tetap"
                            </h2>
                            <span class="pill pill-primary">{format!("{} trayek", data.routes.len())}</span>
                        </div>
                        {if data.routes.is_empty() {
                            view! { <p class="note-card">"PO ini belum membuka trayek tetap."</p> }.into_any()
                        } else {
                            view! {
                                <div class="ops-grid">
                                    {data
                                        .routes
                                        .into_iter()
                                        .map(|r| {
                                            view! {
                                                <article class="card route-card">
                                                    <header class="route-head">
                                                        <div>
                                                            <h3>{format!("{} → {}", r.asal, r.tujuan)}</h3>
                                                            <p>{crate::web::jam::hari_label(r.hari_operasi)}</p>
                                                        </div>
                                                        <span class="price">
                                                            <strong>{format!("Rp {}", format_rupiah(r.harga))}</strong>
                                                        </span>
                                                    </header>
                                                    <RouteTimeline
                                                        jam=r.jam_berangkat.clone()
                                                        tiba=r.jam_tiba.clone()
                                                        asal=r.asal.clone()
                                                        from=r.lokasi_jemput.clone()
                                                        to=r.tujuan.clone()
                                                        mid=format!("{} kursi · {}", r.kapasitas, r.konfigurasi)
                                                    />
                                                </article>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }}

                        <div class="section-head">
                            <h2>
                                <Icon name="event_available" />
                                "Keberangkatan Terdekat"
                            </h2>
                        </div>
                        {if berangkat.is_empty() {
                            view! { <p class="note-card">"Belum ada keberangkatan yang bisa dipesan."</p> }.into_any()
                        } else {
                            view! {
                                <div class="trip-grid">
                                    {berangkat.into_iter().map(|s| view! { <TripCard schedule=s /> }).collect_view()}
                                </div>
                            }
                                .into_any()
                        }}

                        <div class="section-head">
                            <h2>
                                <Icon name="directions_bus" />
                                "Armada"
                            </h2>
                            <span class="pill pill-primary">{format!("{} unit", data.armadas.len())}</span>
                        </div>
                        <div class="po-armada">
                            {data
                                .armadas
                                .into_iter()
                                .map(|a| {
                                    view! {
                                        <div class="card po-armada-item">
                                            <span class="swatch" style=format!("background:{}", a.color_hex)></span>
                                            <strong>{a.name}</strong>
                                            {if a.jumlah_rating > 0 {
                                                view! {
                                                    <small>
                                                        <RatingStars value=a.avg_rating_armada />
                                                        {format!(" {:.1} ({})", a.avg_rating_armada, a.jumlah_rating)}
                                                    </small>
                                                }
                                                    .into_any()
                                            } else {
                                                view! { <small>"Belum ada ulasan"</small> }.into_any()
                                            }}
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }}
            </Suspense>
        </div>
    }
}
