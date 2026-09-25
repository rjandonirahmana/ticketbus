//! web/pages/merchant.rs — "/merchant": armada, jadwal, dan foto trip MILIK
//! SENDIRI saja (server fn yang men-scope-nya, bukan halaman ini).

use leptos::prelude::*;

use crate::web::api::{
    create_armada, delete_photo, delete_schedule, list_my_armadas, list_my_photos, list_my_schedules, list_photos,
};
use crate::web::app::SessionResource;
use crate::web::components::{
    clean_error, format_rupiah, spawn_client, ArmadaManager, Icon, PhotoGallery, PhotoUpload, RentalManager,
    ScheduleCard, ScheduleForm,
};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Jadwal,
    Armada,
    Foto,
    Sewa,
}

#[component]
pub fn MerchantPage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let merchant_name = move || session.get().and_then(|r| r.ok()).flatten().map(|u| u.name).unwrap_or_default();

    let active_tab = RwSignal::new(Tab::Jadwal);

    let armadas = Resource::new(|| (), |_| list_my_armadas());
    let schedules = Resource::new(|| (), |_| list_my_schedules());

    let photo_armada = RwSignal::new(None::<String>);
    // Tanpa filter = foto dari SEMUA armada milik sendiri (bukan `list_photos(None)`,
    // yang berarti seluruh foto di platform — termasuk milik merchant lain).
    let photos = Resource::new(
        move || photo_armada.get(),
        |f| async move {
            match f {
                Some(id) => list_photos(Some(id)).await,
                None => list_my_photos().await,
            }
        },
    );

    // ── Kelola armada ───────────────────────────────────────────────────────
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

    let remove_schedule = move |id: String| {
        spawn_client(async move {
            let _ = delete_schedule(id).await;
            schedules.refetch();
        });
    };

    let tab_tile = move |tab: Tab, icon: &'static str, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || if active_tab.get() == tab { "quick-tile active" } else { "quick-tile" }
                on:click=move |_| active_tab.set(tab)
            >
                <span class="quick-icon">
                    <Icon name=icon />
                </span>
                <span>{label}</span>
            </button>
        }
    };

    view! {
        <div class="page">
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                <section class="card po-card">
                    <div class="po-top">
                        <span class="po-logo">
                            <Icon name="directions_bus" />
                        </span>
                        <div class="po-id">
                            <h1>
                                {move || merchant_name()}
                                <Icon name="verified" filled=true class="verified" />
                            </h1>
                            <p>
                                <Icon name="storefront" />
                                "Mitra PO LajuBus"
                            </p>
                        </div>
                        <span class="status-pill">
                            <i></i>
                            "Terverifikasi"
                        </span>
                    </div>
                    <div class="po-strip">
                        <Icon name="shield" />
                        <div>
                            <strong>"Akses Merchant Terproteksi"</strong>
                            <small>"Hanya armada, jadwal & foto milik PO Anda"</small>
                        </div>
                        <span class="pill pill-primary">"100% Hak Kelola"</span>
                    </div>
                </section>

                <button type="button" class="action-bar" on:click=move |_| active_tab.set(Tab::Jadwal)>
                    <span class="action-icon">
                        <Icon name="add_road" />
                    </span>
                    <span class="action-text">
                        <strong>"Terbitkan Jadwal Keberangkatan"</strong>
                        <small>"Jadwal langsung tampil di beranda & kalender penumpang"</small>
                    </span>
                    <Icon name="chevron_right" />
                </button>

                <div class="section-head">
                    <h2>"Statistik Armada"</h2>
                    <span class="label-caps">"Semua Jadwal"</span>
                </div>
                {move || {
                    let sch = schedules.get().and_then(|r| r.ok()).unwrap_or_default();
                    let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                    let terjual: i64 = sch.iter().map(|s| s.kursi_terjual as i64).sum();
                    let kapasitas: i64 = sch.iter().map(|s| s.kapasitas as i64).sum();
                    let bruto: i64 = sch.iter().map(|s| s.harga * s.kursi_terjual as i64).sum();
                    let okupansi = if kapasitas > 0 { terjual * 100 / kapasitas } else { 0 };
                    view! {
                        <div class="stat-grid">
                            <div class="card stat">
                                <div class="stat-top">
                                    <span>"Tiket Terjual"</span>
                                    <span class="stat-icon s-blue">
                                        <Icon name="confirmation_number" />
                                    </span>
                                </div>
                                <strong>{terjual}</strong>
                                <small>{format!("dari {kapasitas} kursi dijadwalkan")}</small>
                            </div>
                            <div class="card stat">
                                <div class="stat-top">
                                    <span>"Pendapatan Bruto"</span>
                                    <span class="stat-icon s-orange">
                                        <Icon name="payments" />
                                    </span>
                                </div>
                                <strong>{format!("Rp {}", format_rupiah(bruto))}</strong>
                                <small>"Akumulasi seluruh jadwal"</small>
                            </div>
                            <div class="card stat">
                                <div class="stat-top">
                                    <span>"Rata-rata Okupansi"</span>
                                    <span class="stat-icon s-green">
                                        <Icon name="airline_seat_recline_normal" />
                                    </span>
                                </div>
                                <strong>{format!("{okupansi}%")}</strong>
                                <div class="bar">
                                    <i style=format!("width:{okupansi}%")></i>
                                </div>
                            </div>
                            <div class="card stat">
                                <div class="stat-top">
                                    <span>"Armada Aktif"</span>
                                    <span class="stat-icon s-violet">
                                        <Icon name="directions_bus" />
                                    </span>
                                </div>
                                <strong>{format!("{} unit", arm.len())}</strong>
                                <small>{format!("{} jadwal terbit", sch.len())}</small>
                            </div>
                        </div>
                    }
                }}

                <div class="quick-row">
                    {tab_tile(Tab::Jadwal, "event_available", "Jadwal")}
                    {tab_tile(Tab::Armada, "directions_bus", "Armada")}
                    {tab_tile(Tab::Foto, "photo_library", "Foto Trip")}
                    {tab_tile(Tab::Sewa, "beach_access", "Sewa & Wisata")}
                </div>

                {move || match active_tab.get() {
                    Tab::Jadwal => {
                        let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                        let sch = schedules.get().and_then(|r| r.ok()).unwrap_or_default();
                        let count = sch.len();
                        view! {
                            <div class="section-head">
                                <div>
                                    <h2>"Terbitkan Jadwal Baru"</h2>
                                    <p>"Jadwal langsung tampil di beranda penumpang"</p>
                                </div>
                            </div>
                            {if arm.is_empty() {
                                view! {
                                    <p class="alert alert-warn">
                                        <Icon name="info" />
                                        "Tambahkan armada dulu di menu \"Armada\"."
                                    </p>
                                }
                                    .into_any()
                            } else {
                                view! { <ScheduleForm armadas=arm on_saved=move |_| schedules.refetch() /> }.into_any()
                            }}

                            <div class="section-head">
                                <h2>"Keberangkatan Bus"</h2>
                                <span class="pill pill-primary">{format!("Semua ({count})")}</span>
                            </div>
                            {if sch.is_empty() {
                                view! {
                                    <div class="card empty-state">
                                        <Icon name="event_busy" />
                                        <p>"Belum ada jadwal."</p>
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div class="ops-grid">
                                        {sch
                                            .into_iter()
                                            .enumerate()
                                            .map(|(i, s)| {
                                                view! { <ScheduleCard schedule=s index=i on_delete=remove_schedule /> }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                                    .into_any()
                            }}
                        }
                            .into_any()
                    }

                    Tab::Armada => {
                        let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                        view! {
                            <section class="card form-card">
                                <div class="step-head">
                                    <span class="step-icon">
                                        <Icon name="directions_bus" />
                                    </span>
                                    <div>
                                        <h3>"Armada Saya"</h3>
                                        <p>"Ubah nama & warna penanda, lihat rating penumpang"</p>
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
                                            placeholder="Nama armada baru, mis. Bus SJ-01"
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

                    Tab::Sewa => view! { <RentalManager /> }.into_any(),

                    Tab::Foto => {
                        let arm = armadas.get().and_then(|r| r.ok()).unwrap_or_default();
                        view! {
                            <PhotoUpload armadas=arm.clone() on_uploaded=move |_| photos.refetch() />
                            <div class="section-head">
                                <h2>"Galeri Trip"</h2>
                            </div>
                            <div class="chip-row">
                                <button
                                    type="button"
                                    class=move || if photo_armada.get().is_none() { "filter-chip active" } else { "filter-chip" }
                                    on:click=move |_| photo_armada.set(None)
                                >
                                    "Semua armada"
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
                                                    if photo_armada.get().as_ref() == Some(&id_cmp) {
                                                        "filter-chip active"
                                                    } else {
                                                        "filter-chip"
                                                    }
                                                }
                                                on:click=move |_| photo_armada.set(Some(id.clone()))
                                            >
                                                <span class="swatch" style=format!("background:{}", a.color_hex)></span>
                                                {a.name.clone()}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
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
            </Suspense>
        </div>
    }
}
