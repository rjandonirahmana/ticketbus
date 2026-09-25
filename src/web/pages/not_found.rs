use leptos::prelude::*;

use crate::web::components::Icon;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="page">
            <div class="card empty-state not-found">
                <Icon name="wrong_location" />
                <h1>"404"</h1>
                <p>"Halaman tidak ditemukan — mungkin rutenya sudah berubah."</p>
                <a href="/" class="btn btn-primary">
                    <Icon name="home" />
                    "Kembali ke beranda"
                </a>
            </div>
        </div>
    }
}
