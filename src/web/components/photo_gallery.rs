use leptos::prelude::*;

use super::{format_tanggal, Icon};
use crate::web::models::TripPhoto;

#[component]
pub fn PhotoGallery(
    photos: Vec<TripPhoto>,
    is_admin: bool,
    #[prop(into)] on_delete: Callback<String>,
) -> impl IntoView {
    if photos.is_empty() {
        return view! {
            <div class="empty-state">
                <Icon name="photo_library" />
                <p>"Belum ada foto trip."</p>
            </div>
        }
            .into_any();
    }

    view! {
        <div class="photo-grid">
            {photos
                .into_iter()
                .map(|p| {
                    let id = p.id.clone();
                    let tgl = format_tanggal(p.created_at.get(..10).unwrap_or(&p.created_at));
                    view! {
                        <figure class="photo-card">
                            <img src=p.url.clone() alt=p.caption.clone() loading="lazy" />
                            <figcaption class="photo-meta">
                                <span class="pill pill-primary">{p.armada_name.clone()}</span>
                                {(!p.caption.is_empty()).then(|| view! { <p class="photo-caption">{p.caption.clone()}</p> })}
                                <small>{tgl}</small>
                            </figcaption>
                            {is_admin
                                .then(|| {
                                    view! {
                                        <button
                                            type="button"
                                            class="icon-btn icon-btn-danger photo-del"
                                            title="Hapus foto"
                                            on:click=move |_| on_delete.run(id.clone())
                                        >
                                            <Icon name="delete" />
                                        </button>
                                    }
                                })}
                        </figure>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}
