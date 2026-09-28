//! Tab "Banner" di /admin: kelola banner "Promo & Info Spesial" beranda —
//! gambar, teks, tautan, dan jadwal tayang. Otorisasi (admin saja) ditegakkan
//! server fn.

use leptos::prelude::*;

use super::{clean_error, format_tanggal, spawn_client, today_wib, BannerSlide, Icon, ImageUploadField, TextField};
use crate::web::api::{delete_banner, list_manage_banners, release_banner_now, save_banner, set_banner_active};
use crate::web::models::{Banner, BannerStatus, NewBanner};

const TEMA: [(&str, &str); 4] = [
    ("sapphire", "Sapphire"),
    ("malam", "Malam"),
    ("emerald", "Emerald"),
    ("tangerine", "Tangerine"),
];

fn status_view(s: BannerStatus) -> (&'static str, &'static str) {
    match s {
        BannerStatus::Tayang => ("Aktif Tayang", "pill pill-ok"),
        BannerStatus::Terjadwal => ("Terjadwal", "pill pill-warn"),
        BannerStatus::Berakhir => ("Berakhir", "pill pill-danger"),
        BannerStatus::Draf => ("Draf", "pill"),
    }
}

/// "Tayang 01 Okt – 31 Okt 2026" dsb. dari tanggal mulai/selesai.
fn jadwal_text(b: &Banner) -> String {
    match (b.mulai.is_empty(), b.selesai.is_empty()) {
        (true, true) => "Tayang tanpa batas waktu".into(),
        (false, true) => format!("Tayang mulai {}", format_tanggal(&b.mulai)),
        (true, false) => format!("Tayang sampai {}", format_tanggal(&b.selesai)),
        (false, false) => format!("{} – {}", format_tanggal(&b.mulai), format_tanggal(&b.selesai)),
    }
}

#[component]
pub fn BannerManager() -> impl IntoView {
    let banners = Resource::new(|| (), |_| list_manage_banners());
    // None = form tertutup, Some(None) = banner baru, Some(Some(b)) = ubah b.
    let editing = RwSignal::new(None::<Option<Banner>>);
    let today = today_wib().format("%Y-%m-%d").to_string();
    let today = StoredValue::new(today);

    let run = move |fut: std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>>| {
        spawn_client(async move {
            let _ = fut.await;
            banners.refetch();
        });
    };

    view! {
        <section class="card banner-admin-head">
            <div class="step-head">
                <span class="step-icon">
                    <Icon name="ad_units" />
                </span>
                <div>
                    <h3>
                        "Manajemen Banner Promo & Informasi"
                        <span class="pill pill-live">
                            <i class="live-dot"></i>
                            "Live di Beranda"
                        </span>
                    </h3>
                    <p>"Kelola materi visual, jadwal tayang, dan tautan promo di beranda penumpang."</p>
                </div>
            </div>
            <button type="button" class="btn btn-primary btn-block" on:click=move |_| editing.set(Some(None))>
                <Icon name="add_circle" />
                "Tambah Banner Baru"
            </button>
            <div class="banner-quick">
                <span>
                    <Icon name="upload_file" />
                    <strong>"Upload Gambar Banner"</strong>
                    <small>"Rekomendasi 1080×480 px"</small>
                </span>
                <span>
                    <Icon name="link" />
                    <strong>"Atur Tautan"</strong>
                    <small>"Halaman LajuBus atau URL promo"</small>
                </span>
                <span>
                    <Icon name="event_available" />
                    <strong>"Teks & Jadwal"</strong>
                    <small>"Atur tanggal mulai & selesai tayang"</small>
                </span>
            </div>
        </section>

        {move || {
            editing
                .get()
                .map(|init| {
                    view! {
                        <BannerForm
                            initial=init
                            on_close=move |_| editing.set(None)
                            on_saved=move |_| {
                                editing.set(None);
                                banners.refetch();
                            }
                        />
                    }
                })
        }}

        <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
            {move || {
                let list = banners.get().and_then(|r| r.ok()).unwrap_or_default();
                let tayang = list.iter().filter(|b| b.status(&today.get_value()) == BannerStatus::Tayang).count();
                view! {
                    <div class="section-head">
                        <h2>"Daftar Banner"</h2>
                        <span class="pill pill-primary">{format!("{tayang} tayang · {} total", list.len())}</span>
                    </div>
                    {if list.is_empty() {
                        view! {
                            <div class="card empty-state">
                                <Icon name="ad_units" />
                                <p>"Belum ada banner. Beranda menampilkan banner bawaan \"Sewa bus rombongan\" sampai ada banner yang tayang."</p>
                            </div>
                        }
                            .into_any()
                    } else {
                        list.into_iter()
                            .map(|b| {
                                let status = b.status(&today.get_value());
                                let (status_label, status_class) = status_view(status);
                                let id = b.id.clone();
                                let (id_t, id_r, id_d) = (id.clone(), id.clone(), id.clone());
                                let aktif = b.aktif;
                                let edit_b = b.clone();
                                let target = if b.link_url.is_empty() { "Tanpa tautan".to_string() } else { b.link_url.clone() };
                                let judul = if b.judul.is_empty() { "(Banner gambar)".to_string() } else { b.judul.clone() };
                                view! {
                                    <article class=if aktif { "card banner-admin-card" } else { "card banner-admin-card inactive" }>
                                        <div class="banner-admin-thumb">
                                            <BannerSlide banner=b.clone() preview=true />
                                        </div>
                                        <div class="banner-admin-info">
                                            <strong>{judul}</strong>
                                            <div class="status-row">
                                                <span class=status_class>{status_label}</span>
                                                {(!b.kode_promo.is_empty()).then(|| view! { <span class="pill pill-secondary">{b.kode_promo.clone()}</span> })}
                                                <span class="pill">{format!("Urutan {}", b.urutan)}</span>
                                            </div>
                                            <small>
                                                <Icon name="schedule" />
                                                {jadwal_text(&b)}
                                            </small>
                                            <small>
                                                <Icon name="link" />
                                                {target}
                                            </small>
                                        </div>
                                        <div class="banner-admin-actions">
                                            <button type="button" class="btn btn-soft btn-sm" on:click=move |_| editing.set(Some(Some(edit_b.clone())))>
                                                <Icon name="edit" />
                                                "Ubah Banner"
                                            </button>
                                            {if status == BannerStatus::Terjadwal {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="btn btn-primary btn-sm"
                                                        on:click=move |_| run(Box::pin(release_banner_now(id_r.clone())))
                                                    >
                                                        <Icon name="rocket_launch" />
                                                        "Rilis Sekarang"
                                                    </button>
                                                }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class=if aktif { "status-toggle on" } else { "status-toggle" }
                                                        title=if aktif { "Sembunyikan dari beranda" } else { "Tayangkan di beranda" }
                                                        on:click=move |_| run(Box::pin(set_banner_active(id_t.clone(), !aktif)))
                                                    >
                                                        {if aktif { "Tayang" } else { "Draf" }}
                                                    </button>
                                                }
                                                    .into_any()
                                            }}
                                            <button
                                                type="button"
                                                class="icon-btn icon-btn-danger"
                                                title="Hapus banner"
                                                on:click=move |_| run(Box::pin(delete_banner(id_d.clone())))
                                            >
                                                <Icon name="delete" />
                                            </button>
                                        </div>
                                    </article>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                }
            }}
        </Suspense>
    }
}

#[component]
fn BannerForm(
    initial: Option<Banner>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_saved: Callback<()>,
) -> impl IntoView {
    let id = initial.as_ref().map(|b| b.id.clone()).unwrap_or_default();
    let is_edit = !id.is_empty();
    let b = initial.unwrap_or(Banner {
        id: String::new(),
        judul: String::new(),
        subjudul: String::new(),
        label: String::new(),
        kode_promo: String::new(),
        cta_label: String::new(),
        link_url: String::new(),
        gambar_url: String::new(),
        tema: "sapphire".into(),
        mulai: String::new(),
        selesai: String::new(),
        urutan: 0,
        aktif: true,
    });
    let judul = RwSignal::new(b.judul);
    let subjudul = RwSignal::new(b.subjudul);
    let label = RwSignal::new(b.label);
    let kode = RwSignal::new(b.kode_promo);
    let cta = RwSignal::new(b.cta_label);
    let link = RwSignal::new(b.link_url);
    let gambar = RwSignal::new(b.gambar_url);
    let tema = RwSignal::new(b.tema);
    let mulai = RwSignal::new(b.mulai);
    let selesai = RwSignal::new(b.selesai);
    let urutan = RwSignal::new(b.urutan.to_string());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let input = move || NewBanner {
        judul: judul.get(),
        subjudul: subjudul.get(),
        label: label.get(),
        kode_promo: kode.get(),
        cta_label: cta.get(),
        link_url: link.get(),
        gambar_url: gambar.get(),
        tema: tema.get(),
        mulai: mulai.get(),
        selesai: selesai.get(),
        urutan: urutan.get().trim().parse().unwrap_or(0),
    };
    // Pratinjau langsung — tampilan sama persis dengan kartu di beranda.
    let preview = move || {
        let i = input();
        Banner {
            id: String::new(),
            judul: i.judul.trim().to_string(),
            subjudul: i.subjudul.trim().to_string(),
            label: i.label.trim().to_string(),
            kode_promo: i.kode_promo.trim().to_uppercase(),
            cta_label: i.cta_label.trim().to_string(),
            link_url: String::new(),
            gambar_url: i.gambar_url,
            tema: i.tema,
            mulai: String::new(),
            selesai: String::new(),
            urutan: 0,
            aktif: true,
        }
    };

    let submit = move |_| {
        let data = input();
        let id = id.clone();
        busy.set(true);
        spawn_client(async move {
            match save_banner(id, data).await {
                Ok(_) => on_saved.run(()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <section class="card form-card">
            <div class="step-head">
                <span class="step-icon">
                    <Icon name="ad_units" />
                </span>
                <div>
                    <h3>{if is_edit { "Ubah Banner" } else { "Banner Baru" }}</h3>
                    <p>"Gambar saja, teks saja, atau teks di atas gambar"</p>
                </div>
                <button type="button" class="icon-btn" title="Tutup" on:click=move |_| on_close.run(())>
                    <Icon name="close" />
                </button>
            </div>

            <div class="field">
                <span class="field-label">"Pratinjau di Beranda"</span>
                <div class="banner-preview">
                    {move || {
                        let p = preview();
                        if p.judul.is_empty() && p.gambar_url.is_empty() {
                            view! { <p class="note-card">"Isi judul atau unggah gambar untuk melihat pratinjau."</p> }.into_any()
                        } else {
                            view! { <BannerSlide banner=p preview=true /> }.into_any()
                        }
                    }}
                </div>
            </div>

            <ImageUploadField
                url=gambar
                label="Gambar Banner (opsional)"
                hint="JPG/PNG, rekomendasi 1080×480 px. Tanpa judul = tampil gambar saja."
            />
            {move || {
                (!gambar.get().is_empty())
                    .then(|| {
                        view! {
                            <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| gambar.set(String::new())>
                                <Icon name="hide_image" />
                                "Hapus gambar"
                            </button>
                        }
                    })
            }}

            <TextField label="Judul (opsional bila ada gambar)" icon="title" placeholder="Diskon Rute Jawa–Sumatera s/d 25%" value=judul />
            <TextField label="Keterangan" icon="notes" placeholder="Gunakan kode kupon saat checkout perjalananmu." value=subjudul />
            <div class="field-grid">
                <TextField label="Label Kecil" icon="sell" placeholder="Spesial Liburan" value=label />
                <TextField label="Kode Promo" icon="confirmation_number" placeholder="LAJUSERU" value=kode />
            </div>
            <div class="field-grid">
                <TextField label="Teks Tombol (bila tanpa kode)" icon="smart_button" placeholder="Klaim Promo" value=cta />
                <TextField label="Tautan" icon="link" placeholder="/wisata atau https://…" value=link />
            </div>
            <div class="field">
                <span class="field-label">"Tema Warna (tanpa gambar)"</span>
                <div class="chip-row">
                    {TEMA
                        .into_iter()
                        .map(|(v, l)| {
                            view! {
                                <button
                                    type="button"
                                    class=move || if tema.get() == v { "filter-chip active" } else { "filter-chip" }
                                    on:click=move |_| tema.set(v.to_string())
                                >
                                    <span class=format!("swatch tema-{v}")></span>
                                    {l}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
            <div class="field-grid">
                <label class="field">
                    <span class="field-label">"Mulai Tayang"</span>
                    <span class="input-wrap">
                        <Icon name="event_available" />
                        <input type="date" prop:value=move || mulai.get() on:input=move |ev| mulai.set(event_target_value(&ev)) />
                    </span>
                </label>
                <label class="field">
                    <span class="field-label">"Selesai Tayang"</span>
                    <span class="input-wrap">
                        <Icon name="event_busy" />
                        <input type="date" prop:value=move || selesai.get() on:input=move |ev| selesai.set(event_target_value(&ev)) />
                    </span>
                </label>
            </div>
            <TextField label="Urutan (kecil tampil duluan)" icon="format_list_numbered" placeholder="0" value=urutan number=true />

            <button type="button" class="btn btn-cta btn-block" on:click=submit disabled=move || busy.get()>
                <Icon name="publish" />
                {move || match (busy.get(), is_edit) {
                    (true, _) => "Menyimpan…",
                    (false, true) => "Simpan Perubahan",
                    (false, false) => "Terbitkan Banner",
                }}
            </button>
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
        </section>
    }
}
