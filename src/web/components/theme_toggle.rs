use leptos::prelude::*;

use super::Icon;

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let toggle = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Some(html) = doc.document_element() {
                        let current = html.get_attribute("data-theme").unwrap_or_default();
                        let next = if current == "dark" { "light" } else { "dark" };
                        let _ = html.set_attribute("data-theme", next);
                        if let Ok(Some(storage)) = win.local_storage() {
                            let _ = storage.set_item("bis.theme", next);
                        }
                    }
                }
            }
        }
    };
    view! {
        <button type="button" class="icon-btn" on:click=toggle title="Ganti tema terang/gelap">
            <Icon name="dark_mode" />
        </button>
    }
}
