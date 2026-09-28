//! web/pages/merchant_register.rs — "/daftar-mitra": pendaftaran Mitra PO.
//! Data PO + penanggung jawab → OTP WhatsApp (/verify-otp) → akun mitra
//! berstatus "Menunggu Verifikasi" sampai disetujui admin. Pratinjau profil
//! PO berubah langsung mengikuti isian form.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::web::api::register_merchant;
use crate::web::components::{clean_error, spawn_client, Icon, PoCard, ProfileFields, ProfileSignals, TextField};

#[component]
pub fn MerchantRegisterPage() -> impl IntoView {
    let navigate = use_navigate();
    let sig = ProfileSignals::new(None);
    let pic = RwSignal::new(String::new());
    let phone = RwSignal::new(String::new());
    let setuju = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let (n, p) = (pic.get_untracked(), phone.get_untracked());
        if n.trim().is_empty() || p.trim().is_empty() {
            error.set("Nama penanggung jawab dan no. WhatsApp wajib diisi".into());
            return;
        }
        if !setuju.get_untracked() {
            error.set("Centang persetujuan syarat Mitra PO dulu".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        let navigate = navigate.clone();
        let phone_digits: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
        let input = sig.input();
        spawn_client(async move {
            match register_merchant(p, n, input).await {
                Ok(()) => navigate(&format!("/verify-otp?phone={phone_digits}"), Default::default()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let langkah = [
        ("edit_note", "Isi data PO", "Profil, layanan & kontak penanggung jawab"),
        ("chat", "Verifikasi WhatsApp", "Kode OTP + password akun dikirim ke WA"),
        ("fact_check", "Review admin", "Biasanya 1×24 jam kerja"),
        ("rocket_launch", "Jadwal tayang", "Trayek & tiket tampil di beranda"),
    ];

    view! {
        <div class="page mitra-page">
            <section class="mitra-hero">
                <span class="pill pill-glass">
                    <Icon name="storefront" />
                    "Program Mitra PO"
                </span>
                <h1>"Jual tiket bus Anda di LajuBus"</h1>
                <p>"Kelola trayek tetap, armada, driver harian, dan kursi bernomor — penumpang memesan langsung dari beranda."</p>
                <div class="mitra-benefits">
                    <span>
                        <Icon name="event_repeat" />
                        "Trayek otomatis tiap hari"
                    </span>
                    <span>
                        <Icon name="near_me" />
                        "Pelacakan GPS driver"
                    </span>
                    <span>
                        <Icon name="payments" />
                        "Tanpa biaya pendaftaran"
                    </span>
                </div>
            </section>

            <ol class="mitra-steps">
                {langkah
                    .into_iter()
                    .enumerate()
                    .map(|(i, (icon, t, s))| {
                        view! {
                            <li>
                                <span class="mitra-step-icon">
                                    <Icon name=icon />
                                </span>
                                <span>
                                    <strong>{format!("{}. {t}", i + 1)}</strong>
                                    <small>{s}</small>
                                </span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ol>

            <div class="mitra-grid">
                <section class="card form-card">
                    <div class="step-head">
                        <span class="step-icon">
                            <Icon name="directions_bus" />
                        </span>
                        <div>
                            <h3>"Data PO"</h3>
                            <p>"Logo, sampul & dokumen izin bisa diunggah setelah akun jadi"</p>
                        </div>
                    </div>
                    <ProfileFields sig=sig />

                    <h4 class="form-sub">
                        <Icon name="person" />
                        "Akun Penanggung Jawab"
                    </h4>
                    <div class="field-grid">
                        <TextField label="Nama Penanggung Jawab" icon="person" placeholder="Nama lengkap" value=pic />
                        <label class="field">
                            <span class="field-label">"No. WhatsApp Aktif"</span>
                            <span class="input-wrap">
                                <Icon name="smartphone" />
                                <input
                                    type="tel"
                                    autocomplete="tel"
                                    placeholder="081234567890"
                                    prop:value=move || phone.get()
                                    on:input=move |ev| phone.set(event_target_value(&ev))
                                />
                            </span>
                        </label>
                    </div>
                    <label class="check-row">
                        <input type="checkbox" prop:checked=move || setuju.get() on:change=move |ev| setuju.set(event_target_checked(&ev)) />
                        <span>
                            <strong>"Data yang saya isi benar"</strong>
                            <small>"PO saya memiliki izin usaha yang sah dan bersedia diverifikasi admin LajuBus."</small>
                        </span>
                    </label>
                    <button type="button" class="btn btn-cta btn-block btn-lg" on:click=submit disabled=move || busy.get()>
                        <Icon name="chat" />
                        {move || if busy.get() { "Mengirim OTP…" } else { "Daftar & Kirim OTP WhatsApp" }}
                    </button>
                    {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
                    <p class="auth-switch">"Sudah punya akun mitra? " <a href="/login">"Masuk"</a></p>
                </section>

                <aside class="mitra-preview">
                    <span class="field-label">"Pratinjau profil PO di LajuBus"</span>
                    {move || view! { <PoCard profil=sig.preview("menunggu") show_status=true /> }}
                    <p class="field-hint">
                        "Setelah disetujui, profil ini tampil di halaman PO publik dan di setiap kartu jadwal Anda."
                    </p>
                </aside>
            </div>
        </div>
    }
}
