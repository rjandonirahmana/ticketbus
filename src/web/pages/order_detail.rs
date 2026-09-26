use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::web::api::{create_rating, get_order_detail};
use crate::web::components::{
    clean_error, format_rupiah, format_tanggal, spawn_client, Icon, PageHead, RatingInput, RouteTimeline,
};
use crate::web::models::NewRating;

#[component]
pub fn OrderDetailPage() -> impl IntoView {
    let params = use_params_map();
    let order_id = move || params.get().get("id").unwrap_or_default();

    let detail = Resource::new(order_id, |id| async move { get_order_detail(id).await });

    let rating_armada = RwSignal::new(0i32);
    let rating_driver = RwSignal::new(0i32);
    let komentar = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let ok_msg = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit_rating = move |_| {
        let Some(id) = detail.get().and_then(|r| r.ok()).flatten().map(|d| d.id) else {
            return;
        };
        if rating_armada.get_untracked() == 0 || rating_driver.get_untracked() == 0 {
            error.set("Beri rating armada dan driver (1-5)".into());
            return;
        }
        let input = NewRating {
            order_id: id,
            rating_armada: rating_armada.get_untracked(),
            rating_driver: rating_driver.get_untracked(),
            komentar: komentar.get_untracked(),
        };
        error.set(String::new());
        busy.set(true);
        spawn_client(async move {
            match create_rating(input).await {
                Ok(_) => {
                    ok_msg.set("Terima kasih atas rating Anda!".into());
                    detail.refetch();
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="page">
            <PageHead title="E-Tiket" back="/orders" subtitle="Tunjukkan kode order saat naik bus" />
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    let d = detail.get().and_then(|r| r.ok()).flatten();
                    match d {
                        None => {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="search_off" />
                                    <p>"Order tidak ditemukan."</p>
                                </div>
                            }
                                .into_any()
                        }
                        Some(o) => {
                            let driver = if o.driver_nama.is_empty() {
                                "Akan diinformasikan".to_string()
                            } else if o.driver_telp.is_empty() {
                                o.driver_nama.clone()
                            } else {
                                format!("{} · {}", o.driver_nama, o.driver_telp)
                            };
                            view! {
                                <article class="eticket">
                                    <header class="eticket-head">
                                        <div>
                                            <span class="label-caps">"Kode Order"</span>
                                            <strong class="eticket-code">{o.kode_order.clone()}</strong>
                                        </div>
                                        <span class="pill pill-glass">
                                            <Icon name="check_circle" filled=true />
                                            {if o.status == "paid" { "Lunas · Tiket Terbit".to_string() } else { o.status.clone() }}
                                        </span>
                                    </header>
                                    <div class="eticket-body">
                                        <h3 class="eticket-op">
                                            <Icon name="directions_bus" />
                                            {o.armada_name.clone()}
                                            <Icon name="verified" filled=true class="verified" />
                                        </h3>
                                        <RouteTimeline
                                            jam=o.jam.clone()
                                            from=o.lokasi_jemput.clone()
                                            to=o.tujuan.clone()
                                            mid=format_tanggal(&o.tanggal)
                                        />
                                    </div>
                                    <div class="perforation"></div>
                                    <dl class="eticket-grid">
                                        <div>
                                            <dt>"Pemesan"</dt>
                                            <dd>{o.nama_pemesan.clone()}</dd>
                                        </div>
                                        <div>
                                            <dt>"No. HP"</dt>
                                            <dd>{o.telp_pemesan.clone()}</dd>
                                        </div>
                                        <div>
                                            <dt>{if o.kursi.is_empty() { "Jumlah Kursi" } else { "Nomor Kursi" }}</dt>
                                            <dd class=if o.kursi.is_empty() { "" } else { "seat-codes" }>
                                                {if o.kursi.is_empty() {
                                                    format!("{} kursi", o.jumlah_tiket)
                                                } else {
                                                    o.kursi.join(", ")
                                                }}
                                            </dd>
                                        </div>
                                        <div>
                                            <dt>"Driver"</dt>
                                            <dd>{driver}</dd>
                                        </div>
                                    </dl>
                                    <div class="bill eticket-bill">
                                        <div class="bill-row">
                                            <span>
                                                {format!("Tarif Rp {} × {}", format_rupiah(o.harga_satuan), o.jumlah_tiket)}
                                            </span>
                                            <span class="num">{format!("Rp {}", format_rupiah(o.total_harga))}</span>
                                        </div>
                                        <div class="bill-row bill-total">
                                            <span>"Total Dibayar"</span>
                                            <span class="num">{format!("Rp {}", format_rupiah(o.total_harga))}</span>
                                        </div>
                                    </div>
                                </article>

                                {if o.sudah_dirating {
                                    view! {
                                        <p class="alert alert-ok">
                                            <Icon name="star" filled=true />
                                            "Anda sudah memberi rating untuk perjalanan ini."
                                        </p>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <section class="card form-card">
                                            <div class="step-head">
                                                <span class="step-icon">
                                                    <Icon name="reviews" />
                                                </span>
                                                <div>
                                                    <h3>"Beri Rating Perjalanan"</h3>
                                                    <p>"Bantu penumpang lain memilih armada terbaik"</p>
                                                </div>
                                            </div>
                                            <div class="rating-row">
                                                <span class="field-label">"Armada"</span>
                                                <RatingInput value=rating_armada />
                                            </div>
                                            <div class="rating-row">
                                                <span class="field-label">"Driver"</span>
                                                <RatingInput value=rating_driver />
                                            </div>
                                            <label class="field">
                                                <span class="field-label">"Komentar (opsional)"</span>
                                                <textarea
                                                    placeholder="Ceritakan pengalamanmu…"
                                                    prop:value=move || komentar.get()
                                                    on:input=move |ev| komentar.set(event_target_value(&ev))
                                                ></textarea>
                                            </label>
                                            <button
                                                type="button"
                                                class="btn btn-cta btn-block"
                                                on:click=submit_rating
                                                disabled=move || busy.get()
                                            >
                                                <Icon name="send" />
                                                {move || if busy.get() { "Mengirim…" } else { "Kirim Rating" }}
                                            </button>
                                            {move || {
                                                (!error.get().is_empty())
                                                    .then(|| view! { <p class="alert alert-error">{error.get()}</p> })
                                            }}
                                            {move || {
                                                (!ok_msg.get().is_empty())
                                                    .then(|| view! { <p class="alert alert-ok">{ok_msg.get()}</p> })
                                            }}
                                        </section>
                                    }
                                        .into_any()
                                }}
                            }
                                .into_any()
                        }
                    }
                }}
            </Suspense>
        </div>
    }
}
