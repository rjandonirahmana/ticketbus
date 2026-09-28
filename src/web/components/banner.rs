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
            {has_img.then(|| view! { <img
                        class="banner-img"
                        src=b.gambar_url.clone()
                        style=crate::web::foto::style(&b.gambar_url)
                        alt=b.judul.clone()
                        loading="lazy"
                    /> })}
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

/// Banner beranda: SATU banner tampil; bila lebih dari satu bisa digeser
/// (swipe di HP, drag mouse di desktop),
/// panah, titik penanda, dan ganti otomatis tiap 5 detik — berhenti sejenak
/// selama pengguna berinteraksi / menyorot banner.
#[component]
pub fn BannerCarousel() -> impl IntoView {
    const AUTO_MS: u64 = 5_000;
    /// Jeda auto-geser setelah interaksi terakhir.
    const DIAM_MS: f64 = 8_000.0;
    const GAP_PX: f64 = 12.0;

    let banners = Resource::new(|| (), |_| list_banners());
    let track = NodeRef::<leptos::html::Div>::new();
    let active = RwSignal::new(0usize);
    let count = RwSignal::new(0usize);
    let hover = StoredValue::new(false);
    let last_touch = StoredValue::new(0.0f64);
    // Drag mouse: (x awal, scrollLeft awal, sudah bergeser?)
    let drag = StoredValue::new(None::<(f64, f64, bool)>);
    let dragged = StoredValue::new(false);
    let dragging = RwSignal::new(false);

    let list = move || {
        let v = banners.get().and_then(|r| r.ok()).unwrap_or_default();
        if v.is_empty() {
            vec![fallback_banner()]
        } else {
            v
        }
    };
    // Jarak antar-awal banner = lebar satu banner + celah.
    let step = move || -> Option<(web_sys::HtmlDivElement, f64)> {
        let el = track.get()?;
        let w = el.first_element_child()?.client_width() as f64;
        (w > 0.0).then_some((el, w + GAP_PX))
    };
    let go = move |i: usize| {
        if let Some((el, st)) = step() {
            el.scroll_to_with_x_and_y(st * i as f64, 0.0);
        }
    };
    let geser = move |arah: i32| {
        let n = count.get_untracked().max(1) as i32;
        let next = (active.get_untracked() as i32 + arah).rem_euclid(n) as usize;
        last_touch.set_value(super::now_ms());
        go(next);
    };
    let on_scroll = move |_| {
        if let Some((el, st)) = step() {
            let n = count.get_untracked();
            let max = (el.scroll_width() - el.client_width()) as f64;
            let left = el.scroll_left() as f64;
            // Ujung kanan = banner terakhir (pembulatan scroll bisa kurang 1-2 px).
            let i = if n > 0 && left >= max - 2.0 { n - 1 } else { (left / st).round() as usize };
            active.set(i.min(n.saturating_sub(1)));
        }
    };

    let on_down = move |ev: web_sys::PointerEvent| {
        last_touch.set_value(super::now_ms());
        if ev.pointer_type() != "mouse" || ev.button() != 0 {
            return; // sentuhan: biarkan geser native
        }
        if let Some(el) = track.get() {
            drag.set_value(Some((ev.client_x() as f64, el.scroll_left() as f64, false)));
            dragged.set_value(false);
        }
    };
    let on_move = move |ev: web_sys::PointerEvent| {
        let Some((x0, left0, moved)) = drag.get_value() else { return };
        let dx = ev.client_x() as f64 - x0;
        if !moved && dx.abs() < 5.0 {
            return;
        }
        if !moved {
            drag.set_value(Some((x0, left0, true)));
            dragging.set(true);
        }
        ev.prevent_default();
        if let Some(el) = track.get() {
            el.set_scroll_left((left0 - dx) as i32);
        }
    };
    let on_up = move |_: web_sys::PointerEvent| {
        let Some((_, _, moved)) = drag.get_value() else { return };
        drag.set_value(None);
        if moved {
            dragged.set_value(true);
            dragging.set(false);
            // Kembalikan snap ke banner terdekat.
            let i = active.get_untracked();
            go(i);
        }
    };
    // Klik yang sebenarnya akhir dari drag tidak boleh membuka tautan banner.
    let on_click = move |ev: web_sys::MouseEvent| {
        if dragged.get_value() {
            dragged.set_value(false);
            ev.prevent_default();
            ev.stop_propagation();
        }
    };

    // Auto-geser (klien saja).
    #[cfg(target_arch = "wasm32")]
    {
        let handle = leptos::prelude::set_interval_with_handle(
            move || {
                let diam = super::now_ms() - last_touch.get_value() > DIAM_MS;
                if count.get_untracked() > 1 && diam && !hover.get_value() && drag.get_value().is_none() {
                    geser(1);
                    // geser() menandai interaksi; auto-geser bukan interaksi.
                    last_touch.set_value(0.0);
                }
            },
            std::time::Duration::from_millis(AUTO_MS),
        );
        if let Ok(h) = handle {
            on_cleanup(move || h.clear());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (AUTO_MS, DIAM_MS);

    view! {
        <section class="banner-section" aria-label="Promo & Info Spesial">
            <Suspense fallback=|| view! { <div class="skeleton-card banner-skeleton"></div> }>
                {move || {
                    let items = list();
                    let n = items.len();
                    count.set(n);
                    view! {
                        <div
                            class="banner-wrap"
                            on:mouseenter=move |_| hover.set_value(true)
                            on:mouseleave=move |_| hover.set_value(false)
                        >
                            <div
                                class=move || match (n > 1, dragging.get()) {
                                    (false, _) => "banner-track single",
                                    (true, true) => "banner-track dragging",
                                    (true, false) => "banner-track",
                                }
                                node_ref=track
                                on:scroll=on_scroll
                                on:pointerdown=on_down
                                on:pointermove=on_move
                                on:pointerup=on_up
                                on:pointerleave=on_up
                                on:click=on_click
                                on:touchstart=move |_| last_touch.set_value(super::now_ms())
                            >
                                {items.into_iter().map(|b| view! { <BannerSlide banner=b /> }).collect_view()}
                            </div>
                            {(n > 1)
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
                                                            on:click=move |_| {
                                                                last_touch.set_value(super::now_ms());
                                                                go(i)
                                                            }
                                                        ></button>
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                        <button type="button" class="banner-nav prev" aria-label="Banner sebelumnya" on:click=move |_| geser(-1)>
                                            <Icon name="chevron_left" />
                                        </button>
                                        <button type="button" class="banner-nav next" aria-label="Banner berikutnya" on:click=move |_| geser(1)>
                                            <Icon name="chevron_right" />
                                        </button>
                                    }
                                })}
                        </div>
                    }
                }}
            </Suspense>
        </section>
    }
}
