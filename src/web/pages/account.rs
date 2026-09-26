//! web/pages/account.rs — "/akun": profil pengguna. Semua angka di sini
//! berasal dari data asli (skor keamanan, jumlah perjalanan) — tak ada saldo
//! atau poin hiasan.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::web::api::{get_security_overview, list_my_orders, logout_user, request_phone_change, verify_phone_change};
use crate::web::app::SessionResource;
use crate::web::components::{clean_error, initials, spawn_client, AccountTabs, Icon};

fn role_label(role: &str) -> &'static str {
    match role {
        "admin" => "Administrator LajuBus",
        "merchant" => "Mitra PO Terverifikasi",
        _ => "Penumpang LajuBus",
    }
}

fn role_sub(role: &str) -> &'static str {
    match role {
        "admin" => "Pengelola platform & armada nasional",
        "merchant" => "Pengelola armada, jadwal & sewa bus",
        _ => "Penjelajah antarkota",
    }
}

/// "6281234567890" → "+62 812-3456-7890".
fn format_phone(p: &str) -> String {
    let Some(rest) = p.strip_prefix("62") else { return p.to_string() };
    let d: Vec<char> = rest.chars().collect();
    if d.len() < 9 {
        return format!("+62 {rest}");
    }
    let a: String = d[..3].iter().collect();
    let b: String = d[3..7].iter().collect();
    let c: String = d[7..].iter().collect();
    format!("+62 {a}-{b}-{c}")
}

#[component]
fn MenuItem(
    #[prop(into)] href: String,
    #[prop(into)] icon: String,
    #[prop(into)] title: String,
    #[prop(into)] sub: String,
    #[prop(optional)] badge: Option<AnyView>,
) -> impl IntoView {
    view! {
        <a href=href class="menu-item rich">
            <span class="menu-icon">
                <Icon name=icon />
            </span>
            <span class="menu-text">
                <strong>
                    {title}
                    {badge}
                </strong>
                <small>{sub}</small>
            </span>
            <Icon name="chevron_right" />
        </a>
    }
}

#[component]
pub fn AccountPage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let navigate = use_navigate();
    let user = move || session.get().and_then(|r| r.ok()).flatten();

    let overview = Resource::new(|| (), |_| get_security_overview());
    let is_buyer = move || user().is_some_and(|u| u.role == "buyer");
    let trips = Resource::new(is_buyer, |buyer| async move {
        if buyer {
            list_my_orders().await.map(|v| v.len())
        } else {
            Ok(0)
        }
    });

    // ── Ganti nomor WhatsApp (OTP ke nomor BARU) ────────────────────────────
    let show_phone = RwSignal::new(false);
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
                    ok_msg.set(format!("Nomor WhatsApp berhasil diganti ke {}", format_phone(&u.phone)));
                    otp_sent.set(false);
                    new_phone.set(String::new());
                    otp.set(String::new());
                    session.refetch();
                    overview.refetch();
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
        <div class="page page-narrow">
            <AccountTabs active="profil" />
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    user()
                        .map(|u| {
                            view! {
                                <section class="profile-card">
                                    <div class="profile-top">
                                        <span class="avatar avatar-xl">
                                            {initials(&u.name)}
                                            <i class="avatar-badge">
                                                <Icon name="verified" filled=true />
                                            </i>
                                        </span>
                                        <div>
                                            <span class="member-badge">
                                                <Icon name="workspace_premium" />
                                                {role_label(&u.role)}
                                            </span>
                                            <h1>{u.name.clone()}</h1>
                                            <p>{role_sub(&u.role)}</p>
                                        </div>
                                    </div>
                                    <div class="profile-row">
                                        <Icon name="check_circle" />
                                        <div>
                                            <span class="label-caps">"Nomor WhatsApp Terverifikasi"</span>
                                            <strong>{format_phone(&u.phone)}</strong>
                                        </div>
                                    </div>
                                </section>

                                <div class="profile-stats">
                                    <a href="/akun/keamanan" class="card pstat">
                                        <div class="pstat-top">
                                            <span class="label-caps">
                                                <Icon name="shield" />
                                                "Skor Keamanan"
                                            </span>
                                            <i class="status-dot"></i>
                                        </div>
                                        <strong class="num">
                                            {move || {
                                                overview
                                                    .get()
                                                    .and_then(|r| r.ok())
                                                    .map(|o| o.skor.to_string())
                                                    .unwrap_or_else(|| "–".into())
                                            }}
                                            <small>"/100"</small>
                                        </strong>
                                        <span class="pstat-link">
                                            "Lihat Pusat Keamanan"
                                            <Icon name="chevron_right" />
                                        </span>
                                    </a>
                                    {match u.role.as_str() {
                                        "buyer" => {
                                            view! {
                                                <a href="/orders" class="card pstat">
                                                    <div class="pstat-top">
                                                        <span class="label-caps">
                                                            <Icon name="confirmation_number" />
                                                            "Perjalanan"
                                                        </span>
                                                    </div>
                                                    <strong class="num accent">
                                                        {move || {
                                                            trips
                                                                .get()
                                                                .and_then(|r| r.ok())
                                                                .map(|n| n.to_string())
                                                                .unwrap_or_else(|| "–".into())
                                                        }}
                                                        <small>"tiket"</small>
                                                    </strong>
                                                    <span class="pstat-link">
                                                        "Buka Tiket Saya"
                                                        <Icon name="chevron_right" />
                                                    </span>
                                                </a>
                                            }
                                                .into_any()
                                        }
                                        role => {
                                            let (href, label) = if role == "admin" {
                                                ("/admin", "Control Hub")
                                            } else {
                                                ("/merchant", "Dashboard Mitra")
                                            };
                                            view! {
                                                <a href=href class="card pstat">
                                                    <div class="pstat-top">
                                                        <span class="label-caps">
                                                            <Icon name="dashboard" />
                                                            "Operasional"
                                                        </span>
                                                    </div>
                                                    <strong class="pstat-title">{label}</strong>
                                                    <span class="pstat-link">
                                                        "Buka dashboard"
                                                        <Icon name="chevron_right" />
                                                    </span>
                                                </a>
                                            }
                                                .into_any()
                                        }
                                    }}
                                </div>
                            }
                        })
                }}
            </Suspense>

            <a href="/" class="promo-strip">
                <span class="why-icon w-orange">
                    <Icon name="event_seat" />
                </span>
                <div>
                    <strong>"Pilih Kursi Favoritmu"</strong>
                    <small>"Cari jadwal dan pilih nomor kursi langsung dari denah bus"</small>
                </div>
                <Icon name="chevron_right" />
            </a>

            <span class="label-caps section-label">"Pengaturan Akun & Perjalanan"</span>
            <nav class="card menu-list">
                <MenuItem
                    href="/akun/keamanan"
                    icon="shield_person"
                    title="Keamanan & Kata Sandi"
                    sub="Perangkat aktif, riwayat login, bekukan akun"
                    badge=view! {
                        <Suspense fallback=|| ()>
                            {move || {
                                overview
                                    .get()
                                    .and_then(|r| r.ok())
                                    .map(|o| {
                                        if o.skor >= 80 {
                                            view! {
                                                <span class="pill pill-ok">
                                                    <Icon name="lock" />
                                                    "Aman"
                                                </span>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <span class="pill pill-warn">
                                                    <Icon name="warning" />
                                                    "Perlu dicek"
                                                </span>
                                            }
                                                .into_any()
                                        }
                                    })
                            }}
                        </Suspense>
                    }
                        .into_any()
                />
                <MenuItem
                    href="/akun/password"
                    icon="lock_reset"
                    title="Ganti Kata Sandi"
                    sub="Perbarui sandi secara berkala"
                />
                <button
                    type="button"
                    class="menu-item rich"
                    on:click=move |_| show_phone.update(|v| *v = !*v)
                >
                    <span class="menu-icon">
                        <Icon name="phonelink_setup" />
                    </span>
                    <span class="menu-text">
                        <strong>"Ganti Nomor WhatsApp"</strong>
                        <small>"OTP dikirim ke nomor baru untuk verifikasi"</small>
                    </span>
                    {move || {
                        if show_phone.get() {
                            view! { <Icon name="expand_less" /> }.into_any()
                        } else {
                            view! { <Icon name="expand_more" /> }.into_any()
                        }
                    }}
                </button>
                {move || {
                    show_phone
                        .get()
                        .then(|| {
                            view! {
                                <div id="nomor" class="menu-panel">
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
                                                <button
                                                    type="button"
                                                    class="btn btn-ghost btn-block"
                                                    on:click=move |_| otp_sent.set(false)
                                                >
                                                    "Ganti nomor tujuan"
                                                </button>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <label class="field">
                                                    <span class="field-label">"No. WhatsApp Baru"</span>
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
                                </div>
                            }
                        })
                }}
                {move || {
                    user()
                        .map(|u| match u.role.as_str() {
                            "buyer" => {
                                view! {
                                    <MenuItem
                                        href="/orders"
                                        icon="confirmation_number"
                                        title="Tiket Saya"
                                        sub="E-tiket, nomor kursi & rating perjalanan"
                                    />
                                }
                                    .into_any()
                            }
                            "merchant" => {
                                view! {
                                    <MenuItem
                                        href="/merchant"
                                        icon="storefront"
                                        title="Dashboard Mitra"
                                        sub="Armada, jadwal, denah kursi, sewa & wisata"
                                    />
                                }
                                    .into_any()
                            }
                            _ => {
                                view! {
                                    <MenuItem
                                        href="/admin"
                                        icon="admin_panel_settings"
                                        title="Control Hub"
                                        sub="Kelola seluruh armada & jadwal"
                                    />
                                }
                                    .into_any()
                            }
                        })
                }}
                <MenuItem
                    href="/sewa"
                    icon="airport_shuttle"
                    title="Sewa Bus & Paket Wisata"
                    sub="Charter rombongan dan paket destinasi"
                />
            </nav>
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
            {move || (!ok_msg.get().is_empty()).then(|| view! { <p class="alert alert-ok">{ok_msg.get()}</p> })}

            <span class="label-caps section-label">"Bantuan & Informasi"</span>
            <nav class="card menu-list">
                <MenuItem
                    href="/bantuan"
                    icon="help"
                    title="Pusat Bantuan 24/7"
                    sub="FAQ akun, tiket, sewa bus & kontak CS"
                />
            </nav>

            <button type="button" class="btn btn-danger-ghost btn-block btn-lg" on:click=do_logout>
                <Icon name="logout" />
                "Keluar dari Akun"
            </button>
            <p class="app-version">{concat!("LajuBus App v", env!("CARGO_PKG_VERSION"))}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomor_terformat() {
        assert_eq!(format_phone("6281234567890"), "+62 812-3456-7890");
        assert_eq!(format_phone("08123"), "08123");
    }
}
