//! Banner "Promo & Info Spesial": `BannerSlide` (satu banner — juga dipakai
//! pratinjau di panel admin) dan `BannerCarousel` (deret geser di beranda).

use leptos::prelude::*;

use super::Icon;
use crate::web::api::list_banners;
use crate::web::models::Banner;

/// Salin teks ke clipboard (klien saja; gagal diam-diam di browser lama).
fn copy_text(text: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(w) = web_sys::window() {
        let _ = w.navigator().clipboard().write_text(text);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}

/// Satu banner. `preview` = tampilan di panel admin: tanpa tautan & tombol salin.
#[component]
pub fn BannerSlide(banner: Banner, #[prop(optional)] preview: bool) -> impl IntoView {
    let b = banner;
    let has_img = !b.gambar_url.is_empty();
    let has_text = !b.judul.is_empty();
    let class = format!(
        "banner-slide tema-{}{}{}",
        b.tema,
        if has_img { " has-img" } else { "" },
        if has_text { "" } else { " img-only" }
    );
    let external = b.link_url.starts_with("http");
    let copied = RwSignal::new(false);
    let kode = b.kode_promo.clone();

    view! {
        <div class=class>
            {has_img.then(|| view! { <img class="banner-img" src=b.gambar_url.clone() alt=b.judul.clone() loading="lazy" /> })}
            {(!has_img).then(|| view! { <span class="banner-glow" aria-hidden="true"></span> })}
            {(!preview && !b.link_url.is_empty())
                .then(|| {
                    view! {
                        <a
                            class="banner-hit"
                            href=b.link_url.clone()
                            target=external.then_some("_blank")
                            rel=external.then_some("noopener")
                            aria-label=if has_text { b.judul.clone() } else { "Buka promo".to_string() }
                        ></a>
                    }
                })}
            {has_text
                .then(|| {
                    view! {
                        <div class="banner-body">
                            <div class="banner-text">
                                {(!b.label.is_empty()).then(|| view! { <span class="banner-label">{b.label.clone()}</span> })}
                                <h3>{b.judul.clone()}</h3>
                                {(!b.subjudul.is_empty()).then(|| view! { <p>{b.subjudul.clone()}</p> })}
                            </div>
                            {if !kode.is_empty() {
                                let k = kode.clone();
                                view! {
                                    <div class="banner-side">
                                        <span class="banner-code">{kode.clone()}</span>
                                        <button
                                            type="button"
                                            class="banner-copy"
                                            disabled=preview
                                            on:click=move |_| {
                                                copy_text(&k);
                                                copied.set(true);
                                            }
                                        >
                                            {move || {
                                                let (icon, text) = if copied.get() {
                                                    ("check", "Tersalin")
                                                } else {
                                                    ("content_copy", "Salin")
                                                };
                                                view! {
                                                    <Icon name=icon />
                                                    {text}
                                                }
                                            }}
                                        </button>
                                    </div>
                                }
                                    .into_any()
                            } else if !b.cta_label.is_empty() {
                                view! {
                                    <div class="banner-side">
                                        <span class="banner-cta">
                                            {b.cta_label.clone()}
                                            <Icon name="arrow_forward" />
                                        </span>
                                    </div>
                                }
                                    .into_any()
                            } else {
                                ().into_any()
                            }}
                        </div>
                    }
                })}
        </div>
    }
}

/// Banner cadangan saat admin belum menayangkan banner apa pun.
fn fallback_banner() -> Banner {
    Banner {
        id: String::new(),
        judul: "Sewa bus untuk rombongan?".into(),
        subjudul: "Mitra PO LajuBus siap untuk study tour, ziarah, dan perjalanan keluarga.".into(),
        label: "Spesial Rombongan".into(),
        kode_promo: String::new(),
        cta_label: "Lihat Paket".into(),
        link_url: "/wisata".into(),
        gambar_url: String::new(),
        tema: "sapphire".into(),
        mulai: String::new(),
        selesai: String::new(),
        urutan: 0,
        aktif: true,
    }
}

/// Deret banner geser (scroll-snap) + titik penanda di beranda.
#[component]
pub fn BannerCarousel() -> impl IntoView {
    let banners = Resource::new(|| (), |_| list_banners());
    let track = NodeRef::<leptos::html::Div>::new();
    let active = RwSignal::new(0usize);

    let list = move || {
        let v = banners.get().and_then(|r| r.ok()).unwrap_or_default();
        if v.is_empty() {
            vec![fallback_banner()]
        } else {
            v
        }
    };
    let on_scroll = move |_| {
        if let Some(el) = track.get() {
            let n = el.child_element_count().max(1) as f64;
            let step = el.scroll_width() as f64 / n;
            if step > 0.0 {
                active.set((el.scroll_left() as f64 / step).round() as usize);
            }
        }
    };
    let go = move |i: usize| {
        if let Some(el) = track.get() {
            let n = el.child_element_count().max(1) as f64;
            let step = el.scroll_width() as f64 / n;
            el.scroll_to_with_x_and_y(step * i as f64, 0.0);
        }
    };

    view! {
        <section class="section banner-section">
            <div class="section-head">
                <h2>
                    <Icon name="local_offer" />
                    "Promo & Info Spesial"
                </h2>
                <Suspense fallback=|| ()>
                    {move || {
                        let n = list().len();
                        (n > 1)
                            .then(|| {
                                view! {
                                    <div class="banner-dots">
                                        {(0..n)
                                            .map(|i| {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class=move || if active.get() == i { "dot active" } else { "dot" }
                                                        aria-label=format!("Banner {}", i + 1)
                                                        on:click=move |_| go(i)
                                                    ></button>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                            })
                    }}
                </Suspense>
            </div>
            <Suspense fallback=|| view! { <div class="skeleton-card banner-skeleton"></div> }>
                <div class="banner-track" node_ref=track on:scroll=on_scroll>
                    {move || list().into_iter().map(|b| view! { <BannerSlide banner=b /> }).collect_view()}
                </div>
            </Suspense>
        </section>
    }
}
