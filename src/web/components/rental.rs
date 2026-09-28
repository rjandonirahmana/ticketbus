//! Komponen bersama halaman sewa (/sewa) & wisata (/wisata): sheet form
//! permintaan sewa, tautan WhatsApp CS, dan field unggah foto listing.

use leptos::html;
use leptos::prelude::*;

use super::{clean_error, spawn_client, Icon};
use crate::web::api::create_rental_request;
use crate::web::app::SessionResource;
use crate::web::models::NewRentalRequest;

/// Percent-encode untuk query string wa.me (UTF-8 aman, termasuk emoji).
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Tautan chat WhatsApp ke nomor CS dengan pesan terisi. `phone` format 62xxx.
pub fn wa_link(phone: &str, text: &str) -> String {
    format!("https://wa.me/{phone}?text={}", url_encode(text))
}

/// Nilai awal form permintaan (dari kalkulator / form custom).
#[derive(Clone, Default, PartialEq)]
pub struct RequestPrefill {
    pub jemput: String,
    pub tujuan: String,
    pub tgl_berangkat: String,
    pub tgl_pulang: String,
    pub tipe_perjalanan: String,
    pub jumlah_orang: i32,
    pub catatan: String,
}

/// Target sheet: item apa yang diminta.
#[derive(Clone, PartialEq)]
pub struct RequestTarget {
    /// `paket` | `charter` | `custom`.
    pub jenis: &'static str,
    pub item_id: String,
    pub item_nama: String,
    /// Subjudul ringkas, mis. "3 Hari 2 Malam · min 40 pax".
    pub item_info: String,
    pub prefill: RequestPrefill,
}

const TIPE: [&str; 3] = ["Pulang-Pergi", "Menginap", "Sekali Jalan"];

#[component]
pub fn RentalRequestSheet(
    target: RequestTarget,
    #[prop(into)] cs_phone: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let p = target.prefill.clone();
    let nama = RwSignal::new(String::new());
    let telp = RwSignal::new(String::new());
    let jemput = RwSignal::new(p.jemput);
    let tujuan = RwSignal::new(p.tujuan);
    let tgl_berangkat = RwSignal::new(p.tgl_berangkat);
    let tgl_pulang = RwSignal::new(p.tgl_pulang);
    let tipe = RwSignal::new(if p.tipe_perjalanan.is_empty() { TIPE[0].to_string() } else { p.tipe_perjalanan });
    let jumlah = RwSignal::new(p.jumlah_orang.max(1).to_string());
    let catatan = RwSignal::new(p.catatan);
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let done = RwSignal::new(false);

    if let Some(Ok(Some(u))) = session.get_untracked() {
        nama.set(u.name);
        telp.set(u.phone);
    }

    let jenis = target.jenis;
    let item_id = target.item_id.clone();
    let submit = move |_| {
        let input = NewRentalRequest {
            jenis: jenis.to_string(),
            item_id: item_id.clone(),
            nama: nama.get_untracked(),
            telp: telp.get_untracked(),
            jemput: jemput.get_untracked(),
            tujuan: tujuan.get_untracked(),
            tgl_berangkat: tgl_berangkat.get_untracked(),
            tgl_pulang: tgl_pulang.get_untracked(),
            tipe_perjalanan: tipe.get_untracked(),
            jumlah_orang: jumlah.get_untracked().trim().parse().unwrap_or(0),
            catatan: catatan.get_untracked(),
        };
        if input.nama.trim().is_empty() || input.telp.trim().is_empty() {
            error.set("Nama dan nomor WhatsApp wajib diisi".into());
            return;
        }
        error.set(String::new());
        busy.set(true);
        spawn_client(async move {
            match create_rental_request(input).await {
                Ok(_) => done.set(true),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    // Pesan WhatsApp ke CS disusun dari isian form saat ini.
    // StoredValue → closure ini `Copy`, bisa dipakai di beberapa closure view.
    let item_nama = StoredValue::new(target.item_nama.clone());
    let wa_text = move || {
        let mut t = format!("Halo CS LajuBus 👋\nSaya ingin konsultasi *{}*.\n", item_nama.get_value());
        let j = jemput.get();
        let tj = tujuan.get();
        if !j.is_empty() || !tj.is_empty() {
            t.push_str(&format!("📍 Rute: {j} → {tj}\n"));
        }
        let tb = tgl_berangkat.get();
        if !tb.is_empty() {
            t.push_str(&format!("📅 Berangkat: {tb}\n"));
        }
        t.push_str(&format!("👥 Rombongan: {} orang\n", jumlah.get()));
        t.push_str(&format!("Atas nama: {}", nama.get()));
        t
    };
    let cs = StoredValue::new(cs_phone);
    let show_tipe = jenis != "paket";
    let judul = target.item_nama.clone();
    let item_info = StoredValue::new(target.item_info.clone());

    view! {
        <div class="modal-overlay" on:click=move |_| on_close.run(())>
            <div class="sheet" on:click=|ev| ev.stop_propagation()>
                <div class="sheet-grip"></div>
                <div class="sheet-header">
                    <div>
                        <span class="label-caps">
                            {match jenis {
                                "paket" => "Booking Paket Wisata",
                                "charter" => "Sewa Armada Charter",
                                _ => "Rencana Wisata Custom",
                            }}
                        </span>
                        <h3>{judul}</h3>
                    </div>
                    <button type="button" class="icon-btn" title="Tutup" on:click=move |_| on_close.run(())>
                        <Icon name="close" />
                    </button>
                </div>
                {move || {
                    if done.get() {
                        let href = wa_link(&cs.get_value(), &wa_text());
                        view! {
                            <div class="sheet-body">
                                <div class="success-state">
                                    <span class="success-icon">
                                        <Icon name="check_circle" filled=true />
                                    </span>
                                    <h3>"Permintaan terkirim!"</h3>
                                    <p>
                                        "Tim LajuBus / mitra PO akan menghubungi Anda lewat WhatsApp untuk penawaran harga & ketersediaan armada."
                                    </p>
                                    {(!cs.get_value().is_empty())
                                        .then(|| {
                                            view! {
                                                <a class="btn btn-wa btn-block" href=href.clone() target="_blank" rel="noopener">
                                                    <Icon name="chat" />
                                                    "Lanjut Chat WhatsApp CS"
                                                </a>
                                            }
                                        })}
                                    <button type="button" class="btn btn-soft btn-block" on:click=move |_| on_close.run(())>
                                        "Tutup"
                                    </button>
                                </div>
                            </div>
                        }
                            .into_any()
                    } else {
                        let submit = submit.clone();
                        view! {
                            <div class="sheet-body">
                                {(!item_info.get_value().is_empty())
                                    .then(|| {
                                        view! {
                                            <p class="info-banner">
                                                <Icon name="info" />
                                                {item_info.get_value()}
                                            </p>
                                        }
                                    })}
                                <div class="card form-card">
                                    <div class="field-grid">
                                        <label class="field">
                                            <span class="field-label">"Nama Pemesan"</span>
                                            <span class="input-wrap">
                                                <Icon name="person" />
                                                <input
                                                    type="text"
                                                    placeholder="Nama / instansi"
                                                    prop:value=move || nama.get()
                                                    on:input=move |ev| nama.set(event_target_value(&ev))
                                                />
                                            </span>
                                        </label>
                                        <label class="field">
                                            <span class="field-label">"No. WhatsApp"</span>
                                            <span class="input-wrap">
                                                <Icon name="chat" />
                                                <input
                                                    type="tel"
                                                    placeholder="08xx"
                                                    prop:value=move || telp.get()
                                                    on:input=move |ev| telp.set(event_target_value(&ev))
                                                />
                                            </span>
                                        </label>
                                    </div>
                                    <div class="route-box">
                                        <label class="field">
                                            <span class="field-label">"Titik Jemput"</span>
                                            <span class="input-wrap">
                                                <Icon name="trip_origin" />
                                                <input
                                                    type="text"
                                                    placeholder="Mis. Jakarta Selatan"
                                                    prop:value=move || jemput.get()
                                                    on:input=move |ev| jemput.set(event_target_value(&ev))
                                                />
                                            </span>
                                        </label>
                                        <label class="field">
                                            <span class="field-label">"Kota / Area Tujuan"</span>
                                            <span class="input-wrap">
                                                <Icon name="location_on" />
                                                <input
                                                    type="text"
                                                    placeholder="Mis. Bandung"
                                                    prop:value=move || tujuan.get()
                                                    on:input=move |ev| tujuan.set(event_target_value(&ev))
                                                />
                                            </span>
                                        </label>
                                    </div>
                                    <div class="field-grid">
                                        <label class="field">
                                            <span class="field-label">"Tanggal Berangkat"</span>
                                            <span class="input-wrap">
                                                <Icon name="calendar_today" />
                                                <input
                                                    type="date"
                                                    prop:value=move || tgl_berangkat.get()
                                                    on:input=move |ev| tgl_berangkat.set(event_target_value(&ev))
                                                />
                                            </span>
                                        </label>
                                        <label class="field">
                                            <span class="field-label">"Tanggal Pulang"</span>
                                            <span class="input-wrap">
                                                <Icon name="event" />
                                                <input
                                                    type="date"
                                                    prop:value=move || tgl_pulang.get()
                                                    on:input=move |ev| tgl_pulang.set(event_target_value(&ev))
                                                />
                                            </span>
                                        </label>
                                    </div>
                                    {show_tipe
                                        .then(|| {
                                            view! {
                                                <div class="segmented seg-3">
                                                    {TIPE
                                                        .into_iter()
                                                        .map(|t| {
                                                            view! {
                                                                <button
                                                                    type="button"
                                                                    class=move || if tipe.get() == t { "seg active" } else { "seg" }
                                                                    on:click=move |_| tipe.set(t.to_string())
                                                                >
                                                                    {t}
                                                                </button>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })}
                                    <label class="field">
                                        <span class="field-label">"Jumlah Rombongan"</span>
                                        <span class="input-wrap">
                                            <Icon name="groups" />
                                            <input
                                                type="number"
                                                min="1"
                                                class="num"
                                                prop:value=move || jumlah.get()
                                                on:input=move |ev| jumlah.set(event_target_value(&ev))
                                            />
                                            <small class="suffix">"orang"</small>
                                        </span>
                                    </label>
                                    <label class="field">
                                        <span class="field-label">"Catatan (opsional)"</span>
                                        <textarea
                                            placeholder="Mis. butuh bagasi besar, jemput di 2 titik, dll."
                                            prop:value=move || catatan.get()
                                            on:input=move |ev| catatan.set(event_target_value(&ev))
                                        ></textarea>
                                    </label>
                                </div>
                                {move || {
                                    (!error.get().is_empty())
                                        .then(|| view! { <p class="alert alert-error">{error.get()}</p> })
                                }}
                                <p class="note-card">
                                    <Icon name="verified" />
                                    "Gratis konsultasi. Harga final dikonfirmasi CS / mitra PO lewat WhatsApp."
                                </p>
                            </div>
                            <div class="sheet-footer">
                                {move || {
                                    let c = cs.get_value();
                                    (!c.is_empty())
                                        .then(|| {
                                            view! {
                                                <a
                                                    class="btn btn-soft"
                                                    href=wa_link(&c, &wa_text())
                                                    target="_blank"
                                                    rel="noopener"
                                                    title="Chat WhatsApp CS"
                                                >
                                                    <Icon name="chat" />
                                                    "Chat CS"
                                                </a>
                                            }
                                        })
                                }}
                                <button type="button" class="btn btn-cta" on:click=submit disabled=move || busy.get()>
                                    {move || if busy.get() { "Mengirim…" } else { "Kirim Permintaan" }}
                                    <Icon name="send" />
                                </button>
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Field unggah foto sampul (POST /upload/listing-photo → URL ke `url`).
#[component]
pub fn ImageUploadField(
    url: RwSignal<String>,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional, into)] hint: Option<String>,
) -> impl IntoView {
    let label = label.unwrap_or_else(|| "Foto Sampul".into());
    let hint = hint.unwrap_or_else(|| "JPG/PNG — tampil di kartu katalog".into());
    let input_ref = NodeRef::<html::Input>::new();
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let on_change = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(file) = input_ref.get().and_then(|i| i.files()).and_then(|f| f.get(0)) else {
                return;
            };
            busy.set(true);
            error.set(String::new());
            spawn_client(async move {
                let form = web_sys::FormData::new().and_then(|f| f.append_with_blob("file", &file).map(|_| f));
                match form {
                    Ok(form) => match gloo_net::http::Request::post("/upload/listing-photo").body(form) {
                        Ok(req) => match req.send().await {
                            Ok(resp) if resp.ok() => {
                                let body = resp.text().await.unwrap_or_default();
                                match serde_json::from_str::<serde_json::Value>(&body) {
                                    Ok(v) => url.set(v["url"].as_str().unwrap_or_default().to_string()),
                                    Err(e) => error.set(e.to_string()),
                                }
                            }
                            Ok(resp) => error.set(resp.text().await.unwrap_or_else(|_| "Upload gagal".into())),
                            Err(e) => error.set(e.to_string()),
                        },
                        Err(e) => error.set(e.to_string()),
                    },
                    Err(_) => error.set("Gagal menyiapkan berkas".into()),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <div class="field">
            <span class="field-label">{label}</span>
            <label class="dropzone">
                <input type="file" accept="image/*" node_ref=input_ref on:change=on_change />
                {move || {
                    let u = url.get();
                    if u.is_empty() {
                        view! {
                            <Icon name="add_photo_alternate" />
                            <strong>{move || if busy.get() { "Mengunggah…" } else { "Pilih foto sampul" }}</strong>
                            <small>{hint.clone()}</small>
                        }
                            .into_any()
                    } else {
                        view! {
                            <img class="dropzone-preview" src=u alt="Pratinjau foto" />
                            <small>"Ketuk untuk mengganti foto"</small>
                        }
                            .into_any()
                    }
                }}
            </label>
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
        </div>
    }
}
