//! web/pages/admin.rs — "/admin": kelola SEMUA armada & jadwal lintas
//! merchant, plus foto trip global. Setara dashboard admin situs asli,
//! sekarang di atas sistem akun sungguhan (AdminGuard + require_role).

use chrono::{Datelike, Utc};
use leptos::prelude::*;

use crate::web::api::{
    create_armada, delete_photo, delete_schedule, list_armadas, list_photos, list_schedules_by_armada,
    list_schedules_month,
};
use crate::web::app::SessionResource;
use crate::web::components::{
    clean_error, format_rupiah, month_label, shift_month, spawn_client, ArmadaManager, BannerManager, CalendarGrid, MitraReview, Icon, PhotoGallery, PhotoUpload,
    RentalManager, RouteManager, ScheduleCard, ScheduleForm, ScheduleModal,
};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Jadwal,
    Tambah,
    Detail,
    Foto,
    Sewa,
    Banner,
    Mitra,
}

#[component]
pub fn AdminPage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let admin_name = move || session.get().and_then(|r| r.ok()).flatten().map(|u| u.name).unwrap_or_default();

    let active_tab = RwSignal::new(Tab::Jadwal);
    let show_oneoff = RwSignal::new(false);

    let now = Utc::now();
    let year = RwSignal::new(now.year());
    let month = RwSignal::new(now.month());
    let selected_day = RwSignal::new(None::<String>);

    let armadas = Resource::new(|| (), |_| list_armadas());
    let schedules_month = Resource::new(
        move || (year.get(), month.get()),
        |(y, m)| async move { list_schedules_month(y, m).await },
    );
    let detail_filter = RwSignal::new(None::<String>);
    let schedules_detail = Resource::new(
        move || detail_filter.get(),
        |f| async move { list_schedules_by_armada(f).await },
    );
    let photo_filter = RwSignal::new(None::<String>);
    let photos = Resource::new(move || photo_filter.get(), |f| async move { list_photos(f).await });

    let remove_schedule = move |id: String| {
        spawn_client(async move {
            let _ = delete_schedule(id).await;
            schedules_month.refetch();
            schedules_detail.refetch();
        });
    };

    // ── Kelola armada ────────────────────────────────────────────────────────
    let a_name = RwSignal::new(String::new());
    let a_color = RwSignal::new("#0f4c81".to_string());
    let armada_error = RwSignal::new(String::new());
    let submit_armada = move |_| {
        let name = a_name.get_untracked();
        let color = a_color.get_untracked();
        spawn_client(async move {
            match create_armada(name, color).await {
                Ok(_) => {
                    a_name.set(String::new());
                    armada_error.set(String::new());
                    armadas.refetch();
                }
                Err(e) => armada_error.set(clean_error(&e.to_string())),
            }
        });
    };

    let tab_pill = move |tab: Tab, icon: &'static str, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || if active_tab.get() == tab { "filter-chip active" } else { "filter-chip" }
                on:click=move |_| active_tab.set(tab)
            >
                <Icon name=icon />
                {label}
            </button>
        }
    };

    // Chip filter armada — dipakai tab Detail (jadwal) & Foto.
    let armada_chips = move |filter: RwSignal<Option<String>>| {
        let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
        view! {
            <div class="chip-row">
                <button
                    type="button"
                    class=move || if filter.get().is_none() { "filter-chip active" } else { "filter-chip" }
                    on:click=move |_| filter.set(None)
                >
                    "Semua Armada"
                </button>
                {arm
                    .into_iter()
                    .map(|a| {
                        let id = a.id.clone();
                        let id_cmp = a.id.clone();
                        view! {
                            <button
                                type="button"
                                class=move || {
                                    if filter.get().as_ref() == Some(&id_cmp) { "filter-chip active" } else { "filter-chip" }
                                }
                                on:click=move |_| filter.set(Some(id.clone()))
                            >
                                <span class="swatch" style=format!("background:{}", a.color_hex)></span>
                                {a.name.clone()}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        }
    };

    view! {
        <div class="page">
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                <section class="hub-card">
                    <div class="hub-top">
                        <span class="hub-logo">
                            <Icon name="directions_bus" />
                        </span>
                        <div>
                            <h1>
                                "LajuBus Control Hub"
                                <span class="super-badge">"SuperAdmin"</span>
                            </h1>
                            <p>{move || format!("Halo, {} — pengawasan armada & jadwal nasional", admin_name())}</p>
                        </div>
                    </div>
                    <span class="status-chip status-chip-card">
                        <i class="status-dot"></i>
                        "Sistem tiket & kalender normal"
                    </span>
                </section>

                {move || {
                    let sch = schedules_month.get().and_then(|r| r.ok()).unwrap_or_default();
                    let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                    let terjual: i64 = sch.iter().map(|s| s.kursi_terjual as i64).sum();
                    let kapasitas: i64 = sch.iter().map(|s| s.kapasitas as i64).sum();
                    let gmv: i64 = sch.iter().map(|s| s.harga * s.kursi_terjual as i64).sum();
                    let okupansi = if kapasitas > 0 { terjual * 100 / kapasitas } else { 0 };
                    let mitra = {
                        let mut ids: Vec<_> = arm.iter().filter_map(|a| a.merchant_id.clone()).collect();
                        ids.sort();
                        ids.dedup();
                        ids.len()
                    };
                    view! {
                        <div class="stat-grid">
                            <div class="card stat">
                                <div class="stat-top">
                                    <span class="label-caps">"GMV Bulan Ini"</span>
                                    <span class="stat-icon s-blue">
                                        <Icon name="account_balance_wallet" />
                                    </span>
                                </div>
                                <strong>{format!("Rp {}", format_rupiah(gmv))}</strong>
                                <small>{move || month_label(year.get(), month.get())}</small>
                            </div>
                            <div class="card stat">
                                <div class="stat-top">
                                    <span class="label-caps">"Tiket Terbit"</span>
                                    <span class="stat-icon s-orange">
                                        <Icon name="confirmation_number" />
                                    </span>
                                </div>
                                <strong>{terjual}</strong>
                                <small>{format!("{} jadwal bulan ini", sch.len())}</small>
                            </div>
                            <div class="card stat">
                                <div class="stat-top">
                                    <span class="label-caps">"Armada Terdaftar"</span>
                                    <span class="stat-icon s-violet">
                                        <Icon name="directions_bus" />
                                    </span>
                                </div>
                                <strong>{arm.len()}</strong>
                                <small>{format!("{mitra} mitra PO")}</small>
                            </div>
                            <div class="card stat">
                                <div class="stat-top">
                                    <span class="label-caps">"Okupansi"</span>
                                    <span class="stat-icon s-green">
                                        <Icon name="speed" />
                                    </span>
                                </div>
                                <strong>{format!("{okupansi}%")}</strong>
                                <div class="bar">
                                    <i style=format!("width:{okupansi}%")></i>
                                </div>
                            </div>
                        </div>
                    }
                }}

                <div class="chip-row tab-row">
                    {tab_pill(Tab::Jadwal, "calendar_month", "Kalender")}
                    {tab_pill(Tab::Tambah, "add_circle", "Trayek & Armada")}
                    {tab_pill(Tab::Detail, "list_alt", "Detail Order")}
                    {tab_pill(Tab::Foto, "photo_library", "Foto Trip")}
                    {tab_pill(Tab::Sewa, "beach_access", "Sewa & Wisata")}
                    {tab_pill(Tab::Mitra, "storefront", "Mitra PO")}
                    {tab_pill(Tab::Banner, "ad_units", "Banner")}
                </div>

                {move || match active_tab.get() {
                    Tab::Jadwal => {
                        view! {
                            <section class="card calendar-card">
                                <div class="calendar-nav">
                                    <button
                                        type="button"
                                        class="icon-btn"
                                        title="Bulan sebelumnya"
                                        on:click=move |_| {
                                            let (y, m) = shift_month(year.get_untracked(), month.get_untracked(), -1);
                                            year.set(y);
                                            month.set(m);
                                        }
                                    >
                                        <Icon name="chevron_left" />
                                    </button>
                                    <span class="month-title">{move || month_label(year.get(), month.get())}</span>
                                    <button
                                        type="button"
                                        class="icon-btn"
                                        title="Bulan berikutnya"
                                        on:click=move |_| {
                                            let (y, m) = shift_month(year.get_untracked(), month.get_untracked(), 1);
                                            year.set(y);
                                            month.set(m);
                                        }
                                    >
                                        <Icon name="chevron_right" />
                                    </button>
                                </div>
                                {move || {
                                    let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                                    let sch = schedules_month.get().and_then(|r| r.ok()).unwrap_or_default();
                                    view! {
                                        <CalendarGrid
                                            selected=selected_day.get().unwrap_or_default()
                                            year=year.get()
                                            month=month.get()
                                            schedules=sch
                                            on_day_click=move |d: String| selected_day.set(Some(d))
                                        />
                                        <div class="legend">
                                            <span class="label-caps">"Keterangan Armada"</span>
                                            <div class="legend-list">
                                                {arm
                                                    .into_iter()
                                                    .map(|a| {
                                                        view! {
                                                            <span class="legend-item">
                                                                <span class="dot" style=format!("background:{}", a.color_hex)></span>
                                                                {a.name}
                                                            </span>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        </div>
                                    }
                                }}
                            </section>
                        }
                            .into_any()
                    }

                    Tab::Tambah => {
                        let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                        view! {
                            <section class="authority-banner">
                                <div class="ab-top">
                                    <span class="ab-badge">
                                        <Icon name="shield_person" />
                                        "SUPERADMIN PLATFORM"
                                    </span>
                                    <span class="ab-live">
                                        <i class="live-dot"></i>
                                        "Bypass Verifikasi"
                                    </span>
                                </div>
                                <h2>"Otoritas Terbit Instan"</h2>
                                <p>"Jadwal & armada dari Admin langsung aktif di beranda dan kalender penumpang."</p>
                                <div class="ab-metrics">
                                    <div>
                                        <span class="ab-icon ab-orange">
                                            <Icon name="event_available" />
                                        </span>
                                        <span>
                                            <strong>
                                                {move || {
                                                    format!(
                                                        "{} Jadwal",
                                                        schedules_month.get().and_then(|r| r.ok()).map(|v| v.len()).unwrap_or(0),
                                                    )
                                                }}
                                            </strong>
                                            <small>"Bulan ini"</small>
                                        </span>
                                    </div>
                                    <div>
                                        <span class="ab-icon ab-green">
                                            <Icon name="bolt" />
                                        </span>
                                        <span>
                                            <strong>{format!("{} Armada", arm.len())}</strong>
                                            <small>"Siap dirilis"</small>
                                        </span>
                                    </div>
                                </div>
                            </section>
                            <RouteManager armadas=arm.clone() />
                            <button type="button" class="action-bar" on:click=move |_| show_oneoff.update(|v| *v = !*v)>
                                <span class="action-icon">
                                    <Icon name="event" />
                                </span>
                                <span class="action-text">
                                    <strong>"Jadwal Tambahan (Sekali Jalan)"</strong>
                                    <small>"Di luar trayek tetap — langsung terbit tanpa verifikasi"</small>
                                </span>
                                {move || view! { <Icon name=if show_oneoff.get() { "expand_less" } else { "expand_more" } /> }}
                            </button>
                            {
                                let arm = arm.clone();
                                move || {
                                    show_oneoff
                                        .get()
                                        .then(|| {
                                            view! {
                                                <ScheduleForm
                                                    armadas=arm.clone()
                                                    on_saved=move |_| {
                                                        schedules_month.refetch();
                                                        schedules_detail.refetch();
                                                    }
                                                />
                                            }
                                        })
                                }
                            }

                            <section class="card form-card">
                                <div class="step-head">
                                    <span class="step-icon">
                                        <Icon name="directions_bus" />
                                    </span>
                                    <div>
                                        <h3>"Kelola Armada"</h3>
                                        <p>"Semua merchant + armada milik platform"</p>
                                    </div>
                                </div>
                                <ArmadaManager armadas=arm on_changed=move |_| armadas.refetch() />
                                <div class="armada-add">
                                    <label class="color-chip" style=move || format!("background:{}", a_color.get())>
                                        <input
                                            type="color"
                                            prop:value=move || a_color.get()
                                            on:input=move |ev| a_color.set(event_target_value(&ev))
                                        />
                                    </label>
                                    <span class="input-wrap">
                                        <Icon name="add_road" />
                                        <input
                                            type="text"
                                            placeholder="Nama armada baru (platform)"
                                            prop:value=move || a_name.get()
                                            on:input=move |ev| a_name.set(event_target_value(&ev))
                                        />
                                    </span>
                                    <button type="button" class="btn btn-cta" on:click=submit_armada>
                                        <Icon name="add" />
                                        "Tambah"
                                    </button>
                                </div>
                                {move || {
                                    (!armada_error.get().is_empty())
                                        .then(|| view! { <p class="alert alert-error">{armada_error.get()}</p> })
                                }}
                            </section>
                        }
                            .into_any()
                    }

                    Tab::Detail => {
                        view! {
                            <div class="section-head">
                                <h2>"Detail Order per Bus"</h2>
                            </div>
                            {armada_chips(detail_filter)}
                            {move || {
                                let sch = schedules_detail.get().and_then(|r| r.ok()).unwrap_or_default();
                                if sch.is_empty() {
                                    view! {
                                        <div class="card empty-state">
                                            <Icon name="event_busy" />
                                            <p>"Belum ada order."</p>
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="ops-grid">
                                            {sch
                                                .into_iter()
                                                .map(|s| view! { <ScheduleCard schedule=s on_delete=remove_schedule /> })
                                                .collect_view()}
                                        </div>
                                    }
                                        .into_any()
                                }
                            }}
                        }
                            .into_any()
                    }

                    Tab::Sewa => view! { <RentalManager /> }.into_any(),

                    Tab::Banner => view! { <BannerManager /> }.into_any(),

                    Tab::Mitra => view! { <MitraReview /> }.into_any(),

                    Tab::Foto => {
                        let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                        view! {
                            <PhotoUpload armadas=arm on_uploaded=move |_| photos.refetch() />
                            <div class="section-head">
                                <h2>"Galeri Trip"</h2>
                            </div>
                            {armada_chips(photo_filter)}
                            {move || {
                                let ph = photos.get().and_then(|r| r.ok()).unwrap_or_default();
                                view! {
                                    <PhotoGallery
                                        photos=ph
                                        is_admin=true
                                        on_delete=move |id: String| {
                                            spawn_client(async move {
                                                let _ = delete_photo(id).await;
                                                photos.refetch();
                                            });
                                        }
                                    />
                                }
                            }}
                        }
                            .into_any()
                    }
                }}

                {move || {
                    selected_day
                        .get()
                        .map(|date| {
                            let sch = schedules_month.get().and_then(|r| r.ok()).unwrap_or_default();
                            let day_items: Vec<_> = sch.into_iter().filter(|s| s.tanggal == date).collect();
                            view! {
                                <ScheduleModal
                                    date=date.clone()
                                    schedules=day_items
                                    is_admin=true
                                    on_close=move |_| selected_day.set(None)
                                    on_delete=move |id: String| {
                                        remove_schedule(id);
                                        selected_day.set(None);
                                    }
                                />
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}
