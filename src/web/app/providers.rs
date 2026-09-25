use leptos::prelude::*;

use crate::web::api::current_session;

use super::contexts::SessionResource;

pub(crate) fn provide_all_app_contexts() {
    // `new_blocking`: SSR menunggu cookie sebelum merender, client membaca
    // state yang sudah di-serialize dari HTML (langsung resolved, tanpa
    // kedipan Suspense).
    let session: SessionResource = Resource::new_blocking(|| (), |_| current_session());
    provide_context(session);
}
