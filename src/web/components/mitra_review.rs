//! Tab "Mitra PO" di /admin: review pendaftar — pratinjau profil PO, data
//! verifikasi (pemilik, kontak, izin, dokumen), lalu setujui / tolak dengan
//! catatan. Keputusan dikirim ke mitra lewat WhatsApp (best-effort).

use leptos::prelude::*;

use super::{clean_error, format_tanggal, spawn_client, wa_link, Icon, PoCard};
use crate::web::api::{decide_merchant, list_merchant_profiles};
use crate::web::models::MerchantProfile;

const FILTER: [(&str, &str); 4] = [
    ("menunggu", "Menunggu"),
    ("disetujui", "Disetujui"),
    ("ditolak", "Ditolak"),
    ("", "Semua"),
];

#[component]
pub fn MitraReview() -> impl IntoView {
    let filter = RwSignal::new("menunggu".to_string());
    let list = Resource::new(move || filter.get(), list_merchant_profiles);

    view! {
        <div class="section-head">
            <div>
                <h2>"Pendaftaran Mitra PO"</h2>
                <p>"Jadwal & trayek PO baru tampil ke penumpang setelah disetujui"</p>
            </div>
        </div>
        <div class="chip-row">
            {FILTER
                .into_iter()
                .map(|(v, l)| {
                    view! {
                        <button
                            type="button"
                            class=move || if filter.get() == v { "filter-chip active" } else { "filter-chip" }
                            on:click=move |_| filter.set(v.to_string())
                        >
                            {l}
                        </button>
                    }
                })
                .collect_view()}
        </div>
        <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
            {move || {
                let rows = list.get().and_then(|r| r.ok()).unwrap_or_default();
                if rows.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <Icon name="storefront" />
                            <p>"Tidak ada pendaftar pada status ini."</p>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="review-grid">
                            {rows
                                .into_iter()
                                .map(|p| view! { <ReviewCard profil=p on_done=move |_| list.refetch() /> })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }
            }}
        </Suspense>
    }
}

#[component]
fn ReviewCard(profil: MerchantProfile, #[prop(into)] on_done: Callback<()>) -> impl IntoView {
    let p = profil;
    let id = StoredValue::new(p.user_id.clone());
    let catatan = RwSignal::new(String::new());
    let tolak_open = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let tanggal = p.created_at.get(..10).map(format_tanggal).unwrap_or_default();

    let decide = move |setujui: bool| {
        busy.set(true);
        let c = catatan.get_untracked();
        spawn_client(async move {
            match decide_merchant(id.get_value(), setujui, c).await {
                Ok(()) => on_done.run(()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let baris = |icon: &'static str, label: &'static str, isi: String| {
        view! {
            <div class="review-row">
                <Icon name=icon />
                <span>{label}</span>
                <strong>{if isi.trim().is_empty() { "—".to_string() } else { isi }}</strong>
            </div>
        }
    };
    let wa = wa_link(&p.telp, &format!("Halo, kami dari LajuBus terkait pendaftaran Mitra PO *{}*.", p.nama_po));
    let status = p.status.clone();
    let publik = (p.status == "disetujui").then(|| format!("/po/{}", p.user_id));

    view! {
        <article class="card review-card">
            <PoCard profil=p.clone() show_status=true />
            <div class="review-data">
                {baris("event", "Mendaftar", tanggal)}
                {baris("badge", "Pemilik", p.nama_pemilik.clone())}
                {baris("smartphone", "WhatsApp", format!("+{}", p.telp))}
                {baris("mail", "Email", p.email.clone())}
                {baris("home_work", "Alamat", p.alamat.clone())}
                {baris("description", "No. Izin", p.nomor_izin.clone())}
                {baris("airport_shuttle", "Armada (klaim / terdaftar)", format!("{} / {}", p.jumlah_armada, p.armada_aktif))}
                {baris("swap_horiz", "Trayek aktif", p.trayek_aktif.to_string())}
            </div>
            {if p.dokumen_url.is_empty() {
                view! {
                    <p class="note-card">
                        <Icon name="info" />
                        "Dokumen izin belum diunggah mitra."
                    </p>
                }
                    .into_any()
            } else {
                view! {
                    <a href=p.dokumen_url.clone() target="_blank" rel="noopener" class="review-doc">
                        <img src=p.dokumen_url.clone() alt="Dokumen izin" loading="lazy" />
                        <span>
                            <Icon name="open_in_new" />
                            "Buka dokumen izin"
                        </span>
                    </a>
                }
                    .into_any()
            }}
            {(!p.catatan_admin.is_empty())
                .then(|| {
                    view! {
                        <p class="note-card">
                            <Icon name="sticky_note_2" />
                            {format!("Catatan sebelumnya: {}", p.catatan_admin)}
                        </p>
                    }
                })}
            <div class="review-actions">
                <a href=wa target="_blank" rel="noopener" class="btn btn-soft btn-sm">
                    <Icon name="chat" />
                    "Hubungi"
                </a>
                {publik
                    .map(|href| {
                        view! {
                            <a href=href class="btn btn-soft btn-sm">
                                <Icon name="open_in_new" />
                                "Halaman PO"
                            </a>
                        }
                    })}
                {(status != "disetujui")
                    .then(|| {
                        view! {
                            <button type="button" class="btn btn-cta btn-sm" disabled=move || busy.get() on:click=move |_| decide(true)>
                                <Icon name="verified" />
                                "Setujui"
                            </button>
                        }
                    })}
                <button type="button" class="btn btn-danger-ghost btn-sm" on:click=move |_| tolak_open.update(|v| *v = !*v)>
                    <Icon name="block" />
                    {if status == "disetujui" { "Cabut / Tolak" } else { "Tolak" }}
                </button>
            </div>
            {move || {
                tolak_open
                    .get()
                    .then(|| {
                        view! {
                            <div class="review-reject">
                                <textarea
                                    placeholder="Alasan penolakan (dikirim ke mitra), mis. dokumen izin tidak terbaca"
                                    prop:value=move || catatan.get()
                                    on:input=move |ev| catatan.set(event_target_value(&ev))
                                ></textarea>
                                <button type="button" class="btn btn-danger-ghost btn-sm" disabled=move || busy.get() on:click=move |_| decide(false)>
                                    "Kirim Penolakan"
                                </button>
                            </div>
                        }
                    })
            }}
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
        </article>
    }
}
