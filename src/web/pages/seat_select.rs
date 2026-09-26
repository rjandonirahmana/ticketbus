//! web/pages/seat_select.rs — "/pesan/:id": pilih kursi bernomor + data
//! pemesan, lalu checkout. Denah dibangkitkan `web::seats::layout` — fungsi
//! yang SAMA dipakai server untuk memvalidasi kode kursi saat order dibuat.

use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::web::api::{create_order, get_seat_map};
use crate::web::app::SessionResource;
use crate::web::components::{
    clean_error, format_rupiah, format_tanggal, spawn_client, today_wib, Icon, PageHead, RouteTimeline,
};
use crate::web::models::{NewOrder, Schedule};
use crate::web::seats::{layout, sisi, Seat};

const MAKS_KURSI: usize = 10;

fn kelas_label(konfigurasi: &str) -> &'static str {
    match konfigurasi {
        "1-1" => "Sleeper 1-1",
        "2-1" => "VIP 2-1",
        _ => "Eksekutif 2-2",
    }
}

/// "Rp 280rb" untuk tile kursi yang sempit.
fn harga_ringkas(n: i64) -> String {
    if n >= 1_000_000 {
        format!("Rp {}jt", format!("{:.1}", n as f64 / 1e6).trim_end_matches(".0").replace('.', ","))
    } else if n >= 1_000 {
        format!("Rp {}rb", n / 1_000)
    } else {
        format!("Rp {n}")
    }
}

fn posisi(seat: &Seat, kiri: u8, dua_dek: bool) -> String {
    let dek = if !dua_dek {
        String::new()
    } else if seat.dek == 1 {
        "Dek Bawah · ".into()
    } else {
        "Dek Atas · ".into()
    };
    let sisi = if seat.kolom < kiri { "Kiri" } else { "Kanan" };
    let tempat = if seat.jendela { "Jendela" } else { "Lorong" };
    format!("{dek}Sisi {tempat} {sisi}")
}

#[component]
pub fn SeatSelectPage() -> impl IntoView {
    let params = use_params_map();
    let schedule_id = move || params.get().get("id").unwrap_or_default();
    let map = Resource::new(schedule_id, |id| async move { get_seat_map(id).await });

    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let user = move || session.get().and_then(|r| r.ok()).flatten();

    let dek = RwSignal::new(1u8);
    let dipilih = RwSignal::new(Vec::<String>::new());
    let nama = RwSignal::new(String::new());
    let telp = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    // Prefill pemesan dari sesi, tanpa menimpa yang sudah diketik.
    Effect::new(move |_| {
        if let Some(Ok(Some(u))) = session.get() {
            if nama.get_untracked().is_empty() {
                nama.set(u.name.clone());
            }
            if telp.get_untracked().is_empty() {
                telp.set(u.phone.clone());
            }
        }
    });

    let toggle = move |kode: String, sisa: i32| {
        error.set(String::new());
        dipilih.update(|v| {
            if let Some(i) = v.iter().position(|k| *k == kode) {
                v.remove(i);
            } else if v.len() >= MAKS_KURSI {
                error.set(format!("Maksimal {MAKS_KURSI} kursi per pemesanan"));
            } else if v.len() as i32 >= sisa {
                error.set(format!("Sisa kursi jadwal ini {sisa}"));
            } else {
                v.push(kode);
                v.sort();
            }
        });
    };

    let navigate = use_navigate();
    let confirm = move |_| {
        let kursi = dipilih.get_untracked();
        if kursi.is_empty() {
            error.set("Pilih minimal 1 kursi".into());
            return;
        }
        let input = NewOrder {
            schedule_id: schedule_id(),
            jumlah_tiket: kursi.len() as i32,
            kursi: kursi.join(","),
            nama_pemesan: nama.get_untracked(),
            telp_pemesan: telp.get_untracked(),
        };
        if input.nama_pemesan.trim().is_empty() || input.telp_pemesan.trim().is_empty() {
            error.set("Nama dan nomor WhatsApp pemesan wajib diisi".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        let navigate = navigate.clone();
        spawn_client(async move {
            match create_order(input).await {
                Ok(order) => navigate(&format!("/orders/{}", order.id), Default::default()),
                Err(e) => {
                    // Kemungkinan besar kursi keburu diambil orang: muat ulang
                    // denah & lepaskan pilihan yang kini sudah terisi.
                    error.set(clean_error(&e.to_string()));
                    map.refetch();
                }
            }
            busy.set(false);
        });
    };

    view! {
        <div class="page page-narrow page-seat">
            <PageHead title="Pilih Kursi & Penumpang" back="/" />
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    match map.get().and_then(|r| r.ok()).flatten() {
                        None => {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="event_busy" />
                                    <p>"Jadwal tidak ditemukan."</p>
                                    <a href="/" class="btn btn-primary">
                                        "Cari Jadwal Lain"
                                    </a>
                                </div>
                            }
                                .into_any()
                        }
                        Some(m) => {
                            let s = m.schedule.clone();
                            let terisi = m.terisi.clone();
                            // Kursi pilihan yang ternyata sudah terisi (denah dimuat ulang) dilepas.
                            dipilih.update_untracked(|v| v.retain(|k| !terisi.contains(k)));
                            let lewat = s.tanggal < today_wib().format("%Y-%m-%d").to_string();
                            view! {
                                <TripHeader schedule=s.clone() />
                                <SeatMapView
                                    schedule=s.clone()
                                    terisi=terisi
                                    dek=dek
                                    dipilih=dipilih
                                    on_toggle=move |k: String| toggle(k, s.sisa_kursi())
                                    disabled=lewat
                                />
                                <SelectionDetail schedule=m.schedule.clone() dipilih=dipilih />
                                {(!lewat)
                                    .then(|| {
                                        view! {
                                            <section class="card pw-card">
                                                <div class="pw-head">
                                                    <span>"Data Pemesan"</span>
                                                    <span class="pill pill-primary">"E-tiket via WhatsApp"</span>
                                                </div>
                                                <label class="field">
                                                    <span class="field-label">"Nama Lengkap"</span>
                                                    <span class="input-wrap">
                                                        <Icon name="person" />
                                                        <input
                                                            type="text"
                                                            placeholder="Nama sesuai KTP"
                                                            prop:value=move || nama.get()
                                                            on:input=move |ev| nama.set(event_target_value(&ev))
                                                        />
                                                    </span>
                                                </label>
                                                <label class="field">
                                                    <span class="field-label">"No. WhatsApp"</span>
                                                    <span class="input-wrap">
                                                        <Icon name="chat" />
                                                        <input
                                                            type="tel"
                                                            placeholder="08xx"
                                                            prop:value=move || telp.get()
                                                            on:input=move |ev| telp.set(event_target_value(&ev))
                                                        />
                                                    </span>
                                                </label>
                                            </section>
                                        }
                                    })}
                                {(!m.schedule.kursi_wanita.is_empty())
                                    .then(|| {
                                        view! {
                                            <section class="care-note">
                                                <Icon name="shield" />
                                                <div>
                                                    <strong>"Komitmen Keamanan & Perlindungan Wanita"</strong>
                                                    <p>
                                                        "Kursi bertanda merah muda diprioritaskan bagi penumpang wanita demi keamanan dan kenyamanan selama perjalanan."
                                                    </p>
                                                </div>
                                            </section>
                                        }
                                    })}
                                {lewat
                                    .then(|| {
                                        view! {
                                            <p class="alert alert-warn">
                                                <Icon name="history" />
                                                "Jadwal ini sudah lewat — kursi tak bisa dipesan lagi."
                                            </p>
                                        }
                                    })}
                            }
                                .into_any()
                        }
                    }
                }}
            </Suspense>

            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}

            <Suspense fallback=|| ()>
                {move || {
                    let m = map.get().and_then(|r| r.ok()).flatten()?;
                    let harga = m.schedule.harga;
                    let lewat = m.schedule.tanggal < today_wib().format("%Y-%m-%d").to_string();
                    let u = user();
                    Some(view! {
                        <div class="checkout-bar">
                            <div class="checkout-total">
                                <span class="label-caps">
                                    {move || {
                                        let v = dipilih.get();
                                        if v.is_empty() {
                                            "Belum ada kursi".to_string()
                                        } else {
                                            format!("Total ({} kursi) · {}", v.len(), v.join(", "))
                                        }
                                    }}
                                </span>
                                <strong class="price-lg">
                                    {move || format!("Rp {}", format_rupiah(harga * dipilih.get().len() as i64))}
                                </strong>
                            </div>
                            {match u {
                                None => {
                                    view! {
                                        <a href="/login" class="btn btn-cta">
                                            "Masuk untuk Pesan"
                                            <Icon name="login" />
                                        </a>
                                    }
                                        .into_any()
                                }
                                Some(u) if u.role != "buyer" => {
                                    view! {
                                        <button type="button" class="btn btn-soft" disabled=true>
                                            "Khusus akun penumpang"
                                        </button>
                                    }
                                        .into_any()
                                }
                                Some(_) => {
                                    let confirm = confirm.clone();
                                    view! {
                                        <button
                                            type="button"
                                            class="btn btn-cta"
                                            on:click=confirm
                                            disabled=move || lewat || busy.get() || dipilih.get().is_empty()
                                        >
                                            {move || if busy.get() { "Memproses…" } else { "Konfirmasi Kursi" }}
                                            <Icon name="arrow_forward" />
                                        </button>
                                    }
                                        .into_any()
                                }
                            }}
                        </div>
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn TripHeader(schedule: Schedule) -> impl IntoView {
    let s = schedule;
    let sisa = s.sisa_kursi();
    view! {
        <section class="card trip-card seat-trip">
            <header class="trip-head">
                <span class="po-logo small">
                    <Icon name="directions_bus" />
                </span>
                <div class="trip-op">
                    <h3>
                        {s.armada_name.clone()}
                        <Icon name="verified" filled=true class="verified" />
                    </h3>
                    <p>{format!("Kelas {} · {} kursi", kelas_label(&s.konfigurasi), s.kapasitas)}</p>
                </div>
                <span class="class-badge">{kelas_label(&s.konfigurasi)}</span>
            </header>
            <RouteTimeline jam=s.jam.clone() from=s.lokasi_jemput.clone() to=s.tujuan.clone() mid="Langsung" />
            <div class="seat-trip-meta">
                <span>
                    <Icon name="calendar_today" />
                    {format_tanggal(&s.tanggal)}
                </span>
                <span class=if sisa <= 5 { "sisa low" } else { "sisa" }>
                    <Icon name="airline_seat_recline_normal" />
                    {format!("Sisa {sisa} kursi")}
                </span>
            </div>
            {(!s.catatan.is_empty() || s.dua_dek)
                .then(|| {
                    view! {
                        <div class="facility-row">
                            {s.dua_dek
                                .then(|| {
                                    view! {
                                        <span class="facility">
                                            <Icon name="stacks" />
                                            "Double Decker"
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
                    }
                })}
        </section>
    }
}

#[component]
fn SeatMapView(
    schedule: Schedule,
    terisi: Vec<String>,
    dek: RwSignal<u8>,
    dipilih: RwSignal<Vec<String>>,
    #[prop(into)] on_toggle: Callback<String>,
    disabled: bool,
) -> impl IntoView {
    let s = schedule;
    let denah = layout(s.kapasitas, &s.konfigurasi, s.dua_dek);
    let (kiri, kanan) = sisi(&s.konfigurasi);
    let lebar = (kiri + kanan) as usize;
    let harga = harga_ringkas(s.harga);
    let wanita = StoredValue::new(s.kursi_wanita.clone());
    let terisi = StoredValue::new(terisi);
    let denah = StoredValue::new(denah);
    let dua_dek = s.dua_dek;
    // Kolom grid: kursi kiri, lorong, kursi kanan.
    let grid = format!(
        "grid-template-columns: repeat({kiri}, minmax(0, 1fr)) 28px repeat({kanan}, minmax(0, 1fr))"
    );

    let dek_tab = move |n: u8, label: &'static str, icon: &'static str| {
        view! {
            <button
                type="button"
                class=move || if dek.get() == n { "seg active" } else { "seg" }
                on:click=move |_| dek.set(n)
            >
                <Icon name=icon />
                {label}
                {move || {
                    let ada = denah.with_value(|d| {
                        dipilih.get().iter().any(|k| d.iter().any(|x| &x.kode == k && x.dek == n))
                    });
                    ada.then(|| view! { <i class="seg-dot"></i> })
                }}
            </button>
        }
    };

    view! {
        {dua_dek
            .then(|| {
                view! {
                    <nav class="segmented dek-tabs">
                        {dek_tab(1, "Lantai Bawah (Dek 1)", "layers")}
                        {dek_tab(2, "Lantai Atas (Dek 2)", "upload")}
                    </nav>
                }
            })}
        <div class="card seat-legend">
            <span>
                <i class="lg-seat free"></i>
                "Tersedia"
            </span>
            <span>
                <i class="lg-seat picked"></i>
                "Dipilih"
            </span>
            <span>
                <i class="lg-seat taken"></i>
                "Terisi"
            </span>
            <span>
                <i class="lg-seat women"></i>
                "Khusus Wanita"
            </span>
        </div>
        <section class="card bus-map">
            <div class="bus-front">
                <span class="door">
                    <Icon name="door_front" />
                    <span>
                        <strong>"Pintu Depan"</strong>
                        <small>"Akses penumpang"</small>
                    </span>
                </span>
                <span class="cockpit">
                    <span>
                        <strong>"Kokpit"</strong>
                        <small>"Pengemudi"</small>
                    </span>
                    <Icon name="search_hands_free" />
                </span>
            </div>
            <div class="windshield">
                <span>{move || if dua_dek && dek.get() == 2 { "Kaca Depan Dek Atas" } else { "Kaca Depan Bus" }}</span>
            </div>
            <div class="seat-grid" style=grid>
                {move || {
                    let aktif = if dua_dek { dek.get() } else { 1 };
                    let rows: Vec<Vec<Seat>> = denah.with_value(|d| {
                        let mut out: Vec<Vec<Seat>> = Vec::new();
                        for seat in d.iter().filter(|x| x.dek == aktif) {
                            match out.last_mut() {
                                Some(r) if r[0].baris == seat.baris => r.push(seat.clone()),
                                _ => out.push(vec![seat.clone()]),
                            }
                        }
                        out
                    });
                    rows.into_iter()
                        .enumerate()
                        .map(|(ri, row)| {
                            let harga = harga.clone();
                            (0..lebar)
                                .map(move |col| {
                                    let aisle = (col == kiri as usize)
                                        .then(|| {
                                            view! {
                                                <span class="aisle">
                                                    {if ri == 0 { "↑ Lorong" } else { "·" }}
                                                </span>
                                            }
                                        });
                                    let seat = row.iter().find(|x| x.kolom as usize == col).cloned();
                                    let tile = match seat {
                                        None => view! { <span class="seat-gap"></span> }.into_any(),
                                        Some(seat) => {
                                            let kode = seat.kode.clone();
                                            let is_taken = terisi.with_value(|t| t.contains(&kode));
                                            let is_women = wanita.with_value(|w| w.contains(&kode));
                                            let k_cls = kode.clone();
                                            let k_click = kode.clone();
                                            let judul = if is_taken { "Terisi".to_string() } else { kode.clone() };
                                            let harga = harga.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class=move || {
                                                        let picked = dipilih.get().contains(&k_cls);
                                                        let mut c = String::from("seat");
                                                        if is_taken {
                                                            c.push_str(" taken");
                                                        } else if picked {
                                                            c.push_str(" picked");
                                                        } else if is_women {
                                                            c.push_str(" women");
                                                        }
                                                        c
                                                    }
                                                    disabled=is_taken || disabled
                                                    title=judul
                                                    on:click=move |_| on_toggle.run(k_click.clone())
                                                >
                                                    {move || {
                                                        dipilih
                                                            .get()
                                                            .contains(&kode)
                                                            .then(|| view! { <span class="seat-tag">"Kursi Anda"</span> })
                                                    }}
                                                    <span class="seat-code">{seat.kode.clone()}</span>
                                                    <span class="seat-kind">
                                                        {if is_taken {
                                                            view! { <Icon name="person" /> }.into_any()
                                                        } else if is_women {
                                                            view! { <Icon name="woman" /> }.into_any()
                                                        } else if seat.jendela {
                                                            view! { <Icon name="window" /> }.into_any()
                                                        } else {
                                                            view! { <Icon name="event_seat" /> }.into_any()
                                                        }}
                                                        {if is_taken {
                                                            "Terisi"
                                                        } else if is_women {
                                                            "Wanita"
                                                        } else if seat.jendela {
                                                            "Jendela"
                                                        } else {
                                                            "Lorong"
                                                        }}
                                                    </span>
                                                    <span class="seat-price">
                                                        {if is_taken { "Tidak tersedia".to_string() } else { harga }}
                                                    </span>
                                                </button>
                                            }
                                                .into_any()
                                        }
                                    };
                                    view! {
                                        {aisle}
                                        {tile}
                                    }
                                })
                                .collect_view()
                        })
                        .collect_view()
                }}
            </div>
            <div class="bus-rear">
                <span>
                    <Icon name="emergency" />
                    "Pintu darurat belakang"
                </span>
                {dua_dek
                    .then(|| {
                        view! {
                            <span>
                                <Icon name="stairs" />
                                "Tangga ke dek atas"
                            </span>
                        }
                    })}
            </div>
        </section>
    }
}

#[component]
fn SelectionDetail(schedule: Schedule, dipilih: RwSignal<Vec<String>>) -> impl IntoView {
    let s = schedule;
    let denah = StoredValue::new(layout(s.kapasitas, &s.konfigurasi, s.dua_dek));
    let (kiri, _) = sisi(&s.konfigurasi);
    let wanita = StoredValue::new(s.kursi_wanita.clone());
    let harga = s.harga;
    let kelas = kelas_label(&s.konfigurasi);
    let dua_dek = s.dua_dek;
    view! {
        {move || {
            let v = dipilih.get();
            if v.is_empty() {
                return view! {
                    <p class="note-card">
                        <Icon name="touch_app" />
                        "Ketuk kursi pada denah untuk memilih. Anda bisa memilih hingga 10 kursi sekaligus."
                    </p>
                }
                    .into_any();
            }
            v.into_iter()
                .map(|kode| {
                    let seat = denah.with_value(|d| d.iter().find(|x| x.kode == kode).cloned());
                    let pos = seat.as_ref().map(|x| posisi(x, kiri, dua_dek)).unwrap_or_default();
                    let w = wanita.with_value(|w| w.contains(&kode));
                    view! {
                        <section class="card cabin-card">
                            <span class="cabin-icon">
                                <Icon name="airline_seat_flat" />
                            </span>
                            <div class="cabin-main">
                                <h3>
                                    {format!("Kursi {kode}")}
                                    <span class="class-badge">{kelas}</span>
                                </h3>
                                <p>{pos}</p>
                                {w
                                    .then(|| {
                                        view! {
                                            <p class="women-note">
                                                <Icon name="woman" />
                                                "Diprioritaskan untuk penumpang wanita"
                                            </p>
                                        }
                                    })}
                            </div>
                            <div class="cabin-price">
                                <span class="label-caps">"Tarif Tiket"</span>
                                <strong>{format!("Rp {}", format_rupiah(harga))}</strong>
                            </div>
                        </section>
                    }
                })
                .collect_view()
                .into_any()
        }}
    }
}
