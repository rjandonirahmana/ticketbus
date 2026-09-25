use leptos::prelude::*;

use crate::web::api::forgot_password;
use crate::web::components::{clean_error, spawn_client, AuthLayout, Icon};

#[component]
pub fn ForgotPasswordPage() -> impl IntoView {
    let phone = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let sent = RwSignal::new(false);
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let p = phone.get_untracked();
        if p.trim().is_empty() {
            error.set("Nomor HP wajib diisi".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        spawn_client(async move {
            match forgot_password(p).await {
                Ok(_) => sent.set(true),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <AuthLayout title="Lupa Password" subtitle="Password baru akan dikirim lewat WhatsApp.">
            {move || {
                if sent.get() {
                    // Kalimat sengaja sama untuk nomor terdaftar maupun
                    // tidak — form ini tak boleh membocorkan siapa punya akun.
                    view! {
                        <p class="alert alert-ok">
                            "Jika nomor tersebut terdaftar, password baru sudah dikirim lewat WhatsApp. \
                             Password lama tetap berlaku sampai password baru dipakai login."
                        </p>
                        <a href="/login" class="btn btn-primary btn-block">
                            <Icon name="arrow_back" />
                            "Kembali ke halaman masuk"
                        </a>
                    }
                        .into_any()
                } else {
                    view! {
                        <label class="field">
                            <span class="field-label">"Nomor WhatsApp Terdaftar"</span>
                            <span class="input-wrap">
                                <Icon name="smartphone" />
                                <input
                                    type="tel"
                                    placeholder="Contoh: 081234567890"
                                    prop:value=move || phone.get()
                                    on:input=move |ev| phone.set(event_target_value(&ev))
                                />
                            </span>
                        </label>
                        <button
                            type="button"
                            class="btn btn-cta btn-block btn-lg"
                            on:click=submit
                            disabled=move || busy.get()
                        >
                            <Icon name="key" />
                            {move || if busy.get() { "Mengirim…" } else { "Kirim Password Baru" }}
                        </button>
                        {move || {
                            (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })
                        }}
                        <p class="auth-switch">"Ingat password? " <a href="/login">"Masuk"</a></p>
                    }
                        .into_any()
                }
            }}
        </AuthLayout>
    }
}
