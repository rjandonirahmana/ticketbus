//! web/pages/browse.rs — "/" publik: pencarian jadwal, kalender jadwal harian,
//! dan beli tiket. Siapa saja boleh lihat; beli tiket perlu login sebagai buyer.

use chrono::Datelike;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::web::api::{create_order, list_schedules_month, list_schedules_upcoming};
use crate::web::app::SessionResource;
use crate::web::components::{
    clean_error, format_rupiah, format_tanggal, format_tanggal_panjang, min_fare, spawn_client, tanggal_parts,
    today_wib, week_start, FareCalendar, Icon, OpLogo, RouteTimeline, TripCard,
};
use crate::web::models::{NewOrder, Schedule};

/// Filter waktu keberangkatan di bawah kalender.
#[derive(Clone, Copy, PartialEq)]
enum Waktu {
    Semua,
    Dini,
    Pagi,
    Siang,
    Malam,
}

impl Waktu {
    const ALL: [Waktu; 5] = [Waktu::Semua, Waktu::Pagi, Waktu::Siang, Waktu::Malam, Waktu::Dini];

    fn label(self) -> &'static str {
        match self {
            Waktu::Semua => "Semua Waktu",
            Waktu::Dini => "Dini Hari (00:00 - 06:00)",
            Waktu::Pagi => "Pagi (06:00 - 12:00)",
            Waktu::Siang => "Siang-Sore (12:00 - 18:00)",
            Waktu::Malam => "Malam (18:00 - 24:00)",
        }
    }

    /// Jam bebas-teks ("19:30", "7.00") → cocok dengan rentang ini? Jam yang
    /// tak terbaca hanya muncul di "Semua Waktu".
    fn cocok(self, jam: &str) -> bool {
        if self == Waktu::Semua {
            return true;
        }
        let jam_ke: Option<u32> = jam
            .trim()
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|h| h.parse().ok())
            .filter(|h| *h < 24);
        match (self, jam_ke) {
            (Waktu::Dini, Some(h)) => h < 6,
            (Waktu::Pagi, Some(h)) => (6..12).contains(&h),
            (Waktu::Siang, Some(h)) => (12..18).contains(&h),
            (Waktu::Malam, Some(h)) => h >= 18,
            _ => false,
        }
    }
}

#[component]
pub fn BrowsePage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let navigate = use_navigate();

    let schedules = Resource::new(|| (), |_| list_schedules_upcoming());

    // Filter pencarian — diterapkan langsung ke daftar jadwal di bawah.
    let q_asal = RwSignal::new(String::new());
    let q_tujuan = RwSignal::new(String::new());
    let q_tanggal = RwSignal::new(None::<String>);

    // Kalender jadwal: default bulan & hari ini (WIB).
    let today = today_wib();
    let today_str = today.format("%Y-%m-%d").to_string();
    let cal_from = RwSignal::new(week_start(today));
    let cal_day = RwSignal::new(today_str.clone());
    // Copy-able — dipakai di beberapa closure `Suspense` di bawah.
    let today_sv = StoredValue::new(today_str.clone());
    let cal_waktu = RwSignal::new(Waktu::Semua);
    // Jendela 2 minggu bisa melintasi 2 bulan → ambil keduanya lalu gabungkan.
    let month_schedules = Resource::new(
        move || cal_from.get(),
        |from| async move {
            let to = from + chrono::Duration::days(13);
            let mut list = list_schedules_month(from.year(), from.month()).await?;
            if (to.year(), to.month()) != (from.year(), from.month()) {
                list.extend(list_schedules_month(to.year(), to.month()).await?);
            }
            Ok::<_, ServerFnError>(list)
        },
    );

    let selected = RwSignal::new(None::<Schedule>);
    let jumlah = RwSignal::new(1i32);
    let nama_pemesan = RwSignal::new(String::new());
    let telp_pemesan = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    // Prefill nama/no.HP dari sesi begitu diketahui, tapi jangan timpa yang
    // sudah diketik user.
    Effect::new(move |_| {
        if let Some(Ok(Some(user))) = session.get() {
            if nama_pemesan.get_untracked().is_empty() {
                nama_pemesan.set(user.name.clone());
            }
            if telp_pemesan.get_untracked().is_empty() {
                telp_pemesan.set(user.phone.clone());
            }
        }
    });

    // Alihkan ke login lewat sinyal — `Callback` butuh closure Send+Sync,
    // sedangkan fungsi `navigate` tidak.
    let need_login = RwSignal::new(false);
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if need_login.get() {
                need_login.set(false);
                navigate("/login", Default::default());
            }
        });
    }

    let open_buy = Callback::new(move |s: Schedule| {
        let user = session.get_untracked().and_then(|r| r.ok()).flatten();
        match user {
            None => need_login.set(true),
            Some(u) if u.role != "buyer" => {
                error.set("Hanya akun penumpang yang bisa membeli tiket".into());
            }
            Some(_) => {
                error.set(String::new());
                jumlah.set(1);
                selected.set(Some(s));
            }
        }
    });

    let submit_order = move |_| {
        let Some(schedule) = selected.get_untracked() else {
            return;
        };
        let input = NewOrder {
            schedule_id: schedule.id,
            jumlah_tiket: jumlah.get_untracked(),
            nama_pemesan: nama_pemesan.get_untracked(),
            telp_pemesan: telp_pemesan.get_untracked(),
        };
        if input.jumlah_tiket <= 0 {
            error.set("Jumlah tiket minimal 1".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        let navigate = navigate.clone();
        spawn_client(async move {
            match create_order(input).await {
                Ok(order) => {
                    schedules.refetch();
                    month_schedules.refetch();
                    navigate(&format!("/orders/{}", order.id), Default::default());
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let all = move || schedules.get().and_then(|r| r.ok()).unwrap_or_default();
    let filtered = move || {
        let asal = q_asal.get().trim().to_lowercase();
        let tujuan = q_tujuan.get().trim().to_lowercase();
        let d = q_tanggal.get();
        all()
            .into_iter()
            .filter(|s| asal.is_empty() || s.lokasi_jemput.to_lowercase().contains(&asal))
            .filter(|s| tujuan.is_empty() || s.tujuan.to_lowercase().contains(&tujuan))
            .filter(|s| d.as_ref().is_none_or(|d| &s.tanggal == d))
            .collect::<Vec<_>>()
    };

    let swap = move |_| {
        let a = q_asal.get_untracked();
        q_asal.set(q_tujuan.get_untracked());
        q_tujuan.set(a);
    };

    // Jadwal pada tanggal terpilih (urut jam), sebelum filter waktu.
    let day_schedules = move || {
        let d = cal_day.get();
        let mut list: Vec<Schedule> = month_schedules
            .get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|s| s.tanggal == d)
            .collect();
        list.sort_by(|a, b| a.jam.cmp(&b.jam));
        list
    };

    view! {
        <div class="page page-home">
            <section class="hero">
                <div class="hero-chips">
                    <span class="chip-glass label-caps">
                        <i class="live-dot"></i>
                        "Resmi & Terpercaya"
                    </span>
                    <span class="chip-glass">
                        <Icon name="bolt" class="accent" />
                        "Pesan Cepat 60 Detik"
                    </span>
                </div>
                <h1>"Mau bepergian ke mana hari ini?"</h1>
                <p>"Cari jadwal bis antarkota & amankan tiket perjalananmu dengan tarif resmi terbaik."</p>
            </section>

            <section class="card search-card">
                <div class="route-stack">
                    <label class="route-field">
                        <span class="route-icon">
                            <Icon name="trip_origin" />
                        </span>
                        <span class="route-text">
                            <span class="label-caps">"Dari Mana"</span>
                            <input
                                type="search"
                                placeholder="Kota / titik jemput"
                                prop:value=move || q_asal.get()
                                on:input=move |ev| q_asal.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                    <button type="button" class="swap-btn" title="Tukar asal & tujuan" on:click=swap>
                        <Icon name="swap_vert" />
                    </button>
                    <label class="route-field">
                        <span class="route-icon route-icon-dest">
                            <Icon name="location_on" />
                        </span>
                        <span class="route-text">
                            <span class="label-caps">"Ke Mana"</span>
                            <input
                                type="search"
                                placeholder="Kota tujuan"
                                prop:value=move || q_tujuan.get()
                                on:input=move |ev| q_tujuan.set(event_target_value(&ev))
                            />
                        </span>
                    </label>
                </div>

                <div class="date-box">
                    <div class="date-box-head">
                        <span class="mini-icon">
                            <Icon name="calendar_today" />
                        </span>
                        <span class="label-caps">"Tanggal Berangkat"</span>
                        {move || {
                            q_tanggal
                                .get()
                                .map(|_| {
                                    view! {
                                        <button type="button" class="link-btn" on:click=move |_| q_tanggal.set(None)>
                                            "Reset"
                                        </button>
                                    }
                                })
                        }}
                    </div>
                    <p class="date-current">
                        {move || match q_tanggal.get() {
                            Some(d) => format_tanggal(&d),
                            None => "Semua tanggal".to_string(),
                        }}
                    </p>
                    <Suspense fallback=|| ()>
                        <div class="date-chips">
                            {move || {
                                let mut dates: Vec<String> = all().into_iter().map(|s| s.tanggal).collect();
                                dates.sort();
                                dates.dedup();
                                dates
                                    .into_iter()
                                    .take(14)
                                    .map(|d| {
                                        let (hari, tgl) = tanggal_parts(&d).unwrap_or(("", d.clone()));
                                        let d_cmp = d.clone();
                                        view! {
                                            <button
                                                type="button"
                                                class=move || {
                                                    if q_tanggal.get().as_ref() == Some(&d_cmp) {
                                                        "date-chip active"
                                                    } else {
                                                        "date-chip"
                                                    }
                                                }
                                                on:click=move |_| q_tanggal.set(Some(d.clone()))
                                            >
                                                {format!("{hari}, {tgl}")}
                                            </button>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                    </Suspense>
                </div>

                <a href="#jadwal" class="btn btn-cta btn-block btn-lg">
                    <Icon name="search" />
                    "Cari Jadwal Bis"
                    <Icon name="arrow_forward" />
                </a>
                <div class="trust-mini">
                    <span>
                        <Icon name="verified_user" />
                        "Tarif Resmi PO"
                    </span>
                    <span>
                        <Icon name="money_off" />
                        "Bebas Biaya Admin"
                    </span>
                    <span>
                        <Icon name="qr_code_2" />
                        "E-Tiket Instan"
                    </span>
                </div>
            </section>

            <section id="kalender" class="section">
                <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                    {
                        let today_str = today_sv.get_value();
                        move || {
                            let sch = month_schedules.get().and_then(|r| r.ok()).unwrap_or_default();
                            view! {
                                <FareCalendar
                                    start=cal_from.get()
                                    schedules=sch
                                    selected=cal_day.get()
                                    today=today_str.clone()
                                    on_pick=move |d: String| {
                                        cal_day.set(d);
                                        cal_waktu.set(Waktu::Semua);
                                    }
                                    on_shift=move |n: i64| cal_from.update(|f| *f += chrono::Duration::days(n))
                                />
                            }
                        }
                    }
                </Suspense>

                <Suspense fallback=|| ()>
                    {
                        let today_str = today_sv.get_value();
                        move || {
                            let d = cal_day.get();
                            let all = month_schedules.get().and_then(|r| r.ok()).unwrap_or_default();
                            let list = day_schedules();
                            let tersedia = list.iter().filter(|s| s.sisa_kursi() > 0).count();
                            let mulai = min_fare(&all, &d);
                            // "Paling hemat" = tarif termurah di jendela 2 minggu (mulai hari ini).
                            let from = cal_from.get();
                            let hemat = mulai.is_some()
                                && (0..14)
                                    .map(|i| (from + chrono::Duration::days(i)).format("%Y-%m-%d").to_string())
                                    .filter(|ds| *ds >= today_str)
                                    .filter_map(|ds| min_fare(&all, &ds))
                                    .min() == mulai;
                            let sub = match mulai {
                                Some(m) => format!("{tersedia} bus tersedia • Tarif mulai Rp {}", format_rupiah(m)),
                                None if list.is_empty() => "Belum ada bus pada tanggal ini".to_string(),
                                None => "Semua kursi sudah terjual".to_string(),
                            };
                            view! {
                                <div class="day-banner">
                                    <span class="day-icon">
                                        <Icon name="event_available" />
                                    </span>
                                    <div>
                                        <h3>
                                            {format_tanggal_panjang(&d)}
                                            {(d == today_str).then(|| view! { <small>"Hari ini"</small> })}
                                        </h3>
                                        <p>{sub}</p>
                                    </div>
                                    {hemat.then(|| view! { <span class="hemat-chip">"Paling Hemat"</span> })}
                                </div>
                                <div class="chip-row">
                                    {Waktu::ALL
                                        .into_iter()
                                        .filter(|w| *w == Waktu::Semua || list.iter().any(|s| w.cocok(&s.jam)) || *w != Waktu::Dini)
                                        .map(|w| {
                                            let n = list.iter().filter(|s| w.cocok(&s.jam)).count();
                                            let label = if w == Waktu::Semua {
                                                format!("{} ({n})", w.label())
                                            } else {
                                                w.label().to_string()
                                            };
                                            view! {
                                                <button
                                                    type="button"
                                                    class=move || if cal_waktu.get() == w { "time-chip active" } else { "time-chip" }
                                                    on:click=move |_| cal_waktu.set(w)
                                                >
                                                    {label}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                        }
                    }
                </Suspense>

                <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                    {
                        let today_str = today_sv.get_value();
                        move || {
                            let bookable = cal_day.get() >= today_str;
                            let w = cal_waktu.get();
                            let list: Vec<Schedule> = day_schedules().into_iter().filter(|s| w.cocok(&s.jam)).collect();
                            if list.is_empty() {
                                view! {
                                    <div class="card empty-state">
                                        <Icon name="event_busy" />
                                        <p>"Tidak ada bus pada tanggal / waktu ini. Coba tanggal lain di kalender."</p>
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div class="trip-grid">
                                        {list
                                            .into_iter()
                                            .map(|s| view! { <TripCard schedule=s on_pick=open_buy bookable=bookable /> })
                                            .collect_view()}
                                    </div>
                                }
                                    .into_any()
                            }
                        }
                    }
                </Suspense>
            </section>

            <section class="promo-banner">
                <div>
                    <span class="pill pill-cta">"Spesial Rombongan"</span>
                    <h3>"Sewa bus untuk rombongan?"</h3>
                    <p>"Mitra PO LajuBus siap untuk study tour, ziarah, dan perjalanan keluarga."</p>
                </div>
                <span class="promo-code">
                    <strong>"LAJUBUS"</strong>
                    <small>"Hubungi mitra"</small>
                </span>
            </section>

            <section id="jadwal" class="section">
                <div class="section-head">
                    <div>
                        <h2>"Jadwal Keberangkatan"</h2>
                        <p>
                            {move || match q_tanggal.get() {
                                Some(d) => format_tanggal(&d),
                                None => "Semua keberangkatan mendatang".to_string(),
                            }}
                        </p>
                    </div>
                    <Suspense fallback=|| ()>
                        <span class="pill pill-primary">{move || format!("{} jadwal", filtered().len())}</span>
                    </Suspense>
                </div>
                {move || {
                    (!error.get().is_empty() && selected.get().is_none())
                        .then(|| view! { <p class="alert alert-error">{error.get()}</p> })
                }}
                <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                    {move || {
                        let list = filtered();
                        if list.is_empty() {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="directions_bus" />
                                    <p>"Belum ada jadwal yang cocok. Coba asal, tujuan, atau tanggal lain."</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            view! {
                                <div class="trip-grid">
                                    {list
                                        .into_iter()
                                        .map(|s| view! { <TripCard schedule=s on_pick=open_buy /> })
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
                    <h2>"Kenapa Pilih LajuBus?"</h2>
                    <span class="label-caps">"Jaminan Kepuasan"</span>
                </div>
                <div class="feature-row">
                    <div class="card feature">
                        <span class="feature-icon f-orange">
                            <Icon name="confirmation_number" />
                        </span>
                        <strong>"Tiket Resmi"</strong>
                        <p>"Kode order langsung terbit setelah pemesanan."</p>
                    </div>
                    <div class="card feature">
                        <span class="feature-icon f-blue">
                            <Icon name="badge" />
                        </span>
                        <strong>"Driver Terjadwal"</strong>
                        <p>"Nama & kontak driver tercantum di tiketmu."</p>
                    </div>
                    <div class="card feature">
                        <span class="feature-icon f-green">
                            <Icon name="star" />
                        </span>
                        <strong>"Rating Terbuka"</strong>
                        <p>"Nilai armada & driver setelah perjalanan."</p>
                    </div>
                </div>
            </section>

            {move || {
                selected
                    .get()
                    .map(|s| {
                        let submit_order = submit_order.clone();
                        let sisa = s.sisa_kursi();
                        let max_kursi = sisa.max(1);
                        let harga = s.harga;
                        view! {
                            <div class="modal-overlay" on:click=move |_| selected.set(None)>
                                <div class="sheet" on:click=|ev| ev.stop_propagation()>
                                    <div class="sheet-grip"></div>
                                    <div class="sheet-header">
                                        <div>
                                            <span class="label-caps">"Pemesanan"</span>
                                            <h3>"Pilih Kursi & Penumpang"</h3>
                                        </div>
                                        <button type="button" class="icon-btn" title="Tutup" on:click=move |_| selected.set(None)>
                                            <Icon name="close" />
                                        </button>
                                    </div>
                                    <div class="sheet-body">
                                        <div class="card trip-summary">
                                            <div class="trip-head">
                                                <OpLogo name=s.armada_name.clone() color=s.armada_color_hex.clone() />
                                                <div class="trip-op">
                                                    <h3>
                                                        {s.armada_name.clone()}
                                                        <Icon name="verified" filled=true class="verified" />
                                                    </h3>
                                                    <p>
                                                        <Icon name="calendar_today" />
                                                        {format_tanggal(&s.tanggal)}
                                                    </p>
                                                </div>
                                            </div>
                                            <RouteTimeline
                                                jam=s.jam.clone()
                                                from=s.lokasi_jemput.clone()
                                                to=s.tujuan.clone()
                                                mid="Langsung"
                                            />
                                        </div>

                                        <div class="sub-head">
                                            <h4>"Jumlah Kursi"</h4>
                                            <span class="pill pill-ok">{format!("Sisa {sisa} kursi")}</span>
                                        </div>
                                        <div class="stepper">
                                            <button
                                                type="button"
                                                class="icon-btn"
                                                disabled={move || jumlah.get() <= 1}
                                                on:click=move |_| jumlah.update(|n| *n = (*n - 1).max(1))
                                            >
                                                <Icon name="remove" />
                                            </button>
                                            <span class="stepper-val">
                                                <Icon name="event_seat" />
                                                {move || format!("{} kursi", jumlah.get())}
                                            </span>
                                            <button
                                                type="button"
                                                class="icon-btn"
                                                disabled={move || jumlah.get() >= max_kursi}
                                                on:click=move |_| jumlah.update(|n| *n = (*n + 1).min(max_kursi))
                                            >
                                                <Icon name="add" />
                                            </button>
                                        </div>

                                        <div class="sub-head">
                                            <h4>"Data Pemesan"</h4>
                                            <span class="pill pill-primary">"Sesuai KTP"</span>
                                        </div>
                                        <div class="card form-card">
                                            <label class="field">
                                                <span class="field-label">"Nama Lengkap"</span>
                                                <span class="input-wrap">
                                                    <Icon name="person" />
                                                    <input
                                                        type="text"
                                                        placeholder="Nama pemesan"
                                                        prop:value=move || nama_pemesan.get()
                                                        on:input=move |ev| nama_pemesan.set(event_target_value(&ev))
                                                    />
                                                </span>
                                            </label>
                                            <label class="field">
                                                <span class="field-label">"No. WhatsApp / HP"</span>
                                                <span class="input-wrap">
                                                    <Icon name="chat" />
                                                    <input
                                                        type="tel"
                                                        placeholder="08xx"
                                                        prop:value=move || telp_pemesan.get()
                                                        on:input=move |ev| telp_pemesan.set(event_target_value(&ev))
                                                    />
                                                </span>
                                            </label>
                                            <div class="info-row">
                                                <Icon name="location_on" />
                                                <div>
                                                    <strong>{format!("Titik jemput: {}", s.lokasi_jemput)}</strong>
                                                    <small>{format!("Harap tiba sebelum {}", s.jam)}</small>
                                                </div>
                                            </div>
                                        </div>

                                        <div class="sub-head">
                                            <h4>"Rincian Pembayaran"</h4>
                                        </div>
                                        <div class="card bill">
                                            <div class="bill-row">
                                                <span>{move || format!("Tarif kursi × {}", jumlah.get())}</span>
                                                <span class="num">
                                                    {move || format!("Rp {}", format_rupiah(harga * jumlah.get() as i64))}
                                                </span>
                                            </div>
                                            <div class="bill-row">
                                                <span>"Biaya layanan"</span>
                                                <span class="pill pill-ok">"Gratis"</span>
                                            </div>
                                            <div class="bill-row bill-total">
                                                <span>"Total Tagihan"</span>
                                                <span class="num">
                                                    {move || format!("Rp {}", format_rupiah(harga * jumlah.get() as i64))}
                                                </span>
                                            </div>
                                        </div>
                                        <p class="note-card">
                                            <Icon name="verified" />
                                            "Tiket resmi mitra PO LajuBus. Kode order langsung terbit setelah konfirmasi."
                                        </p>
                                        {move || {
                                            (!error.get().is_empty())
                                                .then(|| view! { <p class="alert alert-error">{error.get()}</p> })
                                        }}
                                    </div>
                                    <div class="sheet-footer">
                                        <div class="total-block">
                                            <span class="label-caps">
                                                {move || format!("Total · {} kursi", jumlah.get())}
                                            </span>
                                            <strong class="price-lg">
                                                {move || format!("Rp {}", format_rupiah(harga * jumlah.get() as i64))}
                                            </strong>
                                        </div>
                                        <button
                                            type="button"
                                            class="btn btn-cta"
                                            on:click=submit_order
                                            disabled=move || busy.get()
                                        >
                                            {move || if busy.get() { "Memproses…" } else { "Lanjut Bayar" }}
                                            <Icon name="arrow_forward" />
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}
