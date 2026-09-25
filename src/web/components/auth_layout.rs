//! Tata letak halaman autentikasi (masuk / daftar / OTP / lupa password),
//! mengikuti layar "Login Multi Role": logo, judul, tab, kartu form, kartu
//! bantuan, dan baris kepercayaan di bawah.

use leptos::prelude::*;

use super::{BrandLogo, Icon};

#[derive(Clone, Copy, PartialEq)]
pub enum AuthTab {
    Masuk,
    Daftar,
    None,
}

#[component]
pub fn AuthLayout(
    #[prop(into)] title: String,
    #[prop(into)] subtitle: String,
    #[prop(optional)] tab: Option<AuthTab>,
    children: Children,
) -> impl IntoView {
    let tab = tab.unwrap_or(AuthTab::None);
    view! {
        <div class="auth-page">
            <div class="auth-logo">
                <BrandLogo large=true />
            </div>
            <h1 class="auth-title">{title}</h1>
            <p class="auth-sub">{subtitle}</p>

            {(tab != AuthTab::None)
                .then(|| {
                    view! {
                        <nav class="segmented">
                            <a href="/login" class=if tab == AuthTab::Masuk { "seg active" } else { "seg" }>
                                <Icon name="login" />
                                "Masuk"
                            </a>
                            <a href="/register" class=if tab == AuthTab::Daftar { "seg active" } else { "seg" }>
                                <Icon name="person_add" />
                                "Daftar"
                            </a>
                        </nav>
                    }
                })}

            <section class="card auth-card">{children()}</section>

            <div class="help-card">
                <span class="help-icon">
                    <Icon name="support_agent" />
                </span>
                <div>
                    <strong>"Kendala saat masuk?"</strong>
                    <p>"Kode OTP & password baru dikirim lewat WhatsApp ke nomor terdaftar."</p>
                </div>
            </div>

            <p class="trust-row">
                <span>
                    <Icon name="lock" />
                    "Sesi terenkripsi"
                </span>
                <span class="sep"></span>
                <span>
                    <Icon name="verified" />
                    "Tiket resmi PO mitra"
                </span>
            </p>
            <p class="copyright">"© 2026 LajuBus Indonesia. Seluruh hak cipta dilindungi."</p>
        </div>
    }
}
