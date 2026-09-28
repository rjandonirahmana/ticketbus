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

/// Field unggah foto (POST /upload/listing-photo → URL ke `url`). Setelah
/// terunggah, pratinjau bisa DIGESER untuk mengatur bagian foto yang tampil;
/// posisinya ikut tersimpan di URL (`#pos=x,y`, lihat `web::foto`).
/// `aspect` = rasio tampilan sebenarnya (mis. "9 / 4" untuk banner) supaya
/// potongan di pratinjau sama dengan yang dilihat penumpang.
#[component]
pub fn ImageUploadField(
    url: RwSignal<String>,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional, into)] hint: Option<String>,
    #[prop(optional)] aspect: Option<&'static str>,
    /// Matikan geser posisi (mis. foto dokumen).
    #[prop(optional)]
    no_pan: bool,
) -> impl IntoView {
    let aspect = aspect.unwrap_or("2 / 1");
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

    let has_img = Memo::new(move |_| !url.get().is_empty());
    let pan_ref = NodeRef::<html::Div>::new();
    let img_ref = NodeRef::<html::Img>::new();
    // (x awal pointer, y awal, posisi x awal, posisi y awal)
    let drag = StoredValue::new(None::<(f64, f64, f64, f64)>);
    let on_down = move |ev: web_sys::PointerEvent| {
        if no_pan {
            return;
        }
        ev.prevent_default();
        let (_, (px, py)) = crate::web::foto::split(&url.get_untracked());
        drag.set_value(Some((ev.client_x() as f64, ev.client_y() as f64, px, py)));
    };
    let on_move = move |ev: web_sys::PointerEvent| {
        let Some((sx, sy, px, py)) = drag.get_value() else { return };
        let (Some(box_), Some(img)) = (pan_ref.get(), img_ref.get()) else { return };
        let (cw, ch) = (box_.client_width().max(1) as f64, box_.client_height().max(1) as f64);
        let (nw, nh) = (img.natural_width().max(1) as f64, img.natural_height().max(1) as f64);
        // object-fit: cover → foto diperbesar sampai menutup kotak; yang bisa
        // digeser hanya kelebihannya (ox/oy piksel). Geser 1 px = 1 px foto.
        let skala = (cw / nw).max(ch / nh);
        let (ox, oy) = (nw * skala - cw, nh * skala - ch);
        let geser = |awal: f64, delta: f64, lebih: f64| if lebih > 1.0 { awal - delta / lebih * 100.0 } else { 50.0 };
        let nx = geser(px, ev.client_x() as f64 - sx, ox);
        let ny = geser(py, ev.client_y() as f64 - sy, oy);
        url.set(crate::web::foto::with_pos(&url.get_untracked(), nx, ny));
    };
    let on_up = move |_: web_sys::PointerEvent| drag.set_value(None);

    view! {
        <div class="field">
            <span class="field-label">{label}</span>
            {move || {
                if !has_img.get() {
                    let hint = hint.clone();
                    return view! {
                        <label class="dropzone">
                            <input type="file" accept="image/*" node_ref=input_ref on:change=on_change />
                            <Icon name="add_photo_alternate" />
                            <strong>{move || if busy.get() { "Mengunggah…" } else { "Pilih foto" }}</strong>
                            <small>{hint}</small>
                        </label>
                    }
                        .into_any();
                }
                view! {
                    <div
                        class=if no_pan { "photo-pan static" } else { "photo-pan" }
                        style=format!("aspect-ratio:{aspect}")
                        node_ref=pan_ref
                        on:pointerdown=on_down
                        on:pointermove=on_move
                        on:pointerup=on_up
                        on:pointerleave=on_up
                        on:pointercancel=on_up
                    >
                        <img
                            node_ref=img_ref
                            src=move || crate::web::foto::split(&url.get()).0.to_string()
                            style=move || crate::web::foto::style(&url.get())
                            alt="Pratinjau foto"
                            draggable="false"
                        />
                        {(!no_pan)
                            .then(|| {
                                view! {
                                    <span class="photo-pan-hint">
                                        <Icon name="open_with" />
                                        "Geser untuk atur posisi"
                                    </span>
                                }
                            })}
                    </div>
                    <div class="photo-pan-bar">
                        <label class="btn btn-soft btn-sm">
                            <input type="file" accept="image/*" node_ref=input_ref on:change=on_change hidden />
                            <Icon name="photo_camera" />
                            {move || if busy.get() { "Mengunggah…" } else { "Ganti foto" }}
                        </label>
                        {(!no_pan)
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        class="btn btn-ghost btn-sm"
                                        on:click=move |_| url.update(|u| *u = crate::web::foto::with_pos(u, 50.0, 50.0))
                                    >
                                        <Icon name="center_focus_strong" />
                                        "Tengahkan"
                                    </button>
                                }
                            })}
                    </div>
                }
                    .into_any()
            }}
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
        </div>
    }
}
