//! web/app/guards.rs — guard rute berbasis peran (UX klien saja).
//!
//! CATATAN KEAMANAN: guard ini hanya menyembunyikan UI & mengalihkan
//! halaman — bukan batas keamanan. Otorisasi sebenarnya ditegakkan di
//! server function (`require_role` di `web/api/server_fns/helpers.rs`).

use leptos::prelude::*;

use super::contexts::SessionResource;

/// Alihkan halaman tanpa menavigasi di tengah pembangunan view (lihat
/// catatan panjang di e-ticketing/src/web/app/guards.rs untuk alasan
/// `Effect` dipakai di client, bukan navigasi langsung di badan komponen).
#[component]
fn GuardRedirect(path: &'static str) -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        view! { <leptos_router::components::Redirect path=path /> }.into_any()
    }
    #[cfg(not(feature = "ssr"))]
    {
        let navigate = leptos_router::hooks::use_navigate();
        Effect::new(move |sudah: Option<()>| {
            if sudah.is_none() {
                navigate(
                    path,
                    leptos_router::NavigateOptions { replace: true, ..Default::default() },
                );
            }
        });
        view! { <div class="page-loading">"Memuat…"</div> }.into_any()
    }
}

fn role_guard(children: ChildrenFn, allowed: &'static [&'static str], redirect: &'static str) -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource not provided");
    let children = StoredValue::new(children);
    view! {
        <Suspense fallback=|| view! { <div class="page-loading">"Memuat…"</div> }>
            {move || {
                session
                    .get()
                    .map(|result| match result {
                        Ok(Some(user)) if allowed.contains(&user.role.as_str()) => {
                            children.with_value(|c| c()).into_any()
                        }
                        _ => view! { <GuardRedirect path=redirect /> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
pub(crate) fn BuyerGuard(children: ChildrenFn) -> impl IntoView {
    role_guard(children, &["buyer"], "/login")
}

#[component]
pub(crate) fn MerchantGuard(children: ChildrenFn) -> impl IntoView {
    role_guard(children, &["merchant"], "/login")
}

#[component]
pub(crate) fn AnyUserGuard(children: ChildrenFn) -> impl IntoView {
    role_guard(children, &["buyer", "merchant", "admin"], "/login")
}

#[component]
pub(crate) fn AdminGuard(children: ChildrenFn) -> impl IntoView {
    role_guard(children, &["admin"], "/login")
}
