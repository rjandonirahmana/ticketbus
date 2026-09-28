//! Tab "Sewa & Wisata" di dashboard mitra PO & admin: permintaan sewa masuk,
//! paket wisata, dan katalog bus sewa. Scope data (milik sendiri vs semua)
//! ditegakkan server fn — komponen ini sama untuk kedua peran.

use leptos::prelude::*;

use super::{clean_error, format_rupiah, format_tanggal, spawn_client, wa_link, Icon, ImageUploadField};
use crate::web::api::{
    create_charter_bus, create_tour_package, delete_listing, list_manage_buses, list_manage_packages,
    list_rental_requests, set_listing_active, set_rental_request_status,
};
use crate::web::models::{NewCharterBus, NewTourPackage, RentalRequest};

#[derive(Clone, Copy, PartialEq)]
enum Sub {
    Permintaan,
    Paket,
    Bus,
}

const STATUS: [(&str, &str); 4] = [("baru", "Baru"), ("dihubungi", "Dihubungi"), ("deal", "Deal"), ("batal", "Batal")];

fn status_class(s: &str) -> &'static str {
    match s {
        "baru" => "pill pill-cta",
        "dihubungi" => "pill pill-primary",
        "deal" => "pill pill-ok",
        _ => "pill pill-danger",
    }
}

#[component]
pub fn RentalManager() -> impl IntoView {
    let sub = RwSignal::new(Sub::Permintaan);
    let requests = Resource::new(|| (), |_| list_rental_requests());
    let packages = Resource::new(|| (), |_| list_manage_packages());
    let buses = Resource::new(|| (), |_| list_manage_buses());

    let tab = move |s: Sub, icon: &'static str, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || if sub.get() == s { "filter-chip active" } else { "filter-chip" }
                on:click=move |_| sub.set(s)
            >
                <Icon name=icon />
                {label}
            </button>
        }
    };

    let toggle = move |kind: &'static str, id: String, aktif: bool| {
        spawn_client(async move {
            let _ = set_listing_active(kind.to_string(), id, aktif).await;
            packages.refetch();
            buses.refetch();
        });
    };
    let remove = move |kind: &'static str, id: String| {
        spawn_client(async move {
            let _ = delete_listing(kind.to_string(), id).await;
            packages.refetch();
            buses.refetch();
        });
    };

    view! {
        <div class="chip-row">
            {tab(Sub::Permintaan, "inbox", "Permintaan")}
            {tab(Sub::Paket, "beach_access", "Paket Wisata")}
            {tab(Sub::Bus, "airport_shuttle", "Bus Sewa")}
        </div>
        <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
            {move || match sub.get() {
                Sub::Permintaan => {
                    let list = requests.get().and_then(|r| r.ok()).unwrap_or_default();
                    if list.is_empty() {
                        view! {
                            <div class="card empty-state">
                                <Icon name="inbox" />
                                <p>"Belum ada permintaan sewa. Permintaan dari halaman Charter & Wisata akan muncul di sini."</p>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="ops-grid">
                                {list
                                    .into_iter()
                                    .map(|r| view! { <RequestCard req=r on_changed=move |_| requests.refetch() /> })
                                    .collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }
                Sub::Paket => {
                    let list = packages.get().and_then(|r| r.ok()).unwrap_or_default();
                    view! {
                        <PackageForm on_saved=move |_| packages.refetch() />
                        <div class="section-head">
                            <h2>"Paket Wisata"</h2>
                            <span class="pill pill-primary">{format!("{} paket", list.len())}</span>
                        </div>
                        {if list.is_empty() {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="beach_access" />
                                    <p>"Belum ada paket wisata."</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            list.into_iter()
                                .map(|p| {
                                    let id_t = p.id.clone();
                                    let id_d = p.id.clone();
                                    let aktif = p.aktif;
                                    let harga = if p.harga_pax > 0 {
                                        format!("Rp {}/pax", format_rupiah(p.harga_pax))
                                    } else {
                                        format!("Rp {}/trip", format_rupiah(p.harga_charter))
                                    };
                                    view! {
                                        <ListingRow
                                            foto=p.foto_url
                                            title=p.judul
                                            sub=format!("{} · {} · {}", p.kawasan, p.durasi, p.pemilik)
                                            harga=harga
                                            aktif=aktif
                                            on_toggle=move |_| toggle("paket", id_t.clone(), !aktif)
                                            on_delete=move |_| remove("paket", id_d.clone())
                                        />
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    }
                        .into_any()
                }
                Sub::Bus => {
                    let list = buses.get().and_then(|r| r.ok()).unwrap_or_default();
                    view! {
                        <BusForm on_saved=move |_| buses.refetch() />
                        <div class="section-head">
                            <h2>"Katalog Bus Sewa"</h2>
                            <span class="pill pill-primary">{format!("{} bus", list.len())}</span>
                        </div>
                        {if list.is_empty() {
                            view! {
                                <div class="card empty-state">
                                    <Icon name="airport_shuttle" />
                                    <p>"Belum ada bus di katalog sewa."</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            list.into_iter()
                                .map(|b| {
                                    let id_t = b.id.clone();
                                    let id_d = b.id.clone();
                                    let aktif = b.aktif;
                                    view! {
                                        <ListingRow
                                            foto=b.foto_url
                                            title=b.nama
                                            sub=format!("{} seat · {} · {}", b.kapasitas, b.kelas, b.pemilik)
                                            harga=format!("Rp {}/hari", format_rupiah(b.harga_harian))
                                            aktif=aktif
                                            on_toggle=move |_| toggle("bus", id_t.clone(), !aktif)
                                            on_delete=move |_| remove("bus", id_d.clone())
                                        />
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    }
                        .into_any()
                }
            }}
        </Suspense>
    }
}

#[component]
fn ListingRow(
    foto: String,
    title: String,
    sub: String,
    harga: String,
    aktif: bool,
    #[prop(into)] on_toggle: Callback<()>,
    #[prop(into)] on_delete: Callback<()>,
) -> impl IntoView {
    view! {
        <div class=if aktif { "card listing-row" } else { "card listing-row inactive" }>
            {if foto.is_empty() {
                view! {
                    <span class="listing-thumb">
                        <Icon name="image" />
                    </span>
                }
                    .into_any()
            } else {
                view! { <img class="listing-thumb" style=crate::web::foto::style(&foto) src=foto.clone() alt="" loading="lazy" /> }.into_any()
            }}
            <div class="listing-main">
                <strong>{title}</strong>
                <small>{sub}</small>
                <span class="listing-price">{harga}</span>
            </div>
            <button
                type="button"
                class=if aktif { "status-toggle on" } else { "status-toggle" }
                title=if aktif { "Sembunyikan dari katalog" } else { "Tampilkan di katalog" }
                on:click=move |_| on_toggle.run(())
            >
                {if aktif { "Tayang" } else { "Draf" }}
            </button>
            <button type="button" class="icon-btn icon-btn-danger" title="Hapus" on:click=move |_| on_delete.run(())>
                <Icon name="delete" />
            </button>
        </div>
    }
}

#[component]
fn RequestCard(req: RentalRequest, #[prop(into)] on_changed: Callback<()>) -> impl IntoView {
    let r = req;
    let jenis = match r.jenis.as_str() {
        "paket" => ("Paket Wisata", "beach_access"),
        "charter" => ("Sewa Charter", "airport_shuttle"),
        _ => ("Custom", "edit_note"),
    };
    let wa = wa_link(
        &r.telp,
        &format!("Halo {} 👋 Kami dari LajuBus, menindaklanjuti permintaan *{}* Anda.", r.nama, r.item_nama),
    );
    let tanggal = match (r.tgl_berangkat.as_str(), r.tgl_pulang.as_str()) {
        ("", _) => "Tanggal fleksibel".to_string(),
        (a, "") => format_tanggal(a),
        (a, b) => format!("{} → {}", format_tanggal(a), format_tanggal(b)),
    };
    let current = r.status.clone();
    let id = r.id.clone();

    view! {
        <article class="card ops-card">
            <header class="ops-head">
                <span class="ops-num">
                    <Icon name=jenis.1 />
                </span>
                <div class="ops-title">
                    <h3>{r.item_nama.clone()}</h3>
                    <p>{format!("{} · {}", jenis.0, format_tanggal(r.created_at.get(..10).unwrap_or_default()))}</p>
                </div>
                <span class=status_class(&r.status)>
                    {STATUS.iter().find(|(k, _)| *k == r.status).map(|(_, l)| *l).unwrap_or("?")}
                </span>
            </header>
            <div class="ops-meta">
                <span class="ops-driver">
                    <Icon name="person" />
                    {format!("{} · +{}", r.nama, r.telp)}
                </span>
                {(!r.jemput.is_empty() || !r.tujuan.is_empty())
                    .then(|| {
                        view! {
                            <span class="ops-driver">
                                <Icon name="route" />
                                {format!("{} → {}", r.jemput, r.tujuan)}
                            </span>
                        }
                    })}
                <span class="ops-driver">
                    <Icon name="calendar_today" />
                    {tanggal}
                    {(!r.tipe_perjalanan.is_empty()).then(|| format!(" · {}", r.tipe_perjalanan))}
                </span>
                <span class="ops-driver">
                    <Icon name="groups" />
                    {format!("{} orang", r.jumlah_orang)}
                </span>
                {(!r.catatan.is_empty())
                    .then(|| {
                        view! {
                            <span class="ops-note">
                                <Icon name="sticky_note_2" />
                                {r.catatan.clone()}
                            </span>
                        }
                    })}
            </div>
            <div class="status-row">
                {STATUS
                    .into_iter()
                    .map(|(k, label)| {
                        let id = id.clone();
                        view! {
                            <button
                                type="button"
                                class=if current == k { "time-chip active" } else { "time-chip" }
                                on:click=move |_| {
                                    let id = id.clone();
                                    spawn_client(async move {
                                        let _ = set_rental_request_status(id, k.to_string()).await;
                                        on_changed.run(());
                                    });
                                }
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <a class="btn btn-wa btn-block" href=wa target="_blank" rel="noopener">
                <Icon name="chat" />
                "Hubungi via WhatsApp"
            </a>
        </article>
    }
}

/// Input teks berlabel dengan ikon (form paket & bus).
#[component]
pub fn TextField(
    #[prop(into)] label: String,
    #[prop(into)] icon: String,
    #[prop(into)] placeholder: String,
    value: RwSignal<String>,
    #[prop(optional)] number: bool,
) -> impl IntoView {
    view! {
        <label class="field">
            <span class="field-label">{label}</span>
            <span class="input-wrap">
                <Icon name=icon />
                <input
                    type=if number { "number" } else { "text" }
                    class=if number { "num" } else { "" }
                    placeholder=placeholder
                    prop:value=move || value.get()
                    on:input=move |ev| value.set(event_target_value(&ev))
                />
            </span>
        </label>
    }
}

#[component]
fn PackageForm(#[prop(into)] on_saved: Callback<()>) -> impl IntoView {
    let open = RwSignal::new(false);
    let judul = RwSignal::new(String::new());
    let kategori = RwSignal::new("alam".to_string());
    let kawasan = RwSignal::new(String::new());
    let durasi = RwSignal::new(String::new());
    let badge = RwSignal::new(String::new());
    let tipe = RwSignal::new(String::new());
    let rute = RwSignal::new(String::new());
    let armada = RwSignal::new(String::new());
    let fasilitas = RwSignal::new(String::new());
    let harga_pax = RwSignal::new(String::new());
    let min_pax = RwSignal::new("20".to_string());
    let harga_charter = RwSignal::new(String::new());
    let foto = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let num = |s: String| s.trim().parse::<i64>().unwrap_or(0);
    let submit = move |_| {
        let input = NewTourPackage {
            judul: judul.get_untracked(),
            kategori: kategori.get_untracked(),
            kawasan: kawasan.get_untracked(),
            durasi: durasi.get_untracked(),
            label_badge: badge.get_untracked(),
            label_tipe: tipe.get_untracked(),
            rute: rute.get_untracked(),
            armada_info: armada.get_untracked(),
            fasilitas: fasilitas.get_untracked(),
            harga_pax: num(harga_pax.get_untracked()),
            min_pax: num(min_pax.get_untracked()) as i32,
            harga_charter: num(harga_charter.get_untracked()),
            foto_url: foto.get_untracked(),
        };
        busy.set(true);
        spawn_client(async move {
            match create_tour_package(input).await {
                Ok(_) => {
                    error.set(String::new());
                    for s in [judul, kawasan, durasi, badge, tipe, rute, armada, fasilitas, harga_pax, harga_charter, foto] {
                        s.set(String::new());
                    }
                    open.set(false);
                    on_saved.run(());
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        {move || {
            if !open.get() {
                return view! {
                    <button type="button" class="action-bar" on:click=move |_| open.set(true)>
                        <span class="action-icon">
                            <Icon name="add_circle" />
                        </span>
                        <span class="action-text">
                            <strong>"Tambah Paket Wisata"</strong>
                            <small>"Tampil di halaman Wisata setelah disimpan"</small>
                        </span>
                        <Icon name="chevron_right" />
                    </button>
                }
                    .into_any();
            }
            let submit = submit.clone();
            view! {
                <section class="card form-card">
                    <div class="step-head">
                        <span class="step-icon">
                            <Icon name="beach_access" />
                        </span>
                        <div>
                            <h3>"Paket Wisata Baru"</h3>
                            <p>"Isi info paket, harga per orang dan/atau harga charter"</p>
                        </div>
                        <button type="button" class="icon-btn" title="Tutup" on:click=move |_| open.set(false)>
                            <Icon name="close" />
                        </button>
                    </div>
                    <ImageUploadField url=foto />
                    <TextField label="Judul Paket" icon="title" placeholder="Eksotisme Bromo & Malang Sejuk" value=judul />
                    <div class="field-grid">
                        <label class="field">
                            <span class="field-label">"Kategori"</span>
                            <span class="input-wrap">
                                <Icon name="category" />
                                <select prop:value=move || kategori.get() on:change=move |ev| kategori.set(event_target_value(&ev))>
                                    <option value="alam">"Wisata Alam"</option>
                                    <option value="budaya">"Budaya"</option>
                                    <option value="edukasi">"Edukasi"</option>
                                    <option value="religi">"Wisata Religi"</option>
                                    <option value="lainnya">"Lainnya"</option>
                                </select>
                            </span>
                        </label>
                        <TextField label="Kawasan" icon="map" placeholder="Malang & Bromo" value=kawasan />
                    </div>
                    <div class="field-grid">
                        <TextField label="Durasi" icon="schedule" placeholder="3 Hari 2 Malam (3D2N)" value=durasi />
                        <TextField label="Armada" icon="directions_bus" placeholder="Big Bus 48 Seat" value=armada />
                    </div>
                    <div class="field-grid">
                        <TextField label="Label Sampul (opsional)" icon="sell" placeholder="Favorit Komunitas" value=badge />
                        <TextField label="Tipe Paket (opsional)" icon="label" placeholder="All-In Wisata" value=tipe />
                    </div>
                    <label class="field">
                        <span class="field-label">"Rute / Itinerary"</span>
                        <textarea
                            placeholder="Sunrise Penanjakan Bromo, Kawah Bromo, Museum Angkut Batu, …"
                            prop:value=move || rute.get()
                            on:input=move |ev| rute.set(event_target_value(&ev))
                        ></textarea>
                    </label>
                    <label class="field">
                        <span class="field-label">"Fasilitas (pisahkan koma)"</span>
                        <textarea
                            placeholder="Bus AC Eksekutif, Sopir Wisata & BBM, Tiket Wisata & Parkir"
                            prop:value=move || fasilitas.get()
                            on:input=move |ev| fasilitas.set(event_target_value(&ev))
                        ></textarea>
                    </label>
                    <div class="field-grid">
                        <TextField label="Harga / Pax (Rp)" icon="payments" placeholder="450000" value=harga_pax number=true />
                        <TextField label="Minimal Pax" icon="groups" placeholder="20" value=min_pax number=true />
                    </div>
                    <TextField
                        label="Harga Charter / Trip (Rp, opsional)"
                        icon="airport_shuttle"
                        placeholder="9800000"
                        value=harga_charter
                        number=true
                    />
                    <button type="button" class="btn btn-cta btn-block" on:click=submit disabled=move || busy.get()>
                        <Icon name="publish" />
                        {move || if busy.get() { "Menyimpan…" } else { "Terbitkan Paket" }}
                    </button>
                    {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
                </section>
            }
                .into_any()
        }}
    }
}

#[component]
fn BusForm(#[prop(into)] on_saved: Callback<()>) -> impl IntoView {
    let open = RwSignal::new(false);
    let nama = RwSignal::new(String::new());
    let tipe_bus = RwSignal::new(String::new());
    let kelas = RwSignal::new(String::new());
    let kapasitas = RwSignal::new("45".to_string());
    let konfigurasi = RwSignal::new("2-2".to_string());
    let deskripsi = RwSignal::new(String::new());
    let fasilitas = RwSignal::new(String::new());
    let harga = RwSignal::new(String::new());
    let catatan = RwSignal::new("Bus + Driver + BBM".to_string());
    let foto = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let input = NewCharterBus {
            nama: nama.get_untracked(),
            tipe_bus: tipe_bus.get_untracked(),
            kelas: kelas.get_untracked(),
            kapasitas: kapasitas.get_untracked().trim().parse().unwrap_or(0),
            konfigurasi: konfigurasi.get_untracked(),
            deskripsi: deskripsi.get_untracked(),
            fasilitas: fasilitas.get_untracked(),
            harga_harian: harga.get_untracked().trim().parse().unwrap_or(0),
            catatan_harga: catatan.get_untracked(),
            foto_url: foto.get_untracked(),
        };
        busy.set(true);
        spawn_client(async move {
            match create_charter_bus(input).await {
                Ok(_) => {
                    error.set(String::new());
                    for s in [nama, tipe_bus, kelas, deskripsi, fasilitas, harga, foto] {
                        s.set(String::new());
                    }
                    open.set(false);
                    on_saved.run(());
                }
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        {move || {
            if !open.get() {
                return view! {
                    <button type="button" class="action-bar" on:click=move |_| open.set(true)>
                        <span class="action-icon">
                            <Icon name="add_circle" />
                        </span>
                        <span class="action-text">
                            <strong>"Tambah Bus Sewa"</strong>
                            <small>"Tampil di katalog halaman Charter"</small>
                        </span>
                        <Icon name="chevron_right" />
                    </button>
                }
                    .into_any();
            }
            let submit = submit.clone();
            view! {
                <section class="card form-card">
                    <div class="step-head">
                        <span class="step-icon">
                            <Icon name="airport_shuttle" />
                        </span>
                        <div>
                            <h3>"Bus Sewa Baru"</h3>
                            <p>"Spesifikasi armada & tarif sewa harian"</p>
                        </div>
                        <button type="button" class="icon-btn" title="Tutup" on:click=move |_| open.set(false)>
                            <Icon name="close" />
                        </button>
                    </div>
                    <ImageUploadField url=foto />
                    <TextField label="Nama Bus / PO" icon="directions_bus" placeholder="PO Sinar Jaya Pariwisata" value=nama />
                    <div class="field-grid">
                        <TextField label="Tipe Sasis / Bus" icon="build" placeholder="Mercedes OH 1626" value=tipe_bus />
                        <TextField label="Kelas" icon="workspace_premium" placeholder="SHD / Medium Executive" value=kelas />
                    </div>
                    <div class="field-grid">
                        <TextField label="Kapasitas (seat)" icon="event_seat" placeholder="50" value=kapasitas number=true />
                        <TextField label="Konfigurasi" icon="grid_view" placeholder="2-2" value=konfigurasi />
                    </div>
                    <label class="field">
                        <span class="field-label">"Deskripsi"</span>
                        <textarea
                            placeholder="Cocok untuk study tour, gathering kantor, …"
                            prop:value=move || deskripsi.get()
                            on:input=move |ev| deskripsi.set(event_target_value(&ev))
                        ></textarea>
                    </label>
                    <label class="field">
                        <span class="field-label">"Fasilitas (pisahkan koma)"</span>
                        <textarea
                            placeholder="Full AC, Audio Karaoke, Port USB, Toilet"
                            prop:value=move || fasilitas.get()
                            on:input=move |ev| fasilitas.set(event_target_value(&ev))
                        ></textarea>
                    </label>
                    <div class="field-grid">
                        <TextField label="Tarif Harian (Rp)" icon="payments" placeholder="3200000" value=harga number=true />
                        <TextField label="Termasuk" icon="receipt" placeholder="Bus + Driver + BBM" value=catatan />
                    </div>
                    <button type="button" class="btn btn-cta btn-block" on:click=submit disabled=move || busy.get()>
                        <Icon name="publish" />
                        {move || if busy.get() { "Menyimpan…" } else { "Terbitkan ke Katalog" }}
                    </button>
                    {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
                </section>
            }
                .into_any()
        }}
    }
}
