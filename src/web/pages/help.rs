//! web/pages/help.rs — "/bantuan" publik: FAQ berdasarkan fitur yang benar-
//! benar ada, pencarian, dan saluran CS (WhatsApp & telepon ke ADMIN_PHONE).

use leptos::prelude::*;

use crate::web::api::get_cs_contact;
use crate::web::app::SessionResource;
use crate::web::components::{wa_link, AccountTabs, Icon, PageHead};

#[derive(Clone, Copy, PartialEq)]
enum Kat {
    Semua,
    Akun,
    Tiket,
    Sewa,
    Jadwal,
}

/// (kategori, pertanyaan, jawaban) — hanya fitur yang sungguh tersedia.
const FAQ: &[(Kat, &str, &str)] = &[
    (Kat::Akun, "Bagaimana cara mengganti kata sandi?",
     "Buka Akun → Keamanan → Kata Sandi Akun → Ubah. Masukkan sandi saat ini lalu sandi baru (minimal 8 karakter, huruf besar & kecil, dan angka). Anda juga bisa sekaligus mengeluarkan semua perangkat lain."),
    (Kat::Akun, "Saya lupa kata sandi, bagaimana masuk?",
     "Di halaman Masuk, pilih \"Lupa password?\" dan masukkan nomor WhatsApp terdaftar. Password baru dikirim lewat WhatsApp dan berlaku 24 jam; password lama tetap bisa dipakai sampai password baru digunakan untuk masuk."),
    (Kat::Akun, "Bagaimana mengganti nomor WhatsApp akun?",
     "Buka Akun → Ganti Nomor WhatsApp. Kode OTP dikirim ke nomor BARU (berlaku 10 menit) untuk membuktikan nomor itu milik Anda."),
    (Kat::Akun, "Akun saya dibekukan, bagaimana membukanya?",
     "Gunakan \"Lupa password?\" di halaman Masuk. Masuk dengan password baru yang dikirim ke WhatsApp terdaftar akan membuka kembali akun Anda secara otomatis."),
    (Kat::Akun, "Bagaimana mengeluarkan perangkat yang tidak saya kenal?",
     "Buka Akun → Keamanan → Perangkat & Sesi. Tekan ikon keluar di samping perangkat tersebut, atau \"Keluar dari Semua Perangkat Lain\"."),
    (Kat::Tiket, "Bagaimana cara memesan kursi?",
     "Di Beranda pilih tanggal di Kalender Tarif & Kursi atau cari tujuan, tekan \"Pilih Kursi\", ketuk kursi pada denah, isi data pemesan, lalu Konfirmasi Kursi. Kode order dan nomor kursi langsung terbit."),
    (Kat::Tiket, "Di mana e-tiket saya?",
     "Buka menu Tiket. E-tiket memuat kode order, nomor kursi, titik jemput, jam, serta nama & kontak driver. Tunjukkan kode order saat naik bus."),
    (Kat::Tiket, "Apa arti kursi berwarna merah muda?",
     "Kursi tersebut diprioritaskan bagi penumpang wanita demi keamanan dan kenyamanan perjalanan, terutama perjalanan malam."),
    (Kat::Tiket, "Kursi yang saya pilih tiba-tiba tidak tersedia?",
     "Kursi yang sama mungkin baru saja dipesan penumpang lain. Denah dimuat ulang otomatis — silakan pilih kursi lain yang masih tersedia."),
    (Kat::Tiket, "Bagaimana memberi rating perjalanan?",
     "Buka detail tiket di menu Tiket, lalu beri bintang untuk armada dan driver beserta komentar."),
    (Kat::Tiket, "Bagaimana dengan pembatalan atau perubahan jadwal?",
     "Hubungi CS LajuBus lewat WhatsApp dengan menyertakan kode order Anda untuk dibantu."),
    (Kat::Sewa, "Bagaimana menyewa bus untuk rombongan?",
     "Buka menu Charter, isi Kalkulator Sewa Bus (titik jemput, tujuan, tanggal, jumlah penumpang), pilih armada, lalu kirim permintaan. CS atau mitra PO akan menghubungi Anda lewat WhatsApp."),
    (Kat::Sewa, "Apakah harga di katalog sudah final?",
     "Harga katalog adalah tarif harian dari mitra PO. Harga final, fasilitas, dan titik jemput dikonfirmasi tertulis lewat WhatsApp sebelum berangkat."),
    (Kat::Sewa, "Bisakah membuat paket wisata sendiri?",
     "Bisa. Di menu Wisata, isi form \"Buat Rencana Wisata Sendiri\" — tim LajuBus menyusun itinerary dan penawarannya."),
    (Kat::Jadwal, "Bagaimana melihat bus yang berangkat hari ini?",
     "Kalender Tarif & Kursi di Beranda otomatis menampilkan hari ini. Pilih tanggal lain untuk melihat bus beserta tarif termurahnya, dan saring berdasarkan waktu keberangkatan."),
    (Kat::Jadwal, "Kenapa jadwal tidak bisa dipesan?",
     "Jadwal yang sudah lewat atau kursinya habis tidak bisa dipesan. Coba tanggal lain di kalender."),
    (Kat::Jadwal, "Bagaimana menjadi Mitra PO di LajuBus?",
     "Daftar akun dan pilih peran \"Mitra PO\". Setelah verifikasi OTP, Anda bisa mengelola armada, jadwal, denah kursi, paket wisata, dan bus sewa dari Dashboard Mitra."),
];

#[component]
pub fn HelpPage() -> impl IntoView {
    let session = use_context::<SessionResource>().expect("SessionResource missing");
    let cs = Resource::new(|| (), |_| get_cs_contact());
    let q = RwSignal::new(String::new());
    let kat = RwSignal::new(Kat::Semua);
    let open = RwSignal::new(None::<usize>);

    let tile = move |k: Kat, icon: &'static str, title: &'static str, sub: &'static str| {
        view! {
            <button
                type="button"
                class=move || if kat.get() == k { "card help-tile active" } else { "card help-tile" }
                on:click=move |_| {
                    kat.set(if kat.get_untracked() == k { Kat::Semua } else { k });
                    open.set(None);
                }
            >
                <span class="why-icon w-blue">
                    <Icon name=icon />
                </span>
                <strong>{title}</strong>
                <small>{sub}</small>
            </button>
        }
    };

    view! {
        <div class="page page-narrow">
            <Suspense fallback=|| ()>
                {move || {
                    match session.get().and_then(|r| r.ok()).flatten() {
                        Some(_) => view! { <AccountTabs active="bantuan" /> }.into_any(),
                        None => view! { <PageHead title="Pusat Bantuan" back="/" /> }.into_any(),
                    }
                }}
            </Suspense>

            <label class="card help-search">
                <Icon name="search" />
                <input
                    type="search"
                    placeholder="Cari solusi (cth: lupa sandi, kursi, sewa bus)…"
                    prop:value=move || q.get()
                    on:input=move |ev| {
                        q.set(event_target_value(&ev));
                        open.set(None);
                    }
                />
            </label>

            <section class="fraud-card">
                <span class="fraud-icon">
                    <Icon name="shield" filled=true />
                </span>
                <div>
                    <strong>"Waspada Penipuan Akun"</strong>
                    <p>
                        "Layanan resmi LajuBus " <b>"tidak pernah meminta kode OTP"</b>
                        " atau kata sandi Anda melalui media apa pun."
                    </p>
                </div>
            </section>

            <div class="section-head">
                <h2>
                    <Icon name="bolt" class="accent-orange" />
                    "Kendala Populer"
                </h2>
                <span class="label-caps">"Paling dicari"</span>
            </div>
            <div class="help-tiles">
                {tile(Kat::Akun, "manage_accounts", "Akun & Sandi", "Lupa sandi, ganti nomor, perangkat")}
                {tile(Kat::Tiket, "confirmation_number", "E-Tiket & Kursi", "Pesan kursi, e-tiket, rating")}
                {tile(Kat::Sewa, "airport_shuttle", "Sewa & Wisata", "Charter rombongan, paket wisata")}
                {tile(Kat::Jadwal, "calendar_month", "Jadwal & Mitra", "Kalender, jadwal, jadi mitra PO")}
            </div>

            <div class="section-head">
                <h2>
                    <Icon name="quiz" />
                    "Pertanyaan Sering Diajukan"
                </h2>
            </div>
            <div class="faq-list">
                {move || {
                    let needle = q.get().trim().to_lowercase();
                    let k = kat.get();
                    let list: Vec<(usize, &(Kat, &str, &str))> = FAQ
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| k == Kat::Semua || f.0 == k)
                        .filter(|(_, f)| {
                            needle.is_empty() || f.1.to_lowercase().contains(&needle) || f.2.to_lowercase().contains(&needle)
                        })
                        .collect();
                    if list.is_empty() {
                        return view! {
                            <div class="card empty-state">
                                <Icon name="search_off" />
                                <p>"Tidak ada jawaban yang cocok. Hubungi CS kami di bawah."</p>
                            </div>
                        }
                            .into_any();
                    }
                    list.into_iter()
                        .map(|(i, f)| {
                            let (tanya, jawab) = (f.1, f.2);
                            view! {
                                <div class=move || if open.get() == Some(i) { "card faq open" } else { "card faq" }>
                                    <button
                                        type="button"
                                        class="faq-q"
                                        on:click=move |_| open.update(|o| *o = if *o == Some(i) { None } else { Some(i) })
                                    >
                                        <span>{tanya}</span>
                                        <Icon name="expand_more" />
                                    </button>
                                    {move || (open.get() == Some(i)).then(|| view! { <p class="faq-a">{jawab}</p> })}
                                </div>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>

            <div class="section-head">
                <h2>
                    <Icon name="support_agent" class="accent-orange" />
                    "Saluran Bantuan 24/7"
                </h2>
                <span class="pill pill-ok">
                    <i class="live-dot"></i>
                    "Aktif"
                </span>
            </div>
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    let c = cs.get().and_then(|r| r.ok()).unwrap_or_default();
                    if c.is_empty() {
                        return view! { <p class="empty">"Kontak CS belum diatur."</p> }.into_any();
                    }
                    view! {
                        <a
                            class="card channel"
                            href=wa_link(&c, "Halo CS LajuBus 👋 Saya butuh bantuan.")
                            target="_blank"
                            rel="noopener"
                        >
                            <span class="channel-icon ch-wa">
                                <Icon name="chat" />
                            </span>
                            <div>
                                <strong>
                                    "WhatsApp CS LajuBus"
                                    <span class="pill pill-primary">"Prioritas"</span>
                                </strong>
                                <small>"Kirim pertanyaan beserta kode order Anda"</small>
                            </div>
                            <Icon name="chevron_right" />
                        </a>
                        <a class="card channel" href=format!("tel:+{c}")>
                            <span class="channel-icon ch-call">
                                <Icon name="call" />
                            </span>
                            <div>
                                <strong>"Telepon CS"</strong>
                                <small class="accent-orange">{format!("+{c}")}</small>
                            </div>
                            <Icon name="phone_in_talk" />
                        </a>
                    }
                        .into_any()
                }}
            </Suspense>
            <p class="app-version">"LajuBus · Layanan Pelanggan Terpadu"</p>
        </div>
    }
}
