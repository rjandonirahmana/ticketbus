//! web/pages/change_password.rs — "/akun/password": ganti kata sandi dengan
//! meter kekuatan, daftar syarat (SAMA dengan `validate_new_password` di
//! service/auth.rs), dan opsi "keluarkan dari semua perangkat lain".

use leptos::prelude::*;

use crate::web::api::{change_password, get_cs_contact, password_changed_at};
use crate::web::components::{clean_error, format_tanggal, spawn_client, wa_link, Icon, PageHead};

/// Syarat kata sandi baru: (label, wajib?, terpenuhi?).
fn syarat(p: &str) -> [(&'static str, bool, bool); 4] {
    [
        ("Minimal 8 karakter", true, p.chars().count() >= 8),
        (
            "Mengandung huruf besar (A-Z) & kecil (a-z)",
            true,
            p.chars().any(|c| c.is_uppercase()) && p.chars().any(|c| c.is_lowercase()),
        ),
        ("Mengandung minimal 1 angka (0-9)", true, p.chars().any(|c| c.is_ascii_digit())),
        (
            "Mengandung simbol khusus (@, #, $, dll) — disarankan",
            false,
            p.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace()),
        ),
    ]
}

/// "2026-09-15T…" → "11 hari lalu (15 Sep 2026)".
fn terakhir_diubah(rfc3339: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(rfc3339) else {
        return "-".into();
    };
    let tanggal = format_tanggal(&t.format("%Y-%m-%d").to_string());
    let hari = (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days();
    let rel = match hari {
        ..=0 => "Hari ini".to_string(),
        1 => "Kemarin".to_string(),
        n => format!("{n} hari lalu"),
    };
    format!("{rel} ({tanggal})")
}

/// Input kata sandi berikon dengan tombol tampil/sembunyi.
#[component]
fn PasswordInput(
    value: RwSignal<String>,
    #[prop(into)] icon: String,
    #[prop(into)] placeholder: String,
    #[prop(into)] autocomplete: String,
) -> impl IntoView {
    let show = RwSignal::new(false);
    view! {
        <span class="input-wrap">
            <Icon name=icon />
            <input
                type=move || if show.get() { "text" } else { "password" }
                autocomplete=autocomplete
                placeholder=placeholder
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
            <button
                type="button"
                class="input-action"
                title=move || if show.get() { "Sembunyikan" } else { "Tampilkan" }
                on:click=move |_| show.update(|v| *v = !*v)
            >
                {move || {
                    if show.get() {
                        view! { <Icon name="visibility" /> }.into_any()
                    } else {
                        view! { <Icon name="visibility_off" /> }.into_any()
                    }
                }}
            </button>
        </span>
    }
}

#[component]
pub fn ChangePasswordPage() -> impl IntoView {
    let changed = Resource::new(|| (), |_| password_changed_at());
    let cs = Resource::new(|| (), |_| get_cs_contact());

    let current = RwSignal::new(String::new());
    let baru = RwSignal::new(String::new());
    let konfirmasi = RwSignal::new(String::new());
    let logout_others = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let ok_msg = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let skor = move || syarat(&baru.get()).iter().filter(|s| s.2).count();
    let wajib_ok = move || syarat(&baru.get()).iter().all(|s| !s.1 || s.2);
    let cocok = move || !konfirmasi.get().is_empty() && konfirmasi.get() == baru.get();
    let bisa_simpan = move || !current.get().is_empty() && wajib_ok() && cocok() && !busy.get();

    let submit = move |_| {
        if !bisa_simpan() {
            return;
        }
        error.set(String::new());
        ok_msg.set(String::new());
        busy.set(true);
        let (c, n, lo) = (current.get_untracked(), baru.get_untracked(), logout_others.get_untracked());
        spawn_client(async move {
            match change_password(c, n, lo).await {
                Ok(()) => {
                    current.set(String::new());
                    baru.set(String::new());
                    konfirmasi.set(String::new());
                    ok_msg.set(if lo {
                        "Kata sandi berhasil diperbarui. Semua perangkat lain sudah dikeluarkan.".into()
                    } else {
                        "Kata sandi berhasil diperbarui.".into()
                    });
                    changed.refetch();
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="page page-narrow">
            <PageHead title="Ganti Kata Sandi" back="/akun" />

            <section class="card security-hero">
                <div class="security-top">
                    <span class="security-shield">
                        <Icon name="shield_lock" filled=true />
                    </span>
                    <div>
                        <span class="label-caps kicker-orange">
                            "Keamanan Akun" <i class="status-dot"></i>
                        </span>
                        <h1>"Perbarui Kata Sandi Anda"</h1>
                        <p>
                            "Buat kata sandi yang kuat dan unik minimal 8 karakter untuk melindungi akun tiket & transaksi Anda di LajuBus."
                        </p>
                    </div>
                </div>
                <div class="security-meta">
                    <span>
                        <Icon name="history" />
                        "Terakhir diubah:"
                    </span>
                    <Suspense fallback=|| view! { <strong>"…"</strong> }>
                        <strong>
                            {move || changed.get().and_then(|r| r.ok()).map(|t| terakhir_diubah(&t)).unwrap_or_default()}
                        </strong>
                    </Suspense>
                </div>
            </section>

            <section class="card pw-card">
                <div class="pw-head">
                    <span>"Kata Sandi Saat Ini"</span>
                    <a href="/lupa-password" class="link-accent">
                        "Lupa kata sandi?"
                    </a>
                </div>
                <PasswordInput value=current icon="lock" placeholder="Kata sandi saat ini" autocomplete="current-password" />
            </section>

            <section class="card pw-card">
                <div class="pw-head">
                    <span>"Kata Sandi Baru"</span>
                </div>
                <PasswordInput value=baru icon="key" placeholder="Kata sandi baru" autocomplete="new-password" />
                <div class="strength">
                    <div class="strength-top">
                        <span>"Kekuatan Sandi:"</span>
                        {move || {
                            let (label, cls) = match skor() {
                                0 => ("-", "strength-label"),
                                1 => ("Lemah", "strength-label s-weak"),
                                2 => ("Sedang", "strength-label s-mid"),
                                3 => ("Kuat (Strong)", "strength-label s-strong"),
                                _ => ("Sangat Kuat", "strength-label s-strong"),
                            };
                            view! {
                                <strong class=cls>
                                    <Icon name="verified_user" />
                                    {label}
                                </strong>
                            }
                        }}
                    </div>
                    <div class=move || format!("strength-bars lvl-{}", skor())>
                        <i></i>
                        <i></i>
                        <i></i>
                        <i></i>
                    </div>
                </div>
                <span class="label-caps">"Syarat Sandi Baru"</span>
                <ul class="req-list">
                    {move || {
                        syarat(&baru.get())
                            .into_iter()
                            .map(|(label, _, ok)| {
                                view! {
                                    <li class=if ok { "ok" } else { "" }>
                                        <Icon name=if ok { "check_circle" } else { "radio_button_unchecked" } />
                                        {label}
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </section>

            <section class="card pw-card">
                <div class="pw-head">
                    <span>"Konfirmasi Kata Sandi Baru"</span>
                </div>
                <PasswordInput value=konfirmasi icon="lock_reset" placeholder="Ulangi kata sandi baru" autocomplete="new-password" />
                {move || {
                    (!konfirmasi.get().is_empty())
                        .then(|| {
                            if cocok() {
                                view! {
                                    <p class="match ok">
                                        <Icon name="check" />
                                        "Kata sandi cocok"
                                    </p>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <p class="match bad">
                                        <Icon name="close" />
                                        "Kata sandi belum sama"
                                    </p>
                                }
                                    .into_any()
                            }
                        })
                }}
            </section>

            <h2 class="sub-title">"Pengaturan Keamanan Tambahan"</h2>
            <section class="card toggle-card">
                <span class="why-icon w-blue">
                    <Icon name="devices" />
                </span>
                <div>
                    <strong>"Keluarkan dari semua perangkat lain"</strong>
                    <p>"Rekomendasi jika Anda menduga ada akses mencurigakan pada akun Anda."</p>
                </div>
                <button
                    type="button"
                    role="switch"
                    aria-checked=move || logout_others.get().to_string()
                    class=move || if logout_others.get() { "switch on" } else { "switch" }
                    title="Keluarkan dari semua perangkat lain"
                    on:click=move |_| logout_others.update(|v| *v = !*v)
                >
                    <i></i>
                </button>
            </section>

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

            <div class="pw-actions">
                <button type="button" class="btn btn-cta btn-block btn-lg" on:click=submit disabled=move || !bisa_simpan()>
                    <Icon name="save" />
                    {move || if busy.get() { "Menyimpan…" } else { "Simpan Kata Sandi Baru" }}
                </button>
                <a href="/akun" class="btn btn-soft btn-block btn-lg">
                    "Batal"
                </a>
            </div>

            <section class="card tips-card">
                <div class="tips-head">
                    <Icon name="verified" />
                    <div>
                        <strong>"Tips Keamanan LajuBus"</strong>
                        <p>
                            "Pihak LajuBus tidak pernah meminta kata sandi, PIN, atau kode OTP Anda melalui pesan pribadi, telepon, ataupun media sosial. Jangan berikan akses kepada siapa pun."
                        </p>
                    </div>
                </div>
                <div class="tips-foot">
                    <span>"Mengalami masalah akun?"</span>
                    <Suspense fallback=|| ()>
                        {move || {
                            let c = cs.get().and_then(|r| r.ok()).unwrap_or_default();
                            (!c.is_empty())
                                .then(|| {
                                    view! {
                                        <a
                                            href=wa_link(&c, "Halo CS LajuBus 👋 Saya mengalami masalah dengan akun saya.")
                                            target="_blank"
                                            rel="noopener"
                                        >
                                            "Pusat Bantuan"
                                            <Icon name="arrow_forward" />
                                        </a>
                                    }
                                })
                        }}
                    </Suspense>
                </div>
            </section>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Syarat WAJIB di halaman harus identik dengan validasi server.
    #[test]
    fn syarat_ui_sama_dengan_server() {
        for p in ["Rahasia1", "rahasia123", "RAHASIA123", "RahasiaSaja", "Rhs1", "Aman#2026x"] {
            let ui_ok = syarat(p).iter().all(|s| !s.1 || s.2);
            #[cfg(feature = "ssr")]
            assert_eq!(ui_ok, crate::service::auth::validate_new_password(p).is_ok(), "{p}");
            let _ = ui_ok;
        }
    }
}
