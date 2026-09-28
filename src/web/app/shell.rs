//! web/app/shell.rs — Shell HTML SSR (server-only).

use leptos::prelude::*;
use leptos_meta::*;

use super::router::App;

pub fn shell(options: leptos::config::LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="id" data-theme="light">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="theme-color" content="#0f4c81" />

                // Baca preferensi dark-mode SEBELUM CSS di-parse — mencegah kedipan
                // terang→gelap (FOUC) pada kunjungan yang sudah memilih gelap.
                <script inner_html=r#"(function(){try{var t=localStorage.getItem('bis.theme');if(t==='dark'){document.documentElement.setAttribute('data-theme','dark');}}catch(e){}})();"# />

                <link rel="stylesheet" href="/pkg/bis.css" />

                // Ikon app LajuBus: SVG untuk browser modern, PNG untuk sisanya.
                <link rel="icon" type="image/svg+xml" href="/icon.svg" />
                <link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png" />
                <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
                <link rel="manifest" href="/manifest.webmanifest" />

                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="" />
                <link
                    href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=Space+Grotesk:wght@500;600;700&display=swap"
                    rel="stylesheet"
                />
                // `display=block`: nama ligatur ikon ("directions_bus") tak sempat
                // tampil sebagai teks selama font ikon belum termuat.
                <link
                    href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,300..600,0..1,0&display=block"
                    rel="stylesheet"
                />

                // Peta: Leaflet + MapLibre GL (peta vektor OpenFreeMap, gratis
                // tanpa kunci) lewat plugin maplibre-gl-leaflet, lalu jembatan
                // lajumap.js. `defer` dan diletakkan SEBELUM HydrationScripts
                // supaya `window.LajuMap` sudah ada saat WASM berjalan (skrip
                // modul dieksekusi urut bersama skrip defer).
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/leaflet.min.css"
                    crossorigin=""
                />
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/maplibre-gl/5.6.0/maplibre-gl.css"
                    crossorigin=""
                />
                <script
                    src="https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/leaflet.min.js"
                    crossorigin=""
                    defer=true
                ></script>
                <script
                    src="https://cdnjs.cloudflare.com/ajax/libs/maplibre-gl/5.6.0/maplibre-gl.min.js"
                    crossorigin=""
                    defer=true
                ></script>
                <script
                    src="https://cdn.jsdelivr.net/npm/@maplibre/maplibre-gl-leaflet@0.1.4/leaflet-maplibre-gl.min.js"
                    crossorigin=""
                    defer=true
                ></script>
                {map_tiles_script()}
                <script src="/lajumap.js" defer=true></script>

                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
    .into_any()
}

/// Tile peta dari env → `window.LAJU_TILES` (dibaca public/lajumap.js).
///
/// - `CARTO_TILE_URL`: template Leaflet, mis.
///   `https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png`.
///   Boleh memuat `{apikey}`, yang diganti `CARTO_API_KEY`.
/// - `CARTO_API_KEY`: bila URL tak memuat `{apikey}`, ditambahkan sebagai
///   query `api_key=`.
///
/// Keduanya kosong → tak menyuntik apa pun (peta memakai OpenFreeMap gratis,
/// lihat `baseLayer()` di public/lajumap.js).
/// CATATAN: tile diambil oleh BROWSER, jadi kunci ini ikut terlihat di tab
/// Network — pakai kunci yang dibatasi domain (mis. hanya lajubus.online).
fn map_tiles_script() -> Option<impl IntoView> {
    let url = tile_url(
        std::env::var("CARTO_TILE_URL").unwrap_or_default().trim(),
        std::env::var("CARTO_API_KEY").unwrap_or_default().trim(),
    )?;
    // serde_json meng-escape string; `</` dipecah agar tak menutup <script>.
    let js = format!("window.LAJU_TILES={{url:{}}};", serde_json::to_string(&url).ok()?.replace("</", "<\\/"));
    Some(view! { <script inner_html=js></script> })
}

fn tile_url(url: &str, key: &str) -> Option<String> {
    let base = if url.is_empty() {
        if key.is_empty() {
            return None;
        }
        "https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png"
    } else {
        url
    };
    if !base.starts_with("https://") {
        tracing::warn!("CARTO_TILE_URL harus diawali https:// — diabaikan");
        return None;
    }
    Some(if key.is_empty() {
        base.to_string()
    } else if base.contains("{apikey}") {
        base.replace("{apikey}", key)
    } else {
        let sep = if base.contains('?') { '&' } else { '?' };
        format!("{base}{sep}api_key={key}")
    })
}

#[cfg(test)]
mod tests {
    use super::tile_url;

    #[test]
    fn url_tile_dari_env() {
        assert_eq!(tile_url("", ""), None);
        assert_eq!(
            tile_url("", "K1").as_deref(),
            Some("https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png?api_key=K1")
        );
        assert_eq!(tile_url("https://x/{z}/{x}/{y}.png?key={apikey}", "K2").as_deref(), Some("https://x/{z}/{x}/{y}.png?key=K2"));
        assert_eq!(tile_url("https://x/{z}.png?a=1", "K3").as_deref(), Some("https://x/{z}.png?a=1&api_key=K3"));
        assert_eq!(tile_url("http://tidak-aman/{z}.png", ""), None);
    }
}
