use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::web::api::login_user;
use crate::web::app::SessionResource;
use crate::web::components::{clean_error, spawn_client, AuthLayout, AuthTab, Icon};

#[component]
pub fn LoginPage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let navigate = use_navigate();

    let phone = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let show_pw = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let p = phone.get_untracked();
        let pw = password.get_untracked();
        if p.trim().is_empty() || pw.is_empty() {
            error.set("Nomor HP dan password wajib diisi".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        let navigate = navigate.clone();
        spawn_client(async move {
            match login_user(p, pw).await {
                Ok(user) => {
                    session.refetch();
                    let dest = match user.role.as_str() {
                        "merchant" => "/merchant",
                        "admin" => "/admin",
                        _ => "/",
                    };
                    navigate(dest, Default::default());
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <AuthLayout
            title="Selamat Datang di LajuBus"
            subtitle="Masuk ke ekosistem transportasi bus antarkota modern & terpercaya."
            tab=AuthTab::Masuk
        >
            <p class="info-banner">
                <Icon name="bolt" />
                "Satu form untuk Penumpang, Mitra PO, dan Admin — dashboard menyesuaikan peran akun."
            </p>
            <label class="field">
                <span class="field-label">"Nomor WhatsApp"</span>
                <span class="input-wrap">
                    <Icon name="smartphone" />
                    <input
                        type="tel"
                        autocomplete="tel"
                        placeholder="Contoh: 081234567890"
                        prop:value=move || phone.get()
                        on:input=move |ev| phone.set(event_target_value(&ev))
                    />
                </span>
            </label>
            <label class="field">
                <span class="field-label">"Password"</span>
                <span class="input-wrap">
                    <Icon name="lock" />
                    <input
                        type=move || if show_pw.get() { "text" } else { "password" }
                        autocomplete="current-password"
                        placeholder="Password akun"
                        prop:value=move || password.get()
                        on:input=move |ev| password.set(event_target_value(&ev))
                    />
                    <button
                        type="button"
                        class="input-action"
                        title="Tampilkan password"
                        on:click=move |_| show_pw.update(|v| *v = !*v)
                    >
                        {move || {
                            if show_pw.get() {
                                view! { <Icon name="visibility_off" /> }.into_any()
                            } else {
                                view! { <Icon name="visibility" /> }.into_any()
                            }
                        }}
                    </button>
                </span>
            </label>
            <a href="/lupa-password" class="link-right">"Lupa password?"</a>
            <button type="button" class="btn btn-cta btn-block btn-lg" on:click=submit disabled=move || busy.get()>
                <Icon name="login" />
                {move || if busy.get() { "Masuk…" } else { "Masuk" }}
            </button>
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
            <div class="divider">
                <span>"ATAU"</span>
            </div>
            <a href="/register" class="btn btn-soft btn-block">
                <Icon name="chat" />
                "Daftar akun baru via OTP WhatsApp"
            </a>
        </AuthLayout>
    }
}
