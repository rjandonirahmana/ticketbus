//! Form terbitkan jadwal — dipakai dashboard merchant (armada sendiri) dan
//! admin (semua armada). Scope armada ditegakkan server fn, bukan form ini.

use leptos::prelude::*;

use super::{clean_error, spawn_client, Icon};
use crate::web::api::create_schedule;
use crate::web::models::{Armada, NewSchedule};

#[component]
fn Field(#[prop(into)] label: String, #[prop(into)] icon: String, children: Children) -> impl IntoView {
    view! {
        <label class="field">
            <span class="field-label">{label}</span>
            <span class="input-wrap">
                <Icon name=icon />
                {children()}
            </span>
        </label>
    }
}

#[component]
fn StepHead(
    #[prop(into)] num: String,
    #[prop(into)] title: String,
    #[prop(into)] sub: String,
    #[prop(into)] icon: String,
) -> impl IntoView {
    view! {
        <div class="step-head">
            <span class="step-icon">
                <Icon name=icon />
            </span>
            <div>
                <h3>{format!("{num}. {title}")}</h3>
                <p>{sub}</p>
            </div>
            <span class="pill pill-primary">{format!("Langkah {num}/4")}</span>
        </div>
    }
}

#[component]
pub fn ScheduleForm(armadas: Vec<Armada>, #[prop(into)] on_saved: Callback<()>) -> impl IntoView {
    let f_armada = RwSignal::new(String::new());
    let f_tanggal = RwSignal::new(String::new());
    let f_tujuan = RwSignal::new(String::new());
    let f_lokasi = RwSignal::new(String::new());
    let f_jam = RwSignal::new(String::new());
    let f_harga = RwSignal::new(String::new());
    let f_kapasitas = RwSignal::new("40".to_string());
    let f_driver_nama = RwSignal::new(String::new());
    let f_driver_telp = RwSignal::new(String::new());
    let f_catatan = RwSignal::new(String::new());
    let f_konfig = RwSignal::new("2-2".to_string());
    let f_dua_dek = RwSignal::new(false);
    let f_wanita = RwSignal::new(String::new());
    let form_error = RwSignal::new(String::new());
    let form_ok = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        let input = NewSchedule {
            armada_id: f_armada.get_untracked(),
            tanggal: f_tanggal.get_untracked(),
            tujuan: f_tujuan.get_untracked(),
            lokasi_jemput: f_lokasi.get_untracked(),
            jam: f_jam.get_untracked(),
            harga: f_harga.get_untracked().trim().parse().unwrap_or(0),
            catatan: f_catatan.get_untracked(),
            kapasitas: f_kapasitas.get_untracked().trim().parse().unwrap_or(40),
            driver_nama: f_driver_nama.get_untracked(),
            driver_telp: f_driver_telp.get_untracked(),
            konfigurasi: f_konfig.get_untracked(),
            dua_dek: f_dua_dek.get_untracked(),
            kursi_wanita: f_wanita.get_untracked(),
        };
        if input.armada_id.is_empty() || input.tanggal.is_empty() {
            form_error.set("Armada dan tanggal wajib diisi".into());
            form_ok.set(String::new());
            return;
        }
        busy.set(true);
        spawn_client(async move {
            match create_schedule(input).await {
                Ok(_) => {
                    form_ok.set("Jadwal berhasil diterbitkan!".into());
                    form_error.set(String::new());
                    f_tujuan.set(String::new());
                    f_lokasi.set(String::new());
                    f_jam.set(String::new());
                    f_harga.set(String::new());
                    f_catatan.set(String::new());
                    f_driver_nama.set(String::new());
                    f_driver_telp.set(String::new());
                    f_wanita.set(String::new());
                    on_saved.run(());
                }
                Err(e) => {
                    form_error.set(clean_error(&e.to_string()));
                    form_ok.set(String::new());
                }
            }
            busy.set(false);
        });
    };

    view! {
        <div class="form-stack">
            <section class="card form-card">
                <StepHead num="1" title="Armada & Tanggal" sub="Pilih bus yang berangkat" icon="directions_bus" />
                <div class="armada-pick">
                    {armadas
                        .iter()
                        .map(|a| {
                            let id = a.id.clone();
                            let id_cmp = a.id.clone();
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        if f_armada.get() == id_cmp { "armada-opt selected" } else { "armada-opt" }
                                    }
                                    on:click=move |_| f_armada.set(id.clone())
                                >
                                    <span class="swatch" style=format!("background:{}", a.color_hex)></span>
                                    <span>{a.name.clone()}</span>
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="field-grid">
                    <Field label="Tanggal Berangkat" icon="calendar_today">
                        <input
                            type="date"
                            prop:value=move || f_tanggal.get()
                            on:input=move |ev| f_tanggal.set(event_target_value(&ev))
                        />
                    </Field>
                    <Field label="Jam Jemput" icon="schedule">
                        <input
                            type="time"
                            prop:value=move || f_jam.get()
                            on:input=move |ev| f_jam.set(event_target_value(&ev))
                        />
                    </Field>
                </div>
            </section>

            <section class="card form-card">
                <StepHead num="2" title="Rute & Tarif" sub="Titik jemput, tujuan, dan harga per kursi" icon="route" />
                <div class="route-box">
                    <Field label="Titik Jemput" icon="trip_origin">
                        <input
                            type="text"
                            placeholder="Mis. Terminal Pulo Gebang"
                            prop:value=move || f_lokasi.get()
                            on:input=move |ev| f_lokasi.set(event_target_value(&ev))
                        />
                    </Field>
                    <Field label="Tujuan" icon="location_on">
                        <input
                            type="text"
                            placeholder="Mis. Yogyakarta"
                            prop:value=move || f_tujuan.get()
                            on:input=move |ev| f_tujuan.set(event_target_value(&ev))
                        />
                    </Field>
                </div>
                <div class="field-grid">
                    <Field label="Tarif per Kursi (Rp)" icon="payments">
                        <input
                            type="number"
                            min="0"
                            placeholder="280000"
                            class="num"
                            prop:value=move || f_harga.get()
                            on:input=move |ev| f_harga.set(event_target_value(&ev))
                        />
                    </Field>
                    <Field label="Kapasitas Kursi" icon="event_seat">
                        <input
                            type="number"
                            min="1"
                            class="num"
                            prop:value=move || f_kapasitas.get()
                            on:input=move |ev| f_kapasitas.set(event_target_value(&ev))
                        />
                    </Field>
                </div>
            </section>

            <section class="card form-card">
                <StepHead num="3" title="Denah Kursi" sub="Tata letak yang dilihat penumpang saat memilih kursi" icon="event_seat" />
                <div class="field-grid">
                    <label class="field">
                        <span class="field-label">"Konfigurasi"</span>
                        <span class="input-wrap">
                            <Icon name="grid_view" />
                            <select prop:value=move || f_konfig.get() on:change=move |ev| f_konfig.set(event_target_value(&ev))>
                                {crate::web::seats::KONFIGURASI
                                    .into_iter()
                                    .map(|(v, label)| view! { <option value=v>{label}</option> })
                                    .collect_view()}
                            </select>
                        </span>
                    </label>
                    <label class="field">
                        <span class="field-label">"Dek"</span>
                        <span class="input-wrap">
                            <Icon name="stacks" />
                            <select
                                prop:value=move || if f_dua_dek.get() { "2" } else { "1" }
                                on:change=move |ev| f_dua_dek.set(event_target_value(&ev) == "2")
                            >
                                <option value="1">"1 Dek"</option>
                                <option value="2">"2 Dek (Double Decker)"</option>
                            </select>
                        </span>
                    </label>
                </div>
                <label class="field">
                    <span class="field-label">"Kursi Khusus Wanita (opsional)"</span>
                    <span class="input-wrap">
                        <Icon name="woman" />
                        <input
                            type="text"
                            placeholder="Mis. 02A, 06B"
                            prop:value=move || f_wanita.get()
                            on:input=move |ev| f_wanita.set(event_target_value(&ev))
                        />
                    </span>
                </label>
                <p class="field-hint">
                    {move || {
                        let n = f_kapasitas.get().trim().parse::<i32>().unwrap_or(0);
                        let denah = crate::web::seats::layout(n, &f_konfig.get(), f_dua_dek.get());
                        match (denah.first(), denah.last()) {
                            (Some(a), Some(b)) => format!("Denah: {} kursi, nomor {} – {}", denah.len(), a.kode, b.kode),
                            _ => "Isi kapasitas kursi untuk melihat penomoran".to_string(),
                        }
                    }}
                </p>
            </section>

            <section class="card form-card">
                <StepHead num="4" title="Driver & Catatan" sub="Kontak kru yang bertugas" icon="badge" />
                <div class="field-grid">
                    <Field label="Nama Driver" icon="person">
                        <input
                            type="text"
                            placeholder="Pak Danang"
                            prop:value=move || f_driver_nama.get()
                            on:input=move |ev| f_driver_nama.set(event_target_value(&ev))
                        />
                    </Field>
                    <Field label="No. HP Driver" icon="call">
                        <input
                            type="tel"
                            placeholder="08xx"
                            prop:value=move || f_driver_telp.get()
                            on:input=move |ev| f_driver_telp.set(event_target_value(&ev))
                        />
                    </Field>
                </div>
                <label class="field">
                    <span class="field-label">"Catatan"</span>
                    <textarea
                        placeholder="Fasilitas, info titik kumpul, dsb."
                        prop:value=move || f_catatan.get()
                        on:input=move |ev| f_catatan.set(event_target_value(&ev))
                    ></textarea>
                </label>
            </section>

            <button type="button" class="btn btn-cta btn-block" on:click=submit disabled=move || busy.get()>
                <Icon name="send" />
                {move || if busy.get() { "Menerbitkan…" } else { "Terbitkan Jadwal" }}
            </button>
            {move || (!form_error.get().is_empty()).then(|| view! { <p class="alert alert-error">{form_error.get()}</p> })}
            {move || (!form_ok.get().is_empty()).then(|| view! { <p class="alert alert-ok">{form_ok.get()}</p> })}
        </div>
    }
}
