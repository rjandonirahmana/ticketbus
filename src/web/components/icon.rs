use leptos::prelude::*;

/// Ikon Material Symbols Outlined (font dimuat di `shell.rs`). `filled`
/// memakai varian terisi — dipakai untuk status aktif / badge verifikasi.
#[component]
pub fn Icon(
    #[prop(into)] name: String,
    #[prop(optional, into)] class: String,
    #[prop(optional)] filled: bool,
) -> impl IntoView {
    let cls = match (filled, class.is_empty()) {
        (true, true) => "icon icon-fill".to_string(),
        (true, false) => format!("icon icon-fill {class}"),
        (false, true) => "icon".to_string(),
        (false, false) => format!("icon {class}"),
    };
    view! {
        <span class=cls aria-hidden="true">
            {name}
        </span>
    }
}
