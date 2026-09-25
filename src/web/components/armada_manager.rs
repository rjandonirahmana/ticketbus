use leptos::prelude::*;

use super::{spawn_client, Icon, RatingStars};
use crate::web::api::{delete_armada, update_armada};
use crate::web::models::Armada;

#[component]
pub fn ArmadaManager(armadas: Vec<Armada>, #[prop(into)] on_changed: Callback<()>) -> impl IntoView {
    if armadas.is_empty() {
        return view! {
            <div class="empty-state">
                <Icon name="directions_bus" />
                <p>"Belum ada armada. Tambahkan bus pertama Anda di bawah."</p>
            </div>
        }
            .into_any();
    }
    view! {
        <div class="armada-list">
            <For each=move || armadas.clone() key=|a| a.id.clone() let:armada>
                <ArmadaRow armada=armada on_changed=on_changed />
            </For>
        </div>
    }
        .into_any()
}

#[component]
fn ArmadaRow(armada: Armada, #[prop(into)] on_changed: Callback<()>) -> impl IntoView {
    let name = RwSignal::new(armada.name.clone());
    let color = RwSignal::new(armada.color_hex.clone());
    let id_save = armada.id.clone();
    let id_del = armada.id.clone();

    let save = move |_| {
        let id = id_save.clone();
        let n = name.get_untracked();
        let c = color.get_untracked();
        spawn_client(async move {
            if update_armada(id, n, c).await.is_ok() {
                on_changed.run(());
            }
        });
    };
    let remove = move |_| {
        let id = id_del.clone();
        spawn_client(async move {
            if delete_armada(id).await.is_ok() {
                on_changed.run(());
            }
        });
    };

    view! {
        <div class="armada-row">
            <label class="color-chip" style=move || format!("background:{}", color.get()) title="Ganti warna">
                <input
                    type="color"
                    prop:value=move || color.get()
                    on:input=move |ev| color.set(event_target_value(&ev))
                />
            </label>
            <div class="armada-main">
                <input
                    type="text"
                    class="inline-input"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
                <span class="armada-rating">
                    {if armada.jumlah_rating > 0 {
                        view! {
                            <RatingStars value=armada.avg_rating_armada />
                            <small>{format!("{:.1} · {} ulasan", armada.avg_rating_armada, armada.jumlah_rating)}</small>
                        }
                            .into_any()
                    } else {
                        view! { <small>"Belum ada ulasan"</small> }.into_any()
                    }}
                </span>
            </div>
            <button type="button" class="icon-btn icon-btn-primary" title="Simpan" on:click=save>
                <Icon name="save" />
            </button>
            <button type="button" class="icon-btn icon-btn-danger" title="Hapus" on:click=remove>
                <Icon name="delete" />
            </button>
        </div>
    }
}
