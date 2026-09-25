//! Kartu unggah foto trip (multipart ke `/upload/trip-photo`) — dipakai
//! dashboard merchant & admin. Server yang memastikan armada boleh diisi.

use leptos::html;
use leptos::prelude::*;

use super::Icon;
use crate::web::models::Armada;

#[component]
pub fn PhotoUpload(armadas: Vec<Armada>, #[prop(into)] on_uploaded: Callback<()>) -> impl IntoView {
    let file_input_ref = NodeRef::<html::Input>::new();
    let p_armada = RwSignal::new(String::new());
    let p_caption = RwSignal::new(String::new());
    let p_file = RwSignal::new(String::new());
    let p_error = RwSignal::new(String::new());
    let p_busy = RwSignal::new(false);

    #[cfg(not(target_arch = "wasm32"))]
    let _ = on_uploaded;

    let on_file = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let name = file_input_ref
                .get()
                .and_then(|i| i.files())
                .and_then(|f| f.get(0))
                .map(|f| f.name())
                .unwrap_or_default();
            p_file.set(name);
        }
    };

    let do_upload = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(input) = file_input_ref.get() else {
                return;
            };
            let Some(file) = input.files().and_then(|f| f.get(0)) else {
                p_error.set("Pilih file dulu".into());
                return;
            };
            let armada_id = p_armada.get_untracked();
            if armada_id.is_empty() {
                p_error.set("Pilih armada dulu".into());
                return;
            }
            let caption = p_caption.get_untracked();
            p_error.set(String::new());
            p_busy.set(true);
            super::spawn_client(async move {
                let outcome = (|| -> Result<web_sys::FormData, wasm_bindgen::JsValue> {
                    let form = web_sys::FormData::new()?;
                    form.append_with_blob("file", &file)?;
                    form.append_with_str("armada_id", &armada_id)?;
                    form.append_with_str("caption", &caption)?;
                    Ok(form)
                })();
                match outcome {
                    Ok(form) => match gloo_net::http::Request::post("/upload/trip-photo").body(form) {
                        Ok(req) => match req.send().await {
                            Ok(resp) if resp.ok() => {
                                p_caption.set(String::new());
                                p_file.set(String::new());
                                input.set_value("");
                                on_uploaded.run(());
                            }
                            Ok(resp) => {
                                let msg = resp.text().await.unwrap_or_else(|_| "Upload gagal".into());
                                p_error.set(msg);
                            }
                            Err(e) => p_error.set(e.to_string()),
                        },
                        Err(e) => p_error.set(e.to_string()),
                    },
                    Err(_) => p_error.set("Gagal menyiapkan berkas".into()),
                }
                p_busy.set(false);
            });
        }
    };

    view! {
        <section class="card form-card">
            <div class="step-head">
                <span class="step-icon">
                    <Icon name="add_photo_alternate" />
                </span>
                <div>
                    <h3>"Unggah Foto Trip"</h3>
                    <p>"Format JPG/PNG — dokumentasi perjalanan armada"</p>
                </div>
            </div>
            <label class="field">
                <span class="field-label">"Armada"</span>
                <span class="input-wrap">
                    <Icon name="directions_bus" />
                    <select prop:value=move || p_armada.get() on:change=move |ev| p_armada.set(event_target_value(&ev))>
                        <option value="">"— Pilih Armada —"</option>
                        {armadas
                            .iter()
                            .map(|a| view! { <option value=a.id.clone()>{a.name.clone()}</option> })
                            .collect_view()}
                    </select>
                </span>
            </label>
            <label class="dropzone">
                <input type="file" accept="image/*" node_ref=file_input_ref on:change=on_file />
                <Icon name="cloud_upload" />
                <strong>
                    {move || {
                        let f = p_file.get();
                        if f.is_empty() { "Pilih foto dari perangkat".to_string() } else { f }
                    }}
                </strong>
                <small>"Ketuk untuk memilih berkas"</small>
            </label>
            <label class="field">
                <span class="field-label">"Keterangan"</span>
                <span class="input-wrap">
                    <Icon name="edit_note" />
                    <input
                        type="text"
                        placeholder="Mis. Rombongan wisata Bromo"
                        prop:value=move || p_caption.get()
                        on:input=move |ev| p_caption.set(event_target_value(&ev))
                    />
                </span>
            </label>
            <button type="button" class="btn btn-primary btn-block" on:click=do_upload disabled=move || p_busy.get()>
                <Icon name="upload" />
                {move || if p_busy.get() { "Mengunggah…" } else { "Unggah Foto" }}
            </button>
            {move || (!p_error.get().is_empty()).then(|| view! { <p class="alert alert-error">{p_error.get()}</p> })}
        </section>
    }
}
