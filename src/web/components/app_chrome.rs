//! Kerangka aplikasi: app bar atas + bottom nav (mobile) / tautan inline
//! (desktop). Item nav bergantung peran sesi — satu sumber (`nav_items`)
//! dipakai kedua tampilan supaya tak pernah beda isi.

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::{initials, Icon, ThemeToggle};
use crate::web::app::SessionResource;
use crate::web::models::PublicUser;

struct NavItem {
    href: &'static str,
    label: &'static str,
    icon: &'static str,
}

fn nav_items(user: Option<&PublicUser>) -> Vec<NavItem> {
    let mut items = vec![
        NavItem { href: "/", label: "Beranda", icon: "directions_bus" },
        NavItem { href: "/sewa", label: "Charter", icon: "airport_shuttle" },
    ];
    match user.map(|u| u.role.as_str()) {
        Some("merchant") => items.push(NavItem { href: "/merchant", label: "Mitra", icon: "storefront" }),
        Some("admin") => items.push(NavItem { href: "/admin", label: "Admin", icon: "admin_panel_settings" }),
        _ => items.push(NavItem { href: "/orders", label: "Tiket", icon: "confirmation_number" }),
    }
    items.push(NavItem { href: "/wisata", label: "Wisata", icon: "beach_access" });
    let akun = if user.is_some() { "/akun" } else { "/login" };
    items.push(NavItem { href: akun, label: "Akun", icon: "person" });
    items
}

fn is_active(href: &str, path: &str) -> bool {
    match href {
        "/" => path == "/",
        "/login" | "/akun" => matches!(path, "/login" | "/akun" | "/register" | "/verify-otp" | "/lupa-password"),
        _ => path == href || path.starts_with(&format!("{href}/")),
    }
}

fn nav_links(user: Option<PublicUser>, path: String, class: &'static str) -> impl IntoView {
    nav_items(user.as_ref())
        .into_iter()
        .map(|it| {
            let active = is_active(it.href, &path);
            view! {
                <a href=it.href class=if active { format!("{class} active") } else { class.to_string() }>
                    <Icon name=it.icon filled=active />
                    <span>{it.label}</span>
                </a>
            }
        })
        .collect_view()
}

/// Logo LajuBus: ubin bus sapphire + wordmark. Dipakai app bar & halaman auth.
#[component]
pub fn BrandLogo(#[prop(optional)] large: bool) -> impl IntoView {
    view! {
        <span class=if large { "brand-logo-wrap large" } else { "brand-logo-wrap" }>
            <span class="brand-logo">
                <Icon name="directions_bus" filled=true />
            </span>
            <span class="brand-text">
                <strong>
                    "Laju" <em>"Bus"</em>
                </strong>
                <small>"Tiket & Jadwal Bis"</small>
            </span>
        </span>
    }
}

#[component]
pub fn AppBar() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let location = use_location();
    let user = move || session.get().and_then(|r| r.ok()).flatten();

    view! {
        <header class="appbar">
            <div class="appbar-inner">
                <a href="/" class="brand-mark">
                    <BrandLogo />
                </a>
                <Suspense fallback=|| ()>
                    <nav class="appbar-links">
                        {move || nav_links(user(), location.pathname.get(), "appbar-link")}
                    </nav>
                    <div class="appbar-actions">
                        <ThemeToggle />
                        {move || match user() {
                            Some(u) => {
                                view! {
                                    <span class="status-chip">
                                        <i class="status-dot"></i>
                                        "Online"
                                    </span>
                                    <a href="/akun" class="avatar" title=u.name.clone()>
                                        {initials(&u.name)}
                                    </a>
                                }
                                    .into_any()
                            }
                            None => view! { <a href="/login" class="btn btn-sm btn-primary">"Masuk"</a> }.into_any(),
                        }}
                    </div>
                </Suspense>
            </div>
        </header>
    }
}

#[component]
pub fn BottomNav() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let location = use_location();
    let user = move || session.get().and_then(|r| r.ok()).flatten();

    view! {
        <nav class="bottom-nav">
            <Suspense fallback=|| ()>
                {move || nav_links(user(), location.pathname.get(), "bottom-link")}
            </Suspense>
        </nav>
    }
}

/// Judul halaman dalam (tombol kembali + judul + slot kanan opsional),
/// seperti header "Pilih Kursi & Penumpang" / "Manajemen Armada" di desain.
#[component]
pub fn PageHead(
    #[prop(into)] title: String,
    #[prop(optional)] back: Option<&'static str>,
    #[prop(optional, into)] subtitle: String,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="page-head">
            {back
                .map(|href| {
                    view! {
                        <a href=href class="icon-btn" title="Kembali">
                            <Icon name="arrow_back" />
                        </a>
                    }
                })}
            <div class="page-head-text">
                <h1>{title}</h1>
                {(!subtitle.is_empty()).then(|| view! { <p>{subtitle}</p> })}
            </div>
            {children.map(|c| c())}
        </div>
    }
}
