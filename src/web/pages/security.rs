//! web/pages/security.rs — "/akun/keamanan": skor keamanan (dari faktor
//! nyata), kredensial, perangkat & sesi aktif, riwayat aktivitas, dan
//! bekukan akun bila ponsel hilang.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::web::api::{freeze_account, get_cs_contact, get_security_overview, logout_other_devices, revoke_device};
use crate::web::app::SessionResource;
use crate::web::components::{clean_error, format_tanggal, spawn_client, wa_link, AccountTabs, Icon};
use crate::web::models::{SecurityEvent, SecurityOverview, SessionInfo};

/// RFC 3339 → "Aktif sekarang" / "3 jam lalu" / "12 hari lalu".
fn relatif(rfc3339: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(rfc3339) else {
        return "-".into();
    };
    let d = chrono::Utc::now() - t.with_timezone(&chrono::Utc);
    match (d.num_minutes(), d.num_hours(), d.num_days()) {
        (m, _, _) if m < 10 => "Aktif sekarang".into(),
        (m, _, _) if m < 60 => format!("{m} menit lalu"),
        (_, h, _) if h < 24 => format!("{h} jam lalu"),
        (_, _, 1) => "Kemarin".into(),
        (_, _, n) => format!("{n} hari lalu"),
    }
}

/// RFC 3339 → "Hari ini, 08:14 WIB" / "24 Sep 2026".
fn waktu_event(rfc3339: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(rfc3339) else {
        return "-".into();
    };
    let wib = t.with_timezone(&chrono::Utc) + chrono::Duration::hours(7);
    let today = crate::web::components::today_wib();
    if wib.date_naive() == today {
        format!("Hari ini, {} WIB", wib.format("%H:%M"))
    } else {
        format_tanggal(&wib.format("%Y-%m-%d").to_string())
    }
}

fn event_label(jenis: &str) -> (&'static str, &'static str, &'static str) {
    // (judul, ikon, kelas nada)
    match jenis {
        "login_berhasil" => ("Masuk Akun Berhasil", "login", "ev-ok"),
        "login_gagal" => ("Percobaan Masuk Gagal", "gpp_bad", "ev-bad"),
        "sandi_diubah" => ("Kata Sandi Diperbarui", "key", "ev-info"),
        "lupa_sandi" => ("Permintaan Lupa Sandi", "lock_reset", "ev-warn"),
        "nomor_diubah" => ("Nomor WhatsApp Diganti", "phonelink_setup", "ev-info"),
        "sesi_dicabut" => ("Perangkat Dikeluarkan", "logout", "ev-warn"),
        "keluar_semua" => ("Keluar dari Semua Perangkat Lain", "devices", "ev-warn"),
        "akun_dibekukan" => ("Akun Dibekukan", "ac_unit", "ev-bad"),
        "akun_dipulihkan" => ("Akun Dipulihkan", "lock_open", "ev-ok"),
        _ => ("Aktivitas Akun", "info", "ev-info"),
    }
}

#[component]
pub fn SecurityPage() -> impl IntoView {
    let overview = Resource::new(|| (), |_| get_security_overview());
    let cs = Resource::new(|| (), |_| get_cs_contact());
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let error = RwSignal::new(String::new());
    let ok_msg = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let confirm_freeze = RwSignal::new(false);

    let revoke = move |id: String| {
        error.set(String::new());
        spawn_client(async move {
            match revoke_device(id).await {
                Ok(()) => {
                    ok_msg.set("Perangkat berhasil dikeluarkan.".into());
                    overview.refetch();
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
        });
    };
    let logout_others = move |_| {
        error.set(String::new());
        busy.set(true);
        spawn_client(async move {
            match logout_other_devices().await {
                Ok(()) => {
                    ok_msg.set("Semua perangkat lain sudah dikeluarkan. Perangkat ini tetap masuk.".into());
                    overview.refetch();
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };
    let navigate = use_navigate();
    let freeze = move |_| {
        if !confirm_freeze.get_untracked() {
            confirm_freeze.set(true);
            return;
        }
        busy.set(true);
        let navigate = navigate.clone();
        spawn_client(async move {
            match freeze_account().await {
                Ok(()) => {
                    session.refetch();
                    navigate("/login", Default::default());
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="page page-narrow">
            <AccountTabs active="keamanan" />
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    overview
                        .get()
                        .map(|r| match r {
                            Err(e) => view! { <p class="alert alert-error">{clean_error(&e.to_string())}</p> }.into_any(),
                            Ok(o) => view! { <SecurityBody o=o revoke=revoke /> }.into_any(),
                        })
                }}
            </Suspense>

            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
            {move || {
                (!ok_msg.get().is_empty())
                    .then(|| {
                        view! {
                            <p class="alert alert-ok">
                                <Icon name="check_circle" filled=true />
                                {ok_msg.get()}
                            </p>
                        }
                    })
            }}

            <button type="button" class="btn btn-danger-ghost btn-block" on:click=logout_others disabled=move || busy.get()>
                <Icon name="phonelink_erase" />
                "Keluar dari Semua Perangkat Lain"
            </button>

            <section class="lost-card">
                <div class="lost-head">
                    <span class="lost-icon">
                        <Icon name="gpp_maybe" filled=true />
                    </span>
                    <div>
                        <strong>"Ponsel Anda Hilang atau Dicuri?"</strong>
                        <p>
                            "Bekukan akun LajuBus Anda untuk mengamankan tiket perjalanan. Semua perangkat dikeluarkan; buka kembali lewat \"Lupa password?\" — sandi baru dikirim ke WhatsApp terdaftar."
                        </p>
                    </div>
                </div>
                <div class="lost-actions">
                    <button type="button" class="btn btn-freeze" on:click=freeze disabled=move || busy.get()>
                        <Icon name="lock" />
                        {move || if confirm_freeze.get() { "Ya, Bekukan Sekarang" } else { "Bekukan Akun Sementara" }}
                    </button>
                    <Suspense fallback=|| ()>
                        {move || {
                            let c = cs.get().and_then(|r| r.ok()).unwrap_or_default();
                            (!c.is_empty())
                                .then(|| {
                                    view! {
                                        <a
                                            class="btn btn-soft"
                                            href=wa_link(&c, "Halo CS LajuBus 👋 Saya butuh bantuan keamanan akun (ponsel hilang).")
                                            target="_blank"
                                            rel="noopener"
                                        >
                                            "Hubungi CS 24 Jam"
                                        </a>
                                    }
                                })
                        }}
                    </Suspense>
                </div>
                {move || {
                    confirm_freeze
                        .get()
                        .then(|| {
                            view! {
                                <p class="freeze-warn">
                                    "Anda akan keluar dari perangkat ini juga. Tekan sekali lagi untuk membekukan, atau "
                                    <button type="button" class="link-btn" on:click=move |_| confirm_freeze.set(false)>
                                        "batal"
                                    </button>
                                    "."
                                </p>
                            }
                        })
                }}
            </section>
        </div>
    }
}

#[component]
fn SecurityBody(o: SecurityOverview, #[prop(into)] revoke: Callback<String>) -> impl IntoView {
    let (badge, badge_cls) = match o.skor {
        90.. => ("Terproteksi Maksimal", "sec-badge"),
        70..=89 => ("Terproteksi Baik", "sec-badge"),
        _ => ("Perlu Perhatian", "sec-badge warn"),
    };
    // Saran dari faktor dengan kekurangan poin terbesar.
    let tip = o
        .faktor
        .iter()
        .filter(|f| f.poin < f.maks)
        .max_by_key(|f| f.maks - f.poin)
        .map(|f| format!("Tingkatkan: {} (+{} poin).", f.label, f.maks - f.poin))
        .unwrap_or_else(|| "Semua faktor keamanan terpenuhi. Pertahankan dengan memperbarui kata sandi berkala.".into());
    let pct = o.skor.clamp(0, 100);
    let sesi_n = o.sesi.len();
    let umur = {
        let r = relatif(&o.password_changed_at);
        if r == "Aktif sekarang" { "Baru saja".to_string() } else { r }
    };

    view! {
        <section class="sec-hero">
            <div class="sec-hero-top">
                <div>
                    <span class=badge_cls>
                        <Icon name="verified_user" />
                        {badge}
                    </span>
                    <h1>"Status Keamanan"</h1>
                    <p>"Skor dihitung dari kondisi akun Anda yang sebenarnya — bukan angka hiasan."</p>
                </div>
                <div class="score-ring" style=format!("--pct:{pct}")>
                    <strong>{o.skor}</strong>
                    <small>"/100"</small>
                </div>
            </div>
            <ul class="factor-list">
                {o
                    .faktor
                    .iter()
                    .map(|f| {
                        let full = f.poin >= f.maks;
                        view! {
                            <li class=if full { "ok" } else { "" }>
                                <Icon name=if full { "check_circle" } else { "error" } />
                                <span>{f.label.clone()}</span>
                                <b>{format!("{}/{}", f.poin, f.maks)}</b>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <p class="sec-tip">
                <Icon name="tips_and_updates" />
                {tip}
            </p>
        </section>

        <div class="section-head">
            <h2>
                <Icon name="encrypted" />
                "Autentikasi & Kredensial"
            </h2>
            <span class="label-caps">"Wajib rutin cek"</span>
        </div>
        <section class="card cred-list">
            <div class="cred">
                <span class="why-icon w-blue">
                    <Icon name="password" />
                </span>
                <div>
                    <strong>"Kata Sandi Akun"</strong>
                    <small>{format!("Terakhir diubah {}", umur.to_lowercase())}</small>
                </div>
                <a href="/akun/password" class="btn btn-cta btn-sm">
                    "Ubah"
                    <Icon name="arrow_forward" />
                </a>
            </div>
            <div class="cred">
                <span class="why-icon w-green">
                    <Icon name="mark_chat_read" />
                </span>
                <div>
                    <strong>
                        "Nomor WhatsApp"
                        <span class="pill pill-ok">"Terverifikasi"</span>
                    </strong>
                    <small>{format!("+{} · untuk OTP & reset sandi", o.phone)}</small>
                </div>
                <a href="/akun" class="btn btn-soft btn-sm">
                    "Ganti"
                </a>
            </div>
        </section>

        <div class="section-head">
            <h2>
                "Perangkat & Sesi"
                <span class="pill pill-primary">{format!("{sesi_n} Aktif")}</span>
            </h2>
        </div>
        <section class="card device-list">
            {if o.sesi.is_empty() {
                view! {
                    <p class="note-card">
                        <Icon name="info" />
                        "Sesi perangkat mulai tercatat sejak login berikutnya."
                    </p>
                }
                    .into_any()
            } else {
                o.sesi
                    .into_iter()
                    .map(|s| view! { <DeviceRow s=s revoke=revoke /> })
                    .collect_view()
                    .into_any()
            }}
        </section>

        <div class="section-head">
            <h2>
                <Icon name="history" />
                "Riwayat Aktivitas Keamanan"
            </h2>
        </div>
        <section class="card event-list">
            {if o.riwayat.is_empty() {
                view! { <p class="empty">"Belum ada aktivitas tercatat."</p> }.into_any()
            } else {
                o.riwayat.into_iter().map(|e| view! { <EventRow e=e /> }).collect_view().into_any()
            }}
            {if o.login_gagal_30h == 0 {
                view! {
                    <p class="note-card">
                        <Icon name="shield" />
                        "Tidak ditemukan percobaan login gagal dalam 30 hari terakhir."
                    </p>
                }
                    .into_any()
            } else {
                view! {
                    <p class="alert alert-warn">
                        <Icon name="warning" />
                        {format!(
                            "Ada {} percobaan login gagal dalam 30 hari terakhir. Bila bukan Anda, ganti kata sandi dan keluarkan perangkat lain.",
                            o.login_gagal_30h,
                        )}
                    </p>
                }
                    .into_any()
            }}
        </section>
    }
}

#[component]
fn DeviceRow(s: SessionInfo, #[prop(into)] revoke: Callback<String>) -> impl IntoView {
    let id = s.id.clone();
    view! {
        <div class=if s.saat_ini { "device current" } else { "device" }>
            <span class="device-icon">
                <Icon name=if s.mobile { "smartphone" } else { "laptop" } />
            </span>
            <div class="device-main">
                <strong>{s.perangkat.clone()}</strong>
                {s.saat_ini.then(|| view! { <span class="pill pill-ok">"Perangkat Ini"</span> })}
                <small>
                    {if s.saat_ini { "● ".to_string() } else { String::new() }}
                    {format!("{} · {}", s.ip, relatif(&s.last_seen_at))}
                </small>
            </div>
            {if s.saat_ini {
                view! {
                    <span class="device-ok">
                        <Icon name="check_circle" />
                    </span>
                }
                    .into_any()
            } else {
                view! {
                    <button
                        type="button"
                        class="icon-btn"
                        title="Keluarkan perangkat ini"
                        on:click=move |_| revoke.run(id.clone())
                    >
                        <Icon name="logout" />
                    </button>
                }
                    .into_any()
            }}
        </div>
    }
}

#[component]
fn EventRow(e: SecurityEvent) -> impl IntoView {
    let (judul, icon, cls) = event_label(&e.jenis);
    view! {
        <div class="event">
            <span class=format!("event-icon {cls}")>
                <Icon name=icon />
            </span>
            <div class="event-main">
                <strong>{judul}</strong>
                <small>{format!("{} · {}", e.perangkat, e.ip)}</small>
            </div>
            <span class="event-time">{waktu_event(&e.created_at)}</span>
        </div>
    }
}
