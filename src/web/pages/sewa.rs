//! web/pages/sewa.rs — "/sewa" publik: kalkulator sewa + katalog bus charter.
//! "Pilih Armada" membuka form permintaan sewa (disimpan, lalu CS / mitra PO
//! menghubungi via WhatsApp). Estimasi biaya = tarif harian × jumlah hari.

use chrono::NaiveDate;
use leptos::prelude::*;

use crate::web::api::{get_cs_contact, list_charter_buses};
use crate::web::components::{format_rupiah, wa_link, Icon, RentalRequestSheet, RequestPrefill, RequestTarget};
use crate::web::models::CharterBus;

#[derive(Clone, Copy, PartialEq)]
enum Ukuran {
    Semua,
    Big,
    Medium,
    Mini,
}

impl Ukuran {
    const ALL: [(Ukuran, &'static str); 4] = [
        (Ukuran::Semua, "Semua"),
        (Ukuran::Big, "Big Bus (45–59 Seat)"),
        (Ukuran::Medium, "Medium Bus (21–44 Seat)"),
        (Ukuran::Mini, "Mini / Elf (≤ 20 Seat)"),
    ];

    fn cocok(self, kapasitas: i32) -> bool {
        match self {
            Ukuran::Semua => true,
            Ukuran::Big => kapasitas >= 45,
            Ukuran::Medium => (21..45).contains(&kapasitas),
            Ukuran::Mini => kapasitas <= 20,
        }
    }
}

const TIPE: [&str; 3] = ["Pulang-Pergi", "Menginap", "Sekali Jalan"];

/// Jumlah hari sewa (inklusif). Tanpa tanggal pulang / sekali jalan = 1 hari.
fn jumlah_hari(berangkat: &str, pulang: &str) -> i64 {
    let a = NaiveDate::parse_from_str(berangkat, "%Y-%m-%d");
    let b = NaiveDate::parse_from_str(pulang, "%Y-%m-%d");
    match (a, b) {
        (Ok(a), Ok(b)) if b >= a => (b - a).num_days() + 1,
        _ => 1,
    }
}

#[component]
pub fn SewaPage() -> impl IntoView {
    let buses = Resource::new(|| (), |_| list_charter_buses());
    let cs = Resource::new(|| (), |_| get_cs_contact());
    let cs_phone = move || cs.get().and_then(|r| r.ok()).unwrap_or_default();

    // Kalkulator.
    let tipe = RwSignal::new(TIPE[0].to_string());
    let jemput = RwSignal::new(String::new());
    let tujuan = RwSignal::new(String::new());
    let berangkat = RwSignal::new(String::new());
    let pulang = RwSignal::new(String::new());
    let pax = RwSignal::new(45i32);
    // Rekomendasi aktif setelah tombol "Cari Rekomendasi" ditekan.
    let rekom = RwSignal::new(None::<i32>);

    let ukuran = RwSignal::new(Ukuran::Semua);
    let target = RwSignal::new(None::<RequestTarget>);

    let hari = move || {
        if tipe.get() == "Sekali Jalan" {
            1
        } else {
            jumlah_hari(&berangkat.get(), &pulang.get())
        }
    };
    let all = move || buses.get().and_then(|r| r.ok()).unwrap_or_default();
    let filtered = move || {
        let u = ukuran.get();
        let min = rekom.get();
        let mut list: Vec<CharterBus> = all()
            .into_iter()
            .filter(|b| u.cocok(b.kapasitas))
            .filter(|b| min.is_none_or(|m| b.kapasitas >= m))
            .collect();
        if rekom.get().is_some() {
            // Paling pas duluan: kapasitas terkecil yang masih muat, lalu termurah.
            list.sort_by_key(|b| (b.kapasitas, b.harga_harian));
        }
        list
    };

    let swap = move |_| {
        let a = jemput.get_untracked();
        jemput.set(tujuan.get_untracked());
        tujuan.set(a);
    };

    let pick_bus = move |b: CharterBus| {
        let t = tipe.get_untracked();
        target.set(Some(RequestTarget {
            jenis: "charter",
            item_id: b.id.clone(),
            item_nama: b.nama.clone(),
            item_info: format!(
                "{} seat · Rp {}/hari{}",
                b.kapasitas,
                format_rupiah(b.harga_harian),
                if b.catatan_harga.is_empty() { String::new() } else { format!(" ({})", b.catatan_harga) }
            ),
            prefill: RequestPrefill {
                jemput: jemput.get_untracked(),
                tujuan: tujuan.get_untracked(),
                tgl_berangkat: berangkat.get_untracked(),
                tgl_pulang: if t == "Sekali Jalan" { String::new() } else { pulang.get_untracked() },
                tipe_perjalanan: t,
                jumlah_orang: pax.get_untracked(),
                catatan: String::new(),
            },
        }));
    };

    let open_khusus = move |nama: &'static str| {
        target.set(Some(RequestTarget {
            jenis: "custom",
            item_id: String::new(),
            item_nama: nama.to_string(),
            item_info: "Ceritakan kebutuhan rombongan — CS menyiapkan rekomendasi armada & penawaran.".into(),
            prefill: RequestPrefill {
                jemput: jemput.get_untracked(),
                tujuan: tujuan.get_untracked(),
                tgl_berangkat: berangkat.get_untracked(),
                tgl_pulang: pulang.get_untracked(),
                tipe_perjalanan: tipe.get_untracked(),
                jumlah_orang: pax.get_untracked(),
                catatan: format!("Keperluan: {nama}."),
            },
        }));
    };

    let khusus = move |icon: &'static str, cls: &'static str, title: &'static str, desc: &'static str, cta: &'static str| {
        view! {
            <button type="button" class="card khusus-tile" on:click=move |_| open_khusus(title)>
                <span class=format!("why-icon {cls}")>
                    <Icon name=icon />
                </span>
                <strong>{title}</strong>
                <p>{desc}</p>
                <span class="tile-link">
                    {cta}
                    <Icon name="chevron_right" />
                </span>
            </button>
        }
    };

    view! {
        <div class="page page-rental">
            <section class="charter-hero">
                <span class="hero-badge hero-badge-glass">
                    <Icon name="corporate_fare" />
                    "LajuBus Charter Solution"
                </span>
                <h1>"Sewa Bus Rombongan & Korporat"</h1>
                <p>
                    "Armada mitra PO terverifikasi, sopir profesional berpengalaman, dan penawaran transparan untuk perjalanan kantor, sekolah, hingga keluarga besar."
                </p>
                <div class="hero-trust">
                    <span>
                        <Icon name="verified" />
                        "Armada Terverifikasi"
                    </span>
                    <span>
                        <Icon name="local_gas_station" />
                        "Opsi Inklusif BBM + Driver"
                    </span>
                    <span>
                        <Icon name="receipt_long" />
                        "Penawaran Tertulis"
                    </span>
                </div>
            </section>

            <section class="card calc-card">
                <div class="calc-head">
                    <h2>
                        <Icon name="calculate" />
                        "Kalkulator Sewa Bus"
                    </h2>
                    <span class="label-caps kicker-orange">"Hitung Cepat"</span>
                </div>
                <div class="segmented seg-3">
                    {TIPE
                        .into_iter()
                        .map(|t| {
                            view! {
                                <button
                                    type="button"
                                    class=move || if tipe.get() == t { "seg active" } else { "seg" }
                                    on:click=move |_| tipe.set(t.to_string())
                                >
                                    {t}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="route-stack">
                    <label class="route-field">
                        <span class="route-icon">
                            <Icon name="trip_origin" />
                        </span>
                        <span class="route-text">
                            <span class="label-caps">"Titik Jemput Penumpang"</span>
                            <input
                                type="text"
                                placeholder="Mis. Jakarta Selatan (Jabodetabek)"
                                prop:value=move || jemput.get()
                                on:input=move |ev| jemput.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                    <button type="button" class="swap-btn" title="Tukar" on:click=swap>
                        <Icon name="swap_vert" />
                    </button>
                    <label class="route-field">
                        <span class="route-icon route-icon-dest">
                            <Icon name="location_on" />
                        </span>
                        <span class="route-text">
                            <span class="label-caps">"Kota / Area Tujuan"</span>
                            <input
                                type="text"
                                placeholder="Mis. Bandung, Jawa Barat"
                                prop:value=move || tujuan.get()
                                on:input=move |ev| tujuan.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                </div>
                <div class="field-grid">
                    <label class="field">
                        <span class="field-label">"Tanggal Berangkat"</span>
                        <span class="input-wrap">
                            <Icon name="calendar_today" />
                            <input
                                type="date"
                                prop:value=move || berangkat.get()
                                on:input=move |ev| berangkat.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                    <label class=move || if tipe.get() == "Sekali Jalan" { "field is-disabled" } else { "field" }>
                        <span class="field-label">{move || format!("Tanggal Pulang ({}H)", hari())}</span>
                        <span class="input-wrap">
                            <Icon name="event" />
                            <input
                                type="date"
                                disabled=move || tipe.get() == "Sekali Jalan"
                                prop:value=move || pulang.get()
                                on:input=move |ev| pulang.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                </div>
                <div class="pax-box">
                    <div class="pax-head">
                        <span class="field-label">"Estimasi Jumlah Penumpang"</span>
                        <strong>{move || format!("{} Orang", pax.get())}</strong>
                    </div>
                    <input
                        type="range"
                        min="5"
                        max="60"
                        step="1"
                        class="range"
                        prop:value=move || pax.get().to_string()
                        on:input=move |ev| pax.set(event_target_value(&ev).parse().unwrap_or(45))
                    />
                    <div class="pax-scale">
                        <span>"5"</span>
                        <span>"60+"</span>
                    </div>
                </div>
                <a href="#katalog" class="btn btn-cta btn-block btn-lg" on:click=move |_| rekom.set(Some(pax.get_untracked()))>
                    <Icon name="search" />
                    "Cari Rekomendasi Armada Bus"
                </a>
                <p class="calc-note">
                    <Icon name="receipt_long" />
                    "Penawaran resmi tersedia untuk perusahaan & instansi."
                </p>
            </section>

            <section id="katalog" class="section">
                <div class="section-head">
                    <h2>"Katalog Bus Charter"</h2>
                    <Suspense fallback=|| ()>
                        <span class="label-caps kicker-orange">{move || format!("{} Bus Tersedia", filtered().len())}</span>
                    </Suspense>
                </div>
                <div class="chip-row">
                    {Ukuran::ALL
                        .into_iter()
                        .map(|(u, label)| {
                            view! {
                                <button
                                    type="button"
                                    class=move || if ukuran.get() == u { "region-chip active" } else { "region-chip" }
                                    on:click=move |_| ukuran.set(u)
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                {move || {
                    rekom
                        .get()
                        .map(|m| {
                            view! {
                                <div class="rekom-bar">
                                    <Icon name="auto_awesome" />
                                    <span>
                                        {format!("Rekomendasi untuk {m} orang · estimasi {} hari", hari())}
                                    </span>
                                    <button type="button" class="link-btn" on:click=move |_| rekom.set(None)>
                                        "Tampilkan semua"
                                    </button>
                                </div>
                            }
                        })
                }}
                <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                    {move || {
                        let list = filtered();
                        let days = hari();
                        let est = rekom.get().is_some();
                        if list.is_empty() {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="airport_shuttle" />
                                    <p>"Belum ada armada yang cocok. Ajukan kebutuhan khusus di bawah — CS akan carikan armadanya."</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            view! {
                                <div class="pkg-grid">
                                    {list
                                        .into_iter()
                                        .map(|b| {
                                            view! {
                                                <CharterCard
                                                    bus=b
                                                    hari=if est { Some(days) } else { None }
                                                    on_pick=move |b| pick_bus(b)
                                                />
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                    }}
                </Suspense>
            </section>

            <section class="section">
                <div class="section-head">
                    <div>
                        <span class="label-caps kicker-orange">"Solusi Perjalanan Terintegrasi"</span>
                        <h2>"Paket Keperluan Khusus"</h2>
                    </div>
                </div>
                <div class="khusus-grid">
                    {khusus(
                        "business_center",
                        "w-blue",
                        "Corporate Outing",
                        "Transportasi gathering, outbound, dan kunjungan kerja karyawan.",
                        "Tanya Paket B2B",
                    )}
                    {khusus(
                        "school",
                        "w-orange",
                        "Study Tour Sekolah",
                        "Armada untuk karya wisata, lengkap dengan koordinasi jadwal rombongan.",
                        "Pesan Study Tour",
                    )}
                    {khusus(
                        "mosque",
                        "w-green",
                        "Rombongan Ziarah",
                        "Rute ziarah wali & religi dengan jadwal istirahat yang nyaman.",
                        "Rute Religi",
                    )}
                    {khusus(
                        "flight",
                        "w-violet",
                        "Shuttle Bandara",
                        "Antar-jemput bandara untuk tamu, event, dan rombongan.",
                        "Layanan Shuttle",
                    )}
                </div>
            </section>

            <section class="card standard-card">
                <div class="standard-head">
                    <span class="why-icon w-blue">
                        <Icon name="shield" />
                    </span>
                    <div>
                        <strong>"Standar & Garansi LajuBus"</strong>
                        <p>"Komitmen keselamatan setiap perjalanan rombongan"</p>
                    </div>
                </div>
                <div class="standard-row">
                    <Icon name="verified" filled=true />
                    <div>
                        <strong>"Mitra PO Terdaftar & Terverifikasi"</strong>
                        <p>"Armada di katalog dikelola mitra PO yang terdaftar dan dipantau admin LajuBus."</p>
                    </div>
                </div>
                <div class="standard-row">
                    <Icon name="badge" filled=true />
                    <div>
                        <strong>"Sopir Berpengalaman"</strong>
                        <p>"Kru dipersiapkan untuk perjalanan jarak jauh dan rute rombongan."</p>
                    </div>
                </div>
                <div class="standard-row">
                    <Icon name="handshake" filled=true />
                    <div>
                        <strong>"Penawaran Jelas Sebelum Berangkat"</strong>
                        <p>"Harga final, fasilitas, dan titik jemput dikonfirmasi tertulis lewat WhatsApp."</p>
                    </div>
                </div>
            </section>

            <Suspense fallback=|| ()>
                {move || {
                    let c = cs_phone();
                    (!c.is_empty())
                        .then(|| {
                            view! {
                                <section class="hotline">
                                    <div>
                                        <span class="label-caps">"Charter Hotline"</span>
                                        <strong>{format!("+{c}")}</strong>
                                        <small>"Konsultasi sewa bus via WhatsApp"</small>
                                    </div>
                                    <a
                                        class="btn btn-cta"
                                        href=wa_link(&c, "Halo CS LajuBus 👋 Saya ingin konsultasi sewa bus rombongan.")
                                        target="_blank"
                                        rel="noopener"
                                    >
                                        <Icon name="chat" />
                                        "Hubungi"
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
fn CharterCard(bus: CharterBus, hari: Option<i64>, #[prop(into)] on_pick: Callback<CharterBus>) -> impl IntoView {
    let b = bus;
    let pick = b.clone();
    let seat = if b.konfigurasi.is_empty() {
        format!("{} Seat", b.kapasitas)
    } else {
        format!("{} Seat ({})", b.kapasitas, b.konfigurasi)
    };

    view! {
        <article class="card pkg-card">
            <div class="pkg-media">
                {if b.foto_url.is_empty() {
                    view! {
                        <div class="media-fallback">
                            <Icon name="directions_bus" />
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <img src=b.foto_url.clone() alt=b.nama.clone() loading="lazy" /> }.into_any()
                }}
                <span class="media-badge media-badge-dark">
                    <Icon name="event_seat" />
                    {seat}
                </span>
                {(!b.kelas.is_empty()).then(|| view! { <span class="media-badge media-badge-right">{b.kelas.clone()}</span> })}
            </div>
            <div class="pkg-body">
                <div class="charter-title">
                    <h3>{b.nama.clone()}</h3>
                    {(!b.tipe_bus.is_empty()).then(|| view! { <span class="type-chip">{b.tipe_bus.clone()}</span> })}
                </div>
                {(!b.deskripsi.is_empty()).then(|| view! { <p class="pkg-route">{b.deskripsi.clone()}</p> })}
                <div class="fac-grid">
                    {b
                        .fasilitas
                        .iter()
                        .map(|f| {
                            view! {
                                <span>
                                    <Icon name="check_circle" />
                                    {f.clone()}
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="charter-foot">
                    <div>
                        <span class="label-caps">"Tarif sewa harian"</span>
                        <p class="price">
                            <strong>{format!("Rp {}", format_rupiah(b.harga_harian))}</strong>
                            <small>"/hari"</small>
                        </p>
                        {(!b.catatan_harga.is_empty()).then(|| view! { <small class="price-note">{b.catatan_harga.clone()}</small> })}
                        {hari
                            .filter(|d| *d > 1)
                            .map(|d| {
                                view! {
                                    <small class="price-est">
                                        {format!("Estimasi {d} hari: Rp {}", format_rupiah(b.harga_harian * d))}
                                    </small>
                                }
                            })}
                    </div>
                    <button type="button" class="btn btn-cta" on:click=move |_| on_pick.run(pick.clone())>
                        "Pilih Armada"
                    </button>
                </div>
                <small class="owner-line">
                    <Icon name="storefront" />
                    {b.pemilik.clone()}
                </small>
            </div>
        </article>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hitung_hari_sewa() {
        assert_eq!(jumlah_hari("2026-10-18", "2026-10-20"), 3);
        assert_eq!(jumlah_hari("2026-10-18", ""), 1);
        assert_eq!(jumlah_hari("2026-10-18", "2026-10-17"), 1);
    }

    #[test]
    fn ukuran_bus() {
        assert!(Ukuran::Big.cocok(50) && !Ukuran::Big.cocok(35));
        assert!(Ukuran::Medium.cocok(31) && Ukuran::Mini.cocok(15));
    }
}
