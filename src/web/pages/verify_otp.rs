use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::web::api::{register_verify, resend_otp};
use crate::web::app::SessionResource;
use crate::web::components::{clean_error, spawn_client, AuthLayout, Icon};

#[component]
pub fn VerifyOtpPage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let navigate = use_navigate();
    let query = use_query_map();
    let phone = move || query.get().get("phone").unwrap_or_default();

    let otp = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let info = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let p = phone();
        let code = otp.get_untracked();
        if p.is_empty() {
            error.set("Nomor HP tidak diketahui — ulangi pendaftaran".into());
            return;
        }
        if code.trim().is_empty() {
            error.set("Kode OTP wajib diisi".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        let navigate = navigate.clone();
        spawn_client(async move {
            match register_verify(p, code).await {
                Ok(user) => {
                    let dest = if user.role == "merchant" { "/merchant" } else { "/" };
                    // Sama dengan login: set sesi sebelum navigasi agar guard tak membaca sesi lama.
                    session.set(Some(Ok(Some(user))));
                    navigate(dest, Default::default());
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let resend = move |_| {
        let p = phone();
        if p.is_empty() {
            return;
        }
        info.set(String::new());
        error.set(String::new());
        spawn_client(async move {
            match resend_otp(p).await {
                Ok(_) => info.set("Kode baru sudah dikirim".into()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
        });
    };

    view! {
        <AuthLayout title="Verifikasi OTP" subtitle="Masukkan 6 digit kode yang kami kirim lewat WhatsApp.">
            <p class="info-banner">
                <Icon name="chat" />
                {move || format!("Kode dikirim ke WhatsApp {}", phone())}
            </p>
            <label class="field">
                <span class="field-label">"Kode OTP"</span>
                <input
                    type="text"
                    class="otp-input"
                    inputmode="numeric"
                    autocomplete="one-time-code"
                    maxlength="6"
                    placeholder="••••••"
                    prop:value=move || otp.get()
                    on:input=move |ev| otp.set(event_target_value(&ev))
                />
            </label>
            <button type="button" class="btn btn-cta btn-block btn-lg" on:click=submit disabled=move || busy.get()>
                <Icon name="verified_user" />
                {move || if busy.get() { "Memverifikasi…" } else { "Verifikasi" }}
            </button>
            <button type="button" class="btn btn-ghost btn-block" on:click=resend>
                <Icon name="refresh" />
                "Kirim ulang kode"
            </button>
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
            {move || (!info.get().is_empty()).then(|| view! { <p class="alert alert-ok">{info.get()}</p> })}
        </AuthLayout>
    }
}
