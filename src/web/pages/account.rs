//! web/pages/account.rs — "/akun": profil + ganti nomor HP (OTP ke nomor baru).

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::web::api::{logout_user, request_phone_change, verify_phone_change};
use crate::web::app::SessionResource;
use crate::web::components::{clean_error, initials, spawn_client, Icon, PageHead};

fn role_label(role: &str) -> &'static str {
    match role {
        "admin" => "Admin",
        "merchant" => "Mitra PO",
        _ => "Penumpang",
    }
}

fn home_for(role: &str) -> (&'static str, &'static str, &'static str) {
    match role {
        "admin" => ("/admin", "Dashboard Admin", "admin_panel_settings"),
        "merchant" => ("/merchant", "Dashboard Mitra", "storefront"),
        _ => ("/orders", "Tiket Saya", "confirmation_number"),
    }
}

#[component]
pub fn AccountPage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let navigate = use_navigate();
    let user = move || session.get().and_then(|r| r.ok()).flatten();

    let new_phone = RwSignal::new(String::new());
    let otp = RwSignal::new(String::new());
    let otp_sent = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let ok_msg = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let request_otp = move |_| {
        let p = new_phone.get_untracked();
        if p.trim().is_empty() {
            error.set("Nomor HP baru wajib diisi".into());
            return;
        }
        error.set(String::new());
        ok_msg.set(String::new());
        busy.set(true);
        spawn_client(async move {
            match request_phone_change(p).await {
                Ok(_) => otp_sent.set(true),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let confirm_otp = move |_| {
        let code = otp.get_untracked();
        if code.trim().is_empty() {
            error.set("Kode OTP wajib diisi".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        spawn_client(async move {
            match verify_phone_change(code).await {
                Ok(u) => {
                    ok_msg.set(format!("Nomor HP berhasil diganti ke {}", u.phone));
                    otp_sent.set(false);
                    new_phone.set(String::new());
                    otp.set(String::new());
                    session.refetch();
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let do_logout = move |_| {
        let navigate = navigate.clone();
        spawn_client(async move {
            let _ = logout_user().await;
            session.refetch();
            navigate("/", Default::default());
        });
    };

    view! {
        <div class="page">
            <PageHead title="Akun Saya" subtitle="Profil & keamanan akun" />
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    user()
                        .map(|u| {
                            let (href, label, icon) = home_for(&u.role);
                            view! {
                                <section class="profile-hero">
                                    <span class="avatar avatar-lg">{initials(&u.name)}</span>
                                    <div>
                                        <h2>{u.name.clone()}</h2>
                                        <p>
                                            <Icon name="smartphone" />
                                            {u.phone.clone()}
                                        </p>
                                    </div>
                                    <span class="pill pill-glass">
                                        <Icon name="verified_user" filled=true />
                                        {role_label(&u.role)}
                                    </span>
                                </section>
                                <nav class="card menu-list">
                                    <a href=href class="menu-item">
                                        <span class="menu-icon">
                                            <Icon name=icon />
                                        </span>
                                        <span>{label}</span>
                                        <Icon name="chevron_right" />
                                    </a>
                                    <a href="/" class="menu-item">
                                        <span class="menu-icon">
                                            <Icon name="search" />
                                        </span>
                                        <span>"Cari Jadwal Bis"</span>
                                        <Icon name="chevron_right" />
                                    </a>
                                </nav>
                            }
                        })
                }}
            </Suspense>

            <section class="card form-card">
                <div class="step-head">
                    <span class="step-icon">
                        <Icon name="phonelink_setup" />
                    </span>
                    <div>
                        <h3>"Ganti Nomor HP"</h3>
                        <p>"Kode OTP dikirim ke nomor BARU lewat WhatsApp untuk membuktikan nomor itu milik Anda."</p>
                    </div>
                </div>
                {move || {
                    if otp_sent.get() {
                        view! {
                            <label class="field">
                                <span class="field-label">"Kode OTP dari nomor baru"</span>
                                <input
                                    type="text"
                                    class="otp-input"
                                    inputmode="numeric"
                                    maxlength="6"
                                    placeholder="••••••"
                                    prop:value=move || otp.get()
                                    on:input=move |ev| otp.set(event_target_value(&ev))
                                />
                            </label>
                            <button
                                type="button"
                                class="btn btn-cta btn-block"
                                on:click=confirm_otp
                                disabled=move || busy.get()
                            >
                                {move || if busy.get() { "Memverifikasi…" } else { "Konfirmasi" }}
                            </button>
                            <button type="button" class="btn btn-ghost btn-block" on:click=move |_| otp_sent.set(false)>
                                "Ganti nomor tujuan"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! {
                            <label class="field">
                                <span class="field-label">"No. HP Baru"</span>
                                <span class="input-wrap">
                                    <Icon name="smartphone" />
                                    <input
                                        type="tel"
                                        placeholder="08xx"
                                        prop:value=move || new_phone.get()
                                        on:input=move |ev| new_phone.set(event_target_value(&ev))
                                    />
                                </span>
                            </label>
                            <button
                                type="button"
                                class="btn btn-primary btn-block"
                                on:click=request_otp
                                disabled=move || busy.get()
                            >
                                <Icon name="chat" />
                                {move || if busy.get() { "Mengirim…" } else { "Kirim OTP" }}
                            </button>
                        }
                            .into_any()
                    }
                }}
                {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
                {move || (!ok_msg.get().is_empty()).then(|| view! { <p class="alert alert-ok">{ok_msg.get()}</p> })}
            </section>

            <button type="button" class="btn btn-danger-ghost btn-block" on:click=do_logout>
                <Icon name="logout" />
                "Keluar dari Akun"
            </button>
        </div>
    }
}
