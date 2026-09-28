//! "Trayek Tetap" di dashboard mitra PO & admin: rute bolak-balik dengan jam
//! sama setiap hari. Jadwal harian dibuat otomatis oleh server; di sini
//! pengelola hanya mengganti bus & driver per hari (atau membatalkan satu
//! keberangkatan). Scope data ditegakkan server fn.

use leptos::prelude::*;

use super::{clean_error, format_rupiah, format_tanggal, spawn_client, Icon, PickupPicker, RouteTimeline, TextField};
use crate::web::api::{
    assign_schedule, delete_route, list_route_schedules, list_routes, save_route, set_route_active, set_schedule_batal,
};
use crate::web::jam;
use crate::web::models::{Armada, NewRoute, Route, Schedule};

/// Bus yang boleh dipakai trayek: milik PO yang sama dengan pemilik trayek.
fn bus_untuk(armadas: &[Armada], owner: &Option<String>) -> Vec<Armada> {
    armadas.iter().filter(|a| &a.merchant_id == owner).cloned().collect()
}

#[component]
pub fn RouteManager(armadas: Vec<Armada>) -> impl IntoView {
    let routes = Resource::new(|| (), |_| list_routes());
    // None = form tertutup, Some(None) = trayek baru, Some(Some(r)) = ubah r.
    let editing = RwSignal::new(None::<Option<Route>>);
    let expanded = RwSignal::new(None::<String>);
    let error = RwSignal::new(String::new());
    let armadas = StoredValue::new(armadas);

    let act = move |fut: std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ServerFnError>>>>| {
        spawn_client(async move {
            match fut.await {
                Ok(()) => error.set(String::new()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            routes.refetch();
        });
    };

    view! {
        <section class="card route-intro">
            <div class="step-head">
                <span class="step-icon">
                    <Icon name="swap_horiz" />
                </span>
                <div>
                    <h3>"Trayek Tetap"</h3>
                    <p>
                        "Atur sekali: kota asal–tujuan, jam berangkat & tiba. Jadwal tiap hari dibuat otomatis "
                        {format!("{} hari ke depan", crate::web::jam::HORIZON_HARI)}
                        " — per hari cukup ganti bus & driver bila perlu."
                    </p>
                </div>
            </div>
            <button type="button" class="btn btn-primary btn-block" on:click=move |_| editing.set(Some(None))>
                <Icon name="add_road" />
                "Tambah Trayek (Pergi–Pulang)"
            </button>
        </section>

        {move || {
            editing
                .get()
                .map(|init| {
                    view! {
                        <RouteForm
                            initial=init
                            armadas=armadas.get_value()
                            on_close=move |_| editing.set(None)
                            on_saved=move |_| {
                                editing.set(None);
                                routes.refetch();
                            }
                        />
                    }
                })
        }}
        {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}

        <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
            {move || {
                let list = routes.get().and_then(|r| r.ok()).unwrap_or_default();
                if list.is_empty() {
                    return view! {
                        <div class="card empty-state">
                            <Icon name="route" />
                            <p>"Belum ada trayek tetap. Tambahkan trayek, mis. Solo → Jakarta 17:00 tiba 05:00, sekalian rute baliknya."</p>
                        </div>
                    }
                        .into_any();
                }
                list.into_iter()
                    .map(|r| {
                        let id = r.id.clone();
                        let (id_a, id_d, id_x) = (id.clone(), id.clone(), id.clone());
                        let aktif = r.aktif;
                        let edit_r = r.clone();
                        let owner = r.merchant_id.clone();
                        let dur = jam::durasi(&r.jam_berangkat, &r.jam_tiba);
                        view! {
                            <article class=if aktif { "card route-card" } else { "card route-card inactive" }>
                                <header class="route-head">
                                    <div>
                                        <h3>
                                            {format!("{} → {}", r.asal, r.tujuan)}
                                            {r.pasangan_id.is_some().then(|| view! {
                                                <span class="pill pill-primary" title="Punya rute balik">
                                                    <Icon name="sync_alt" />
                                                    "Pergi–Pulang"
                                                </span>
                                            })}
                                        </h3>
                                        <p>
                                            {format!("{} – {}", r.jam_berangkat, r.jam_tiba)}
                                            {dur.map(|(m, besok)| format!(" · {}{}", jam::durasi_label(m), if besok { " (tiba esok hari)" } else { "" }))}
                                        </p>
                                    </div>
                                    <span class=if aktif { "pill pill-ok" } else { "pill" }>{if aktif { "Beroperasi" } else { "Dijeda" }}</span>
                                </header>
                                <RouteTimeline
                                    jam=r.jam_berangkat.clone()
                                    tiba=r.jam_tiba.clone()
                                    asal=r.asal.clone()
                                    from=r.lokasi_jemput.clone()
                                    to=r.tujuan.clone()
                                    mid=format!("Rp {}", format_rupiah(r.harga))
                                />
                                <div class="route-meta">
                                    <span>
                                        <Icon name="event_repeat" />
                                        {jam::hari_label(r.hari_operasi)}
                                    </span>
                                    <span class=if r.armada_id.is_none() { "warn" } else { "" }>
                                        <Icon name="directions_bus" />
                                        {if r.armada_id.is_none() {
                                            "Bus default dihapus — pilih bus baru".to_string()
                                        } else {
                                            format!("Bus default: {}", r.armada_name)
                                        }}
                                    </span>
                                    <span>
                                        <Icon name="badge" />
                                        {if r.driver_nama.is_empty() { "Driver default belum diisi".to_string() } else { format!("Driver default: {}", r.driver_nama) }}
                                    </span>
                                    <span>
                                        <Icon name="event_seat" />
                                        {format!("{} kursi · {}{}", r.kapasitas, r.konfigurasi, if r.dua_dek { " · 2 dek" } else { "" })}
                                    </span>
                                </div>
                                <div class="route-actions">
                                    <button
                                        type="button"
                                        class=move || if expanded.get().as_deref() == Some(id.as_str()) { "btn btn-primary btn-sm" } else { "btn btn-soft btn-sm" }
                                        on:click=move |_| {
                                            expanded.update(|e| *e = if e.as_deref() == Some(id_x.as_str()) { None } else { Some(id_x.clone()) })
                                        }
                                    >
                                        <Icon name="calendar_month" />
                                        "Bus & Driver Harian"
                                    </button>
                                    <button type="button" class="btn btn-soft btn-sm" on:click=move |_| editing.set(Some(Some(edit_r.clone())))>
                                        <Icon name="edit" />
                                        "Ubah"
                                    </button>
                                    <button
                                        type="button"
                                        class=if aktif { "status-toggle on" } else { "status-toggle" }
                                        title=if aktif { "Jeda: jadwal mendatang tanpa penumpang dihapus" } else { "Aktifkan lagi" }
                                        on:click=move |_| act(Box::pin(set_route_active(id_a.clone(), !aktif)))
                                    >
                                        {if aktif { "Jeda" } else { "Aktifkan" }}
                                    </button>
                                    <button
                                        type="button"
                                        class="icon-btn icon-btn-danger"
                                        title="Hapus trayek (keberangkatan yang sudah ada penumpang tetap jalan)"
                                        on:click=move |_| act(Box::pin(delete_route(id_d.clone())))
                                    >
                                        <Icon name="delete" />
                                    </button>
                                </div>
                                {
                                    let rid = r.id.clone();
                                    move || {
                                        (expanded.get().as_deref() == Some(rid.as_str()))
                                            .then(|| {
                                                let bus = bus_untuk(&armadas.get_value(), &owner);
                                                view! { <DailyAssign route_id=rid.clone() armadas=bus /> }
                                            })
                                    }
                                }
                            </article>
                        }
                    })
                    .collect_view()
                    .into_any()
            }}
        </Suspense>
    }
}

/// Penugasan 14 hari ke depan: bus + driver per keberangkatan.
#[component]
fn DailyAssign(route_id: String, armadas: Vec<Armada>) -> impl IntoView {
    let rid = route_id.clone();
    let list = Resource::new(move || rid.clone(), list_route_schedules);
    let armadas = StoredValue::new(armadas);
    view! {
        <div class="assign-panel">
            <p class="field-hint">
                "Ganti bus/driver hanya untuk tanggal itu. Jadwal yang sudah ada penumpang tidak bisa dibatalkan dari sini."
            </p>
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    let rows = list.get().and_then(|r| r.ok()).unwrap_or_default();
                    if rows.is_empty() {
                        view! { <p class="note-card">"Belum ada keberangkatan — trayek dijeda, bus default kosong, atau hari ini tak beroperasi."</p> }
                            .into_any()
                    } else {
                        rows.into_iter()
                            .map(|s| view! { <AssignRow schedule=s armadas=armadas.get_value() on_changed=move |_| list.refetch() /> })
                            .collect_view()
                            .into_any()
                    }
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn AssignRow(schedule: Schedule, armadas: Vec<Armada>, #[prop(into)] on_changed: Callback<()>) -> impl IntoView {
    let s = schedule;
    let bus = RwSignal::new(s.armada_id.clone());
    let nama = RwSignal::new(s.driver_nama.clone());
    let telp = RwSignal::new(s.driver_telp.clone());
    let orig = (s.armada_id.clone(), s.driver_nama.clone(), s.driver_telp.clone());
    let dirty = move || (bus.get(), nama.get(), telp.get()) != orig;
    let busy = RwSignal::new(false);
    let msg = RwSignal::new((String::new(), false));
    let id = StoredValue::new(s.id.clone());
    let batal = s.batal;

    let simpan = move |_| {
        busy.set(true);
        let (b, n, t) = (bus.get_untracked(), nama.get_untracked(), telp.get_untracked());
        spawn_client(async move {
            match assign_schedule(id.get_value(), b, n, t).await {
                Ok(()) => {
                    msg.set(("Tersimpan".into(), true));
                    on_changed.run(());
                }
                Err(e) => msg.set((clean_error(&e.to_string()), false)),
            }
            busy.set(false);
        });
    };
    let toggle_batal = move |_| {
        busy.set(true);
        spawn_client(async move {
            match set_schedule_batal(id.get_value(), !batal).await {
                Ok(()) => on_changed.run(()),
                Err(e) => msg.set((clean_error(&e.to_string()), false)),
            }
            busy.set(false);
        });
    };

    view! {
        <div class=if batal { "assign-row batal" } else { "assign-row" }>
            <div class="assign-date">
                <strong>{format_tanggal(&s.tanggal)}</strong>
                {if batal {
                    view! { <span class="pill pill-danger">"Dibatalkan"</span> }.into_any()
                } else {
                    view! { <span class="pill">{format!("{}/{} kursi terjual", s.kursi_terjual, s.kapasitas)}</span> }.into_any()
                }}
            </div>
            <div class="assign-fields">
                <span class="input-wrap">
                    <Icon name="directions_bus" />
                    <select prop:value=move || bus.get() on:change=move |ev| bus.set(event_target_value(&ev)) disabled=batal>
                        {armadas
                            .into_iter()
                            .map(|a| view! { <option value=a.id.clone()>{a.name.clone()}</option> })
                            .collect_view()}
                    </select>
                </span>
                <span class="input-wrap">
                    <Icon name="person" />
                    <input
                        type="text"
                        placeholder="Nama driver"
                        disabled=batal
                        prop:value=move || nama.get()
                        on:input=move |ev| nama.set(event_target_value(&ev))
                    />
                </span>
                <span class="input-wrap">
                    <Icon name="call" />
                    <input
                        type="tel"
                        placeholder="No. HP driver"
                        disabled=batal
                        prop:value=move || telp.get()
                        on:input=move |ev| telp.set(event_target_value(&ev))
                    />
                </span>
            </div>
            <div class="assign-actions">
                {(!batal)
                    .then(|| {
                        view! {
                            <button
                                type="button"
                                class="btn btn-cta btn-sm"
                                disabled={move || busy.get() || !dirty()}
                                on:click=simpan
                            >
                                <Icon name="save" />
                                "Simpan"
                            </button>
                        }
                    })}
                <button type="button" class="btn btn-ghost btn-sm" disabled=move || busy.get() on:click=toggle_batal>
                    <Icon name=if batal { "event_available" } else { "event_busy" } />
                    {if batal { "Jalankan lagi" } else { "Batalkan hari ini" }}
                </button>
                {move || {
                    let (m, ok) = msg.get();
                    (!m.is_empty()).then(|| view! { <small class=if ok { "assign-ok" } else { "assign-err" }>{m}</small> })
                }}
            </div>
        </div>
    }
}

/// Input jam "HH:MM".
#[component]
fn TimeField(#[prop(into)] label: String, #[prop(into)] icon: String, value: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="field">
            <span class="field-label">{label}</span>
            <span class="input-wrap">
                <Icon name=icon />
                <input type="time" prop:value=move || value.get() on:input=move |ev| value.set(event_target_value(&ev)) />
            </span>
        </label>
    }
}

/// Pilihan bus (value = id armada; "" = sama dengan rute pergi bila `kosong`).
#[component]
fn BusSelect(
    #[prop(into)] label: String,
    armadas: Vec<Armada>,
    value: RwSignal<String>,
    #[prop(optional, into)] kosong: Option<String>,
) -> impl IntoView {
    view! {
        <label class="field">
            <span class="field-label">{label}</span>
            <span class="input-wrap">
                <Icon name="directions_bus" />
                <select prop:value=move || value.get() on:change=move |ev| value.set(event_target_value(&ev))>
                    <option value="">{kosong.unwrap_or_else(|| "— Pilih bus —".into())}</option>
                    {armadas
                        .into_iter()
                        .map(|a| view! { <option value=a.id.clone()>{a.name.clone()}</option> })
                        .collect_view()}
                </select>
            </span>
        </label>
    }
}

/// Teks "17:00 → 05:00 (+1) · 12j" untuk pratinjau di form.
fn ringkas(berangkat: &str, tiba: &str) -> String {
    match jam::durasi(berangkat, tiba) {
        Some((m, besok)) => format!(
            "Perjalanan {}{}",
            jam::durasi_label(m),
            if besok { " — tiba esok hari" } else { "" }
        ),
        None => "Isi jam berangkat & tiba".into(),
    }
}

#[component]
fn RouteForm(
    initial: Option<Route>,
    armadas: Vec<Armada>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_saved: Callback<()>,
) -> impl IntoView {
    let is_edit = initial.is_some();
    let id = initial.as_ref().map(|r| r.id.clone()).unwrap_or_default();
    // Saat mengubah, bus dibatasi ke milik pemilik trayek.
    let pilihan_bus = match &initial {
        Some(r) => bus_untuk(&armadas, &r.merchant_id),
        None => armadas.clone(),
    };
    let r = initial.unwrap_or(Route {
        id: String::new(),
        merchant_id: None,
        pasangan_id: None,
        asal: String::new(),
        tujuan: String::new(),
        lokasi_jemput: String::new(),
        jemput_lat: None,
        jemput_lng: None,
        jam_berangkat: String::new(),
        jam_tiba: String::new(),
        harga: 0,
        kapasitas: 40,
        konfigurasi: "2-2".into(),
        dua_dek: false,
        kursi_wanita: vec![],
        catatan: String::new(),
        armada_id: None,
        armada_name: String::new(),
        driver_nama: String::new(),
        driver_telp: String::new(),
        hari_operasi: jam::SEMUA_HARI,
        aktif: true,
    });
    let asal = RwSignal::new(r.asal);
    let tujuan = RwSignal::new(r.tujuan);
    let lokasi = RwSignal::new(r.lokasi_jemput);
    let lat = RwSignal::new(r.jemput_lat.map(|v| v.to_string()).unwrap_or_default());
    let lng = RwSignal::new(r.jemput_lng.map(|v| v.to_string()).unwrap_or_default());
    let berangkat = RwSignal::new(r.jam_berangkat);
    let tiba = RwSignal::new(r.jam_tiba);
    let harga = RwSignal::new(if r.harga > 0 { r.harga.to_string() } else { String::new() });
    let kapasitas = RwSignal::new(r.kapasitas.to_string());
    let konfig = RwSignal::new(r.konfigurasi);
    let dua_dek = RwSignal::new(r.dua_dek);
    let wanita = RwSignal::new(r.kursi_wanita.join(", "));
    let catatan = RwSignal::new(r.catatan);
    let bus = RwSignal::new(r.armada_id.unwrap_or_default());
    let driver_nama = RwSignal::new(r.driver_nama);
    let driver_telp = RwSignal::new(r.driver_telp);
    let hari = RwSignal::new(r.hari_operasi);

    let balik = RwSignal::new(!is_edit);
    let b_lokasi = RwSignal::new(String::new());
    let b_berangkat = RwSignal::new(String::new());
    let b_tiba = RwSignal::new(String::new());
    let b_bus = RwSignal::new(String::new());
    let b_nama = RwSignal::new(String::new());
    let b_telp = RwSignal::new(String::new());

    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let input = NewRoute {
            asal: asal.get_untracked(),
            tujuan: tujuan.get_untracked(),
            lokasi_jemput: lokasi.get_untracked(),
            jemput_lat: lat.get_untracked(),
            jemput_lng: lng.get_untracked(),
            jam_berangkat: berangkat.get_untracked(),
            jam_tiba: tiba.get_untracked(),
            harga: harga.get_untracked().trim().parse().unwrap_or(-1),
            kapasitas: kapasitas.get_untracked().trim().parse().unwrap_or(0),
            konfigurasi: konfig.get_untracked(),
            dua_dek: dua_dek.get_untracked(),
            kursi_wanita: wanita.get_untracked(),
            catatan: catatan.get_untracked(),
            armada_id: bus.get_untracked(),
            driver_nama: driver_nama.get_untracked(),
            driver_telp: driver_telp.get_untracked(),
            hari_operasi: hari.get_untracked(),
            balik: !is_edit && balik.get_untracked(),
            balik_lokasi_jemput: b_lokasi.get_untracked(),
            balik_jam_berangkat: b_berangkat.get_untracked(),
            balik_jam_tiba: b_tiba.get_untracked(),
            balik_armada_id: b_bus.get_untracked(),
            balik_driver_nama: b_nama.get_untracked(),
            balik_driver_telp: b_telp.get_untracked(),
        };
        if input.harga < 0 {
            error.set("Isi tarif per kursi".into());
            return;
        }
        let id = id.clone();
        busy.set(true);
        spawn_client(async move {
            match save_route(id, input).await {
                Ok(()) => on_saved.run(()),
                Err(e) => error.set(clean_error(&e.to_string())),
            }
            busy.set(false);
        });
    };

    let bus_balik = pilihan_bus.clone();
    view! {
        <section class="card form-card route-form">
            <div class="step-head">
                <span class="step-icon">
                    <Icon name="route" />
                </span>
                <div>
                    <h3>{if is_edit { "Ubah Trayek" } else { "Trayek Baru" }}</h3>
                    <p>
                        {if is_edit {
                            "Perubahan berlaku untuk jadwal mulai hari ini. Hari yang bus/driver-nya sudah diganti manual tidak ditimpa."
                        } else {
                            "Jam & rute ini akan berulang setiap hari operasi."
                        }}
                    </p>
                </div>
                <button type="button" class="icon-btn" title="Tutup" on:click=move |_| on_close.run(())>
                    <Icon name="close" />
                </button>
            </div>

            <h4 class="form-sub">
                <Icon name="east" />
                {move || {
                    let (a, t) = (asal.get(), tujuan.get());
                    if a.is_empty() && t.is_empty() { "Rute Pergi".to_string() } else { format!("Rute Pergi: {a} → {t}") }
                }}
            </h4>
            <div class="field-grid">
                <TextField label="Kota Asal" icon="trip_origin" placeholder="Solo" value=asal />
                <TextField label="Kota Tujuan" icon="location_on" placeholder="Jakarta" value=tujuan />
            </div>
            <TextField label="Titik Jemput" icon="where_to_vote" placeholder="Terminal Tirtonadi" value=lokasi />
            <PickupPicker lat=lat lng=lng map_id="route-pickup-map" />
            <div class="field-grid">
                <TimeField label="Jam Berangkat" icon="schedule" value=berangkat />
                <TimeField label="Jam Tiba" icon="flag" value=tiba />
            </div>
            <p class="field-hint">{move || ringkas(&berangkat.get(), &tiba.get())}</p>
            <div class="field-grid">
                <BusSelect label="Bus Default" armadas=pilihan_bus value=bus />
                <TextField label="Tarif per Kursi (Rp)" icon="payments" placeholder="250000" value=harga number=true />
            </div>
            <div class="field-grid">
                <TextField label="Driver Default" icon="person" placeholder="Pak Danang" value=driver_nama />
                <TextField label="No. HP Driver" icon="call" placeholder="08xx" value=driver_telp />
            </div>

            {(!is_edit)
                .then(|| {
                    view! {
                        <label class="check-row">
                            <input
                                type="checkbox"
                                prop:checked=move || balik.get()
                                on:change=move |ev| balik.set(event_target_checked(&ev))
                            />
                            <span>
                                <strong>"Buat juga rute balik"</strong>
                                <small>{move || format!("{} → {} dengan jam sendiri", tujuan.get(), asal.get())}</small>
                            </span>
                        </label>
                        {move || {
                            balik
                                .get()
                                .then(|| {
                                    view! {
                                        <div class="balik-box">
                                            <h4 class="form-sub">
                                                <Icon name="west" />
                                                {move || format!("Rute Balik: {} → {}", tujuan.get(), asal.get())}
                                            </h4>
                                            <TextField label="Titik Jemput Balik" icon="where_to_vote" placeholder="Terminal Pulo Gebang" value=b_lokasi />
                                            <div class="field-grid">
                                                <TimeField label="Jam Berangkat" icon="schedule" value=b_berangkat />
                                                <TimeField label="Jam Tiba" icon="flag" value=b_tiba />
                                            </div>
                                            <p class="field-hint">{move || ringkas(&b_berangkat.get(), &b_tiba.get())}</p>
                                            <BusSelect label="Bus Default Balik" armadas=bus_balik.clone() value=b_bus kosong="Sama dengan rute pergi" />
                                            <div class="field-grid">
                                                <TextField label="Driver Balik" icon="person" placeholder="Pak Budi" value=b_nama />
                                                <TextField label="No. HP Driver" icon="call" placeholder="08xx" value=b_telp />
                                            </div>
                                        </div>
                                    }
                                })
                        }}
                    }
                })}

            <div class="field">
                <span class="field-label">"Hari Operasi"</span>
                <div class="chip-row">
                    {jam::HARI
                        .into_iter()
                        .enumerate()
                        .map(|(i, h)| {
                            let bit = 1i16 << i;
                            view! {
                                <button
                                    type="button"
                                    class=move || if hari.get() & bit != 0 { "filter-chip active" } else { "filter-chip" }
                                    on:click=move |_| hari.update(|m| {
                                        let next = *m ^ bit;
                                        if next != 0 {
                                            *m = next;
                                        }
                                    })
                                >
                                    {h}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <p class="field-hint">{move || jam::hari_label(hari.get())}</p>
            </div>

            <div class="field-grid">
                <TextField label="Kapasitas Kursi" icon="event_seat" placeholder="40" value=kapasitas number=true />
                <label class="field">
                    <span class="field-label">"Konfigurasi"</span>
                    <span class="input-wrap">
                        <Icon name="grid_view" />
                        <select prop:value=move || konfig.get() on:change=move |ev| konfig.set(event_target_value(&ev))>
                            {crate::web::seats::KONFIGURASI
                                .into_iter()
                                .map(|(v, label)| view! { <option value=v>{label}</option> })
                                .collect_view()}
                        </select>
                    </span>
                </label>
            </div>
            <div class="field-grid">
                <label class="field">
                    <span class="field-label">"Dek"</span>
                    <span class="input-wrap">
                        <Icon name="stacks" />
                        <select
                            prop:value=move || if dua_dek.get() { "2" } else { "1" }
                            on:change=move |ev| dua_dek.set(event_target_value(&ev) == "2")
                        >
                            <option value="1">"1 Dek"</option>
                            <option value="2">"2 Dek (Double Decker)"</option>
                        </select>
                    </span>
                </label>
                <TextField label="Kursi Khusus Wanita" icon="woman" placeholder="02A, 06B" value=wanita />
            </div>
            <label class="field">
                <span class="field-label">"Catatan (fasilitas, titik kumpul, dsb.)"</span>
                <textarea prop:value=move || catatan.get() on:input=move |ev| catatan.set(event_target_value(&ev))></textarea>
            </label>

            <button type="button" class="btn btn-cta btn-block" on:click=submit disabled=move || busy.get()>
                <Icon name="publish" />
                {move || match (busy.get(), is_edit) {
                    (true, _) => "Menyimpan…",
                    (false, true) => "Simpan Perubahan",
                    (false, false) => "Terbitkan Trayek",
                }}
            </button>
            {move || (!error.get().is_empty()).then(|| view! { <p class="alert alert-error">{error.get()}</p> })}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ringkasan_perjalanan() {
        assert_eq!(ringkas("17:00", "05:00"), "Perjalanan 12j — tiba esok hari");
        assert_eq!(ringkas("08:00", "15:30"), "Perjalanan 7j 30m");
        assert_eq!(ringkas("", "05:00"), "Isi jam berangkat & tiba");
    }
}
