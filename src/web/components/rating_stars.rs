use leptos::prelude::*;

/// Tampilan bintang read-only (mis. rata-rata rating armada di daftar browse).
#[component]
pub fn RatingStars(value: f64, #[prop(default = 5)] max: i32) -> impl IntoView {
    let rounded = value.round() as i32;
    view! {
        <span class="rating-stars" title=format!("{value:.1} / {max}")>
            {(1..=max)
                .map(|i| {
                    let filled = i <= rounded;
                    view! { <span class=if filled { "star filled" } else { "star" }>"★"</span> }
                })
                .collect_view()}
        </span>
    }
}

/// Input bintang interaktif (form rating order).
#[component]
pub fn RatingInput(value: RwSignal<i32>, #[prop(default = 5)] max: i32) -> impl IntoView {
    view! {
        <span class="rating-input">
            {(1..=max)
                .map(|i| {
                    view! {
                        <button
                            type="button"
                            class=move || {
                                let is_filled = value.get() >= i;
                                if is_filled { "star filled" } else { "star" }
                            }
                            on:click=move |_| value.set(i)
                        >
                            "★"
                        </button>
                    }
                })
                .collect_view()}
        </span>
    }
}
