use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::web::api::register_request;
use crate::web::components::{clean_error, spawn_client, AuthLayout, AuthTab, Icon};

#[component]
pub fn RegisterPage() -> impl IntoView {
    let navigate = use_navigate();

    let name = RwSignal::new(String::new());
    let phone = RwSignal::new(String::new());
    let role = RwSignal::new("buyer".to_string());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let n = name.get_untracked();
        let p = phone.get_untracked();
        let r = role.get_untracked();
        if n.trim().is_empty() || p.trim().is_empty() {
            error.set("Nama dan no. HP wajib diisi".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        let navigate = navigate.clone();
        // Nomor buat query string diringkas jadi digit saja — cukup untuk
        // dibaca ulang `verify_otp.rs` tanpa perlu URL-encode `+`/spasi.
        let phone_digits: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
        spawn_client(async move {
            match register_request(p, n, r).await {
                Ok(_) => navigate(&format!("/verify-otp?phone={phone_digits}"), Default::default()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let role_opt = move |value: &'static str, icon: &'static str, title: &'static str, sub: &'static str| {
        view! {
            <button
                type="button"
                class=move || if role.get() == value { "role-opt selected" } else { "role-opt" }
                on:click=move |_| role.set(value.to_string())
            >
                <span class="role-icon">
                    <Icon name=icon />
                </span>
                <strong>{title}</strong>
                <small>{sub}</small>
            </button>
        }
    };

    view! {
        <AuthLayout
            title="Buat Akun LajuBus"
            subtitle="Daftar sekali, kode OTP dikirim lewat WhatsApp."
            tab=AuthTab::Daftar
        >
            <span class="field-label">"Daftar sebagai"</span>
            <div class="role-pick">
                {role_opt("buyer", "confirmation_number", "Penumpang", "Cari jadwal & beli tiket")}
                <a href="/daftar-mitra" class="role-opt">
                    <span class="role-icon">
                        <Icon name="storefront" />
                    </span>
                    <strong>"Mitra PO"</strong>
                    <small>"Formulir khusus PO + verifikasi admin"</small>
                </a>
            </div>
            <label class="field">
                <span class="field-label">"Nama Lengkap"</span>
                <span class="input-wrap">
                    <Icon name="person" />
                    <input
                        type="text"
                        autocomplete="name"
                        placeholder="Nama sesuai KTP / nama PO"
                        prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))
                    />
                </span>
            </label>
            <label class="field">
                <span class="field-label">"Nomor WhatsApp Aktif"</span>
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
            <button type="button" class="btn btn-cta btn-block btn-lg" on:click=submit disabled=move || busy.get()>
                <Icon name="chat" />
                {move || if busy.get() { "Mengirim OTP…" } else { "Kirim Kode OTP via WhatsApp" }}
            </button>
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
            <p class="auth-switch">"Sudah punya akun? " <a href="/login">"Masuk"</a></p>
        </AuthLayout>
    }
}
