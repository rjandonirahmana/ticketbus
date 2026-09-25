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
