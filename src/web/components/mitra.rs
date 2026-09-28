//! Komponen Mitra PO bersama: `PoCard` (kartu profil PO — pratinjau saat
//! mendaftar, review admin, dashboard mitra, dan kepala halaman /po/:id) dan
//! `ProfileFields` (isian profil PO untuk daftar & ubah).

use leptos::prelude::*;

use super::{initials, Icon, ImageUploadField, TextField};
use crate::web::models::{MerchantProfile, NewMerchantProfile};

/// Pilihan layanan PO (bebas ditambah lewat teks, tapi ini yang umum).
pub const LAYANAN: [&str; 6] = ["AKAP", "Pariwisata", "Shuttle", "Travel", "Antar Jemput", "Sleeper"];

pub fn status_label(status: &str) -> (&'static str, &'static str, &'static str) {
    match status {
        "disetujui" => ("Terverifikasi", "pill pill-ok", "verified"),
        "ditolak" => ("Ditolak", "pill pill-danger", "block"),
        _ => ("Menunggu Verifikasi", "pill pill-warn", "hourglass_top"),
    }
}

/// Kartu profil PO. `show_status` menampilkan status review (dashboard/admin);
/// tanpanya hanya badge centang untuk PO terverifikasi (tampilan publik).
#[component]
pub fn PoCard(profil: MerchantProfile, #[prop(optional)] show_status: bool) -> impl IntoView {
    let p = profil;
    let (st_label, st_class, st_icon) = status_label(&p.status);
    let nama = if p.nama_po.trim().is_empty() { "Nama PO Anda".to_string() } else { p.nama_po.clone() };
    let mut sub = Vec::new();
    if !p.kota.is_empty() {
        sub.push(p.kota.clone());
    }
    if let Some(t) = p.tahun_berdiri {
        sub.push(format!("Sejak {t}"));
    }
    let armada = if p.armada_aktif > 0 { p.armada_aktif } else { p.jumlah_armada };
    view! {
        <article class="po-profile">
            <div class="po-cover">
                {(!p.sampul_url.is_empty()).then(|| view! { <img src=p.sampul_url.clone() style=crate::web::foto::style(&p.sampul_url) alt="" loading="lazy" /> })}
            </div>
            <div class="po-body">
                <span class="po-avatar">
                    {if p.logo_url.is_empty() {
                        view! { <span>{initials(&nama)}</span> }.into_any()
                    } else {
                        view! { <img src=p.logo_url.clone() style=crate::web::foto::style(&p.logo_url) alt=format!("Logo {nama}") /> }.into_any()
                    }}
                </span>
                <div class="po-name">
                    <h3>
                        {nama.clone()}
                        {(p.status == "disetujui").then(|| view! { <Icon name="verified" filled=true class="verified" /> })}
                    </h3>
                    <p>
                        <Icon name="location_on" />
                        {if sub.is_empty() { "Kota basis PO".to_string() } else { sub.join(" · ") }}
                    </p>
                </div>
                {show_status
                    .then(|| {
                        view! {
                            <span class=st_class>
                                <Icon name=st_icon />
                                {st_label}
                            </span>
                        }
                    })}
            </div>
            {(!p.layanan.is_empty())
                .then(|| {
                    view! {
                        <div class="po-tags">
                            {p.layanan.iter().map(|l| view! { <span class="pill pill-primary">{l.clone()}</span> }).collect_view()}
                        </div>
                    }
                })}
            <p class="po-desc">
                {if p.deskripsi.is_empty() { "Deskripsi singkat PO akan tampil di sini.".to_string() } else { p.deskripsi.clone() }}
            </p>
            <div class="po-stats">
                <span>
                    <strong>{armada}</strong>
                    <small>"Armada"</small>
                </span>
                <span>
                    <strong>{p.trayek_aktif}</strong>
                    <small>"Trayek aktif"</small>
                </span>
                <span>
                    <strong>{p.tahun_berdiri.map(|t| t.to_string()).unwrap_or_else(|| "-".into())}</strong>
                    <small>"Berdiri"</small>
                </span>
            </div>
        </article>
    }
}

/// Signal-signal form profil PO.
#[derive(Clone, Copy)]
pub struct ProfileSignals {
    pub nama_po: RwSignal<String>,
    pub kota: RwSignal<String>,
    pub alamat: RwSignal<String>,
    pub deskripsi: RwSignal<String>,
    pub tahun: RwSignal<String>,
    pub jumlah_armada: RwSignal<String>,
    pub layanan: RwSignal<Vec<String>>,
    pub pemilik: RwSignal<String>,
    pub email: RwSignal<String>,
    pub izin: RwSignal<String>,
    pub dokumen: RwSignal<String>,
    pub logo: RwSignal<String>,
    pub sampul: RwSignal<String>,
}

impl ProfileSignals {
    pub fn new(p: Option<&MerchantProfile>) -> Self {
        let d = MerchantProfile::default();
        let p = p.unwrap_or(&d);
        Self {
            nama_po: RwSignal::new(p.nama_po.clone()),
            kota: RwSignal::new(p.kota.clone()),
            alamat: RwSignal::new(p.alamat.clone()),
            deskripsi: RwSignal::new(p.deskripsi.clone()),
            tahun: RwSignal::new(p.tahun_berdiri.map(|t| t.to_string()).unwrap_or_default()),
            jumlah_armada: RwSignal::new(if p.jumlah_armada > 0 { p.jumlah_armada.to_string() } else { String::new() }),
            layanan: RwSignal::new(p.layanan.clone()),
            pemilik: RwSignal::new(p.nama_pemilik.clone()),
            email: RwSignal::new(p.email.clone()),
            izin: RwSignal::new(p.nomor_izin.clone()),
            dokumen: RwSignal::new(p.dokumen_url.clone()),
            logo: RwSignal::new(p.logo_url.clone()),
            sampul: RwSignal::new(p.sampul_url.clone()),
        }
    }

    pub fn input(&self) -> NewMerchantProfile {
        NewMerchantProfile {
            nama_po: self.nama_po.get_untracked(),
            kota: self.kota.get_untracked(),
            alamat: self.alamat.get_untracked(),
            deskripsi: self.deskripsi.get_untracked(),
            tahun_berdiri: self.tahun.get_untracked(),
            jumlah_armada: self.jumlah_armada.get_untracked().trim().parse().unwrap_or(0),
            layanan: self.layanan.get_untracked().join(","),
            nama_pemilik: self.pemilik.get_untracked(),
            email: self.email.get_untracked(),
            nomor_izin: self.izin.get_untracked(),
            dokumen_url: self.dokumen.get_untracked(),
            logo_url: self.logo.get_untracked(),
            sampul_url: self.sampul.get_untracked(),
        }
    }

    /// Profil sementara untuk pratinjau langsung (reaktif).
    pub fn preview(&self, status: &str) -> MerchantProfile {
        MerchantProfile {
            nama_po: self.nama_po.get(),
            kota: self.kota.get(),
            deskripsi: self.deskripsi.get(),
            tahun_berdiri: self.tahun.get().trim().parse().ok(),
            jumlah_armada: self.jumlah_armada.get().trim().parse().unwrap_or(0),
            layanan: self.layanan.get(),
            logo_url: self.logo.get(),
            sampul_url: self.sampul.get(),
            status: status.to_string(),
            ..Default::default()
        }
    }
}

/// Isian profil PO. `uploads` = tampilkan unggah logo/sampul/dokumen (butuh
/// login mitra; saat mendaftar belum bisa, dilengkapi dari dashboard).
#[component]
pub fn ProfileFields(sig: ProfileSignals, #[prop(optional)] uploads: bool) -> impl IntoView {
    view! {
        <TextField label="Nama PO" icon="directions_bus" placeholder="PO Sinar Jaya Mandiri" value=sig.nama_po />
        <div class="field-grid">
            <TextField label="Kota Basis" icon="location_city" placeholder="Solo" value=sig.kota />
            <TextField label="Tahun Berdiri" icon="event" placeholder="1998" value=sig.tahun number=true />
        </div>
        <TextField label="Alamat Kantor / Pool" icon="home_work" placeholder="Jl. Adi Sucipto No. 10, Solo" value=sig.alamat />
        <div class="field">
            <span class="field-label">"Layanan"</span>
            <div class="chip-row">
                {LAYANAN
                    .into_iter()
                    .map(|l| {
                        view! {
                            <button
                                type="button"
                                class=move || if sig.layanan.get().iter().any(|x| x == l) { "filter-chip active" } else { "filter-chip" }
                                on:click=move |_| {
                                    sig.layanan.update(|v| {
                                        if let Some(i) = v.iter().position(|x| x == l) {
                                            v.remove(i);
                                        } else {
                                            v.push(l.to_string());
                                        }
                                    })
                                }
                            >
                                {l}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
        <TextField label="Jumlah Armada" icon="airport_shuttle" placeholder="12" value=sig.jumlah_armada number=true />
        <label class="field">
            <span class="field-label">"Deskripsi Singkat PO"</span>
            <textarea
                placeholder="Melayani trayek Solo–Jakarta sejak 1998 dengan armada eksekutif & sleeper…"
                prop:value=move || sig.deskripsi.get()
                on:input=move |ev| sig.deskripsi.set(event_target_value(&ev))
            ></textarea>
        </label>
        {uploads
            .then(|| {
                view! {
                    <div class="field-grid">
                        <ImageUploadField url=sig.logo aspect="1 / 1" label="Logo PO" hint="Persegi, mis. 512×512 px" />
                        <ImageUploadField url=sig.sampul aspect="3 / 1" label="Foto Sampul" hint="Lebar, mis. 1200×400 px" />
                    </div>
                }
            })}
        <h4 class="form-sub">
            <Icon name="shield_person" />
            "Data Verifikasi (hanya dilihat admin)"
        </h4>
        <div class="field-grid">
            <TextField label="Nama Pemilik / Direktur" icon="badge" placeholder="Budi Santoso" value=sig.pemilik />
            <TextField label="Email PO" icon="mail" placeholder="admin@po-sinarjaya.id" value=sig.email />
        </div>
        <TextField label="No. Izin Usaha / Izin Trayek" icon="description" placeholder="NIB 1234567890123" value=sig.izin />
        {uploads
            .then(|| {
                view! { <ImageUploadField url=sig.dokumen aspect="4 / 3" no_pan=true label="Foto Dokumen Izin" hint="Foto NIB / izin trayek (JPG/PNG)" /> }
            })}
    }
}
