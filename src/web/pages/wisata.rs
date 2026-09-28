//! web/pages/wisata.rs — "/wisata" publik: paket bus destinasi wisata +
//! form rencana wisata custom. Booking = permintaan sewa (disimpan) yang
//! ditindaklanjuti CS / mitra PO lewat WhatsApp.

use leptos::prelude::*;

use crate::web::api::{get_cs_contact, list_tour_packages};
use crate::web::components::{format_rupiah, wa_link, Icon, RentalRequestSheet, RequestPrefill, RequestTarget};
use crate::web::models::TourPackage;

#[derive(Clone, Copy, PartialEq)]
enum Kategori {
    Semua,
    Alam,
    BudayaEdukasi,
    Religi,
    Lainnya,
}

impl Kategori {
    const ALL: [(Kategori, &'static str, &'static str); 5] = [
        (Kategori::Semua, "verified", "Semua Paket"),
        (Kategori::Alam, "forest", "Wisata Alam"),
        (Kategori::BudayaEdukasi, "temple_buddhist", "Budaya & Edukasi"),
        (Kategori::Religi, "mosque", "Wisata Religi"),
        (Kategori::Lainnya, "explore", "Lainnya"),
    ];

    fn cocok(self, k: &str) -> bool {
        match self {
            Kategori::Semua => true,
            Kategori::Alam => k == "alam",
            Kategori::BudayaEdukasi => k == "budaya" || k == "edukasi",
            Kategori::Religi => k == "religi",
            Kategori::Lainnya => k == "lainnya",
        }
    }
}

const DURASI: [&str; 4] = ["1 Hari (One Day Trip)", "2H 1M (2D1N)", "3H 2M (3D2N)", "4H 3M atau lebih"];
const ROMBONGAN: [&str; 4] = ["10 - 19 Orang", "20 - 35 Orang", "36 - 59 Orang", "60+ Orang (multi bus)"];

#[component]
pub fn WisataPage() -> impl IntoView {
    let packages = Resource::new(|| (), |_| list_tour_packages());
    let cs = Resource::new(|| (), |_| get_cs_contact());
    let cs_phone = move || cs.get().and_then(|r| r.ok()).unwrap_or_default();

    let kategori = RwSignal::new(Kategori::Semua);
    let kawasan = RwSignal::new(None::<String>);
    let target = RwSignal::new(None::<RequestTarget>);

    // Form rencana wisata custom.
    let c_rute = RwSignal::new(String::new());
    let c_durasi = RwSignal::new(DURASI[0].to_string());
    let c_rombongan = RwSignal::new(ROMBONGAN[1].to_string());
    let c_pref = RwSignal::new(String::new());

    let all = move || packages.get().and_then(|r| r.ok()).unwrap_or_default();
    let by_kategori = move || {
        let k = kategori.get();
        all().into_iter().filter(|p| k.cocok(&p.kategori)).collect::<Vec<_>>()
    };
    let filtered = move || {
        let kw = kawasan.get();
        by_kategori()
            .into_iter()
            .filter(|p| kw.as_ref().is_none_or(|k| &p.kawasan == k))
            .collect::<Vec<_>>()
    };

    let open_custom = move |_| {
        let jumlah = match c_rombongan.get_untracked().as_str() {
            s if s.starts_with("10") => 15,
            s if s.starts_with("20") => 30,
            s if s.starts_with("36") => 50,
            _ => 60,
        };
        let (jemput, tujuan) = match c_rute.get_untracked().split_once('-') {
            Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
            None => (String::new(), c_rute.get_untracked()),
        };
        let mut catatan = format!("Durasi: {}. Rombongan: {}.", c_durasi.get_untracked(), c_rombongan.get_untracked());
        let pref = c_pref.get_untracked();
        if !pref.trim().is_empty() {
            catatan.push_str(&format!(" Preferensi: {pref}"));
        }
        target.set(Some(RequestTarget {
            jenis: "custom",
            item_id: String::new(),
            item_nama: "Rencana Wisata Custom".into(),
            item_info: "Ceritakan rencana wisata Anda — CS menyiapkan itinerary & penawaran terbaik.".into(),
            prefill: RequestPrefill { jemput, tujuan, jumlah_orang: jumlah, catatan, ..Default::default() },
        }));
    };

    view! {
        <div class="page page-rental">
            <section class="soft-hero">
                <span class="hero-badge">
                    <Icon name="stars" />
                    "LajuBus Holiday Charter"
                </span>
                <h1>"Sewa Bus Destinasi Wisata"</h1>
                <p>
                    "Jelajahi keindahan Nusantara bersama rombongan. Paket bus pariwisata lengkap dengan sopir hafal rute wisata & itinerary fleksibel."
                </p>
                <div class="chip-row">
                    {Kategori::ALL
                        .into_iter()
                        .map(|(k, icon, label)| {
                            view! {
                                <button
                                    type="button"
                                    class=move || if kategori.get() == k { "cat-pill active" } else { "cat-pill" }
                                    on:click=move |_| {
                                        kategori.set(k);
                                        kawasan.set(None);
                                    }
                                >
                                    <Icon name=icon />
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </section>

            <section class="section">
                <div class="section-head">
                    <h2>"Pilih Kawasan Wisata"</h2>
                    <Suspense fallback=|| ()>
                        <span class="label-caps kicker-orange">{move || format!("{} Opsi Rute", by_kategori().len())}</span>
                    </Suspense>
                </div>
                <Suspense fallback=|| ()>
                    <div class="chip-row">
                        <button
                            type="button"
                            class=move || if kawasan.get().is_none() { "region-chip active" } else { "region-chip" }
                            on:click=move |_| kawasan.set(None)
                        >
                            {move || format!("Semua ({})", by_kategori().len())}
                        </button>
                        {move || {
                            let mut regions: Vec<String> = by_kategori().into_iter().map(|p| p.kawasan).collect();
                            regions.sort();
                            regions.dedup();
                            regions
                                .into_iter()
                                .map(|r| {
                                    let r_cmp = r.clone();
                                    let r_set = r.clone();
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if kawasan.get().as_ref() == Some(&r_cmp) {
                                                    "region-chip active"
                                                } else {
                                                    "region-chip"
                                                }
                                            }
                                            on:click=move |_| kawasan.set(Some(r_set.clone()))
                                        >
                                            {r}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </Suspense>

                <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                    {move || {
                        let list = filtered();
                        if list.is_empty() {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="landscape" />
                                    <p>"Belum ada paket di kategori ini. Buat rencana wisata sendiri di bawah — gratis konsultasi!"</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            view! {
                                <div class="pkg-grid">
                                    {list
                                        .into_iter()
                                        .map(|p| view! { <PackageCard pkg=p on_book=move |t| target.set(Some(t)) /> })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                    }}
                </Suspense>
            </section>

            <section class="custom-card">
                <span class="hero-badge hero-badge-cta">
                    <Icon name="support_agent" />
                    "Custom Tour Specialist"
                </span>
                <h2>"Buat Rencana Wisata Sendiri"</h2>
                <p>
                    "Punya daftar destinasi impian bersama rombongan? Tentukan sendiri tempat tujuan, durasi, dan titik kuliner favorit — tim LajuBus menyusun itinerary & penawarannya."
                </p>
                <div class="custom-form">
                    <label class="field">
                        <span class="field-label">"Kota Asal & Destinasi Impian"</span>
                        <span class="input-wrap">
                            <Icon name="map" />
                            <input
                                type="text"
                                placeholder="Contoh: Jakarta - Dieng + Solo Heritage"
                                prop:value=move || c_rute.get()
                                on:input=move |ev| c_rute.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                    <div class="field-grid">
                        <label class="field">
                            <span class="field-label">"Durasi"</span>
                            <span class="input-wrap">
                                <Icon name="schedule" />
                                <select prop:value=move || c_durasi.get() on:change=move |ev| c_durasi.set(event_target_value(&ev))>
                                    {DURASI.into_iter().map(|d| view! { <option value=d>{d}</option> }).collect_view()}
                                </select>
                            </span>
                        </label>
                        <label class="field">
                            <span class="field-label">"Perkiraan Rombongan"</span>
                            <span class="input-wrap">
                                <Icon name="groups" />
                                <select
                                    prop:value=move || c_rombongan.get()
                                    on:change=move |ev| c_rombongan.set(event_target_value(&ev))
                                >
                                    {ROMBONGAN.into_iter().map(|d| view! { <option value=d>{d}</option> }).collect_view()}
                                </select>
                            </span>
                        </label>
                    </div>
                    <label class="field">
                        <span class="field-label">"Preferensi Tambahan (opsional)"</span>
                        <span class="input-wrap">
                            <Icon name="edit_note" />
                            <input
                                type="text"
                                placeholder="Contoh: Titik kuliner sate klathak & rest area luas"
                                prop:value=move || c_pref.get()
                                on:input=move |ev| c_pref.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                    <button type="button" class="btn btn-cta btn-block btn-lg" on:click=open_custom>
                        <Icon name="chat" />
                        "Konsultasi Gratis via WhatsApp CS Wisata"
                    </button>
                </div>
                <p class="custom-note">
                    <Icon name="bolt" />
                    "Respons cepat & garansi ketersediaan armada terverifikasi."
                </p>
            </section>

            <section class="section">
                <div class="section-head">
                    <div>
                        <span class="label-caps kicker-orange">"Standar Kenyamanan Rombongan"</span>
                        <h2>"Kenapa Sewa Bus Wisata di LajuBus?"</h2>
                        <p>"Setiap armada dipersiapkan khusus untuk perjalanan rekreasi yang aman, ceria, dan bebas repot."</p>
                    </div>
                </div>
                <div class="why-list">
                    <div class="card why-item">
                        <span class="why-icon w-blue">
                            <Icon name="explore" />
                        </span>
                        <div>
                            <strong>"Driver Paham Rute Wisata"</strong>
                            <p>"Sopir pariwisata berpengalaman di rute destinasi, akses bus ke objek wisata, dan titik istirahat."</p>
                        </div>
                    </div>
                    <div class="card why-item">
                        <span class="why-icon w-orange">
                            <Icon name="mic" />
                        </span>
                        <div>
                            <strong>"Hiburan Sepanjang Jalan"</strong>
                            <p>"Armada pariwisata dengan audio, TV, dan mic — cek detail fasilitas di tiap paket."</p>
                        </div>
                    </div>
                    <div class="card why-item">
                        <span class="why-icon w-green">
                            <Icon name="route" />
                        </span>
                        <div>
                            <strong>"Itinerary Fleksibel"</strong>
                            <p>"Rute, durasi, dan titik kuliner bisa disesuaikan dengan kebutuhan rombongan."</p>
                        </div>
                    </div>
                    <div class="card why-item">
                        <span class="why-icon w-violet">
                            <Icon name="verified_user" />
                        </span>
                        <div>
                            <strong>"Mitra PO Terverifikasi"</strong>
                            <p>"Paket & armada ditawarkan oleh mitra PO yang terdaftar dan dikelola di platform LajuBus."</p>
                        </div>
                    </div>
                </div>
            </section>

            <Suspense fallback=|| ()>
                {move || {
                    let c = cs_phone();
                    (!c.is_empty())
                        .then(|| {
                            view! {
                                <section class="help-card help-card-cta">
                                    <span class="help-icon">
                                        <Icon name="support_agent" />
                                    </span>
                                    <div>
                                        <strong>"Butuh Bantuan Cepat?"</strong>
                                        <p>"Hubungi tim charter kami untuk pertanyaan seputar paket wisata."</p>
                                    </div>
                                    <a
                                        class="btn btn-primary btn-sm"
                                        href=wa_link(&c, "Halo CS LajuBus 👋 Saya ingin tanya paket bus wisata.")
                                        target="_blank"
                                        rel="noopener"
                                    >
                                        <Icon name="chat" />
                                        "WhatsApp"
                                    </a>
                                </section>
                            }
                        })
                }}
            </Suspense>

            {move || {
                target
                    .get()
                    .map(|t| {
                        view! { <RentalRequestSheet target=t cs_phone=cs_phone() on_close=move |_| target.set(None) /> }
                    })
            }}
        </div>
    }
}

#[component]
fn PackageCard(pkg: TourPackage, #[prop(into)] on_book: Callback<RequestTarget>) -> impl IntoView {
    let p = pkg;
    let open = RwSignal::new(false);
    let target = RequestTarget {
        jenis: "paket",
        item_id: p.id.clone(),
        item_nama: p.judul.clone(),
        item_info: format!("{} · {} · oleh {}", p.durasi, p.kawasan, p.pemilik),
        prefill: RequestPrefill { tujuan: p.kawasan.clone(), jumlah_orang: p.min_pax.max(1), ..Default::default() },
    };
    let rute = p.rute.clone();
    let pemilik = p.pemilik.clone();

    view! {
        <article class="card pkg-card">
            <div class="pkg-media">
                {if p.foto_url.is_empty() {
                    view! {
                        <div class="media-fallback">
                            <Icon name="landscape" />
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <img src=p.foto_url.clone() style=crate::web::foto::style(&p.foto_url) alt=p.judul.clone() loading="lazy" /> }.into_any()
                }}
                <div class="media-shade"></div>
                {(!p.label_badge.is_empty()).then(|| view! { <span class="media-badge">{p.label_badge.clone()}</span> })}
                <div class="media-foot">
                    <span>
                        <Icon name="schedule" />
                        {p.durasi.clone()}
                    </span>
                    {(!p.label_tipe.is_empty()).then(|| view! { <span class="media-tag">{p.label_tipe.clone()}</span> })}
                </div>
            </div>
            <div class="pkg-body">
                <div>
                    <h3>{p.judul.clone()}</h3>
                    {(!p.rute.is_empty())
                        .then(|| {
                            view! {
                                <p class=move || if open.get() { "pkg-route" } else { "pkg-route clamp" }>
                                    {format!("Rute Wisata: {rute}")}
                                </p>
                            }
                        })}
                </div>
                {(!p.armada_info.is_empty())
                    .then(|| {
                        view! {
                            <div class="spec-pill">
                                <Icon name="directions_bus" />
                                {p.armada_info.clone()}
                            </div>
                        }
                    })}
                <div class="incl-row">
                    {p
                        .fasilitas
                        .iter()
                        .map(|f| {
                            view! {
                                <span class="incl">
                                    <Icon name="check_circle" />
                                    {f.clone()}
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
                {move || {
                    open.get()
                        .then(|| {
                            view! {
                                <p class="pkg-owner">
                                    <Icon name="storefront" />
                                    {format!("Diselenggarakan oleh {pemilik}")}
                                </p>
                            }
                        })
                }}
                <div class="price-split">
                    {(p.harga_pax > 0)
                        .then(|| {
                            view! {
                                <div>
                                    <span class="label-caps">"Harga per orang"</span>
                                    <p class="price">
                                        <strong>{format!("Rp {}", format_rupiah(p.harga_pax))}</strong>
                                        <small>{format!("/pax (min {} pax)", p.min_pax)}</small>
                                    </p>
                                </div>
                            }
                        })}
                    {(p.harga_charter > 0)
                        .then(|| {
                            view! {
                                <div class="price-right">
                                    <span class="label-caps">"Atau charter bus"</span>
                                    <p class="price">
                                        <strong>{format!("Rp {}", format_rupiah(p.harga_charter))}</strong>
                                        <small>"/trip"</small>
                                    </p>
                                </div>
                            }
                        })}
                </div>
                <div class="pkg-actions">
                    <button type="button" class="btn btn-soft" on:click=move |_| open.update(|v| *v = !*v)>
                        <Icon name="description" />
                        {move || if open.get() { "Tutup Detail" } else { "Detail Itinerary" }}
                    </button>
                    <button type="button" class="btn btn-cta" on:click=move |_| on_book.run(target.clone())>
                        <Icon name="event_available" />
                        "Booking Paket"
                    </button>
                </div>
            </div>
        </article>
    }
}
