//! service/route.rs — trayek tetap (rute bolak-balik berjadwal sama tiap hari).
//!
//! Trayek adalah "cetakan": server membuat baris `schedules` untuk
//! `HORIZON_HARI` ke depan (saat trayek dibuat/diubah, saat start, dan tiap
//! jam lewat `spawn_generator`). Per hari yang diubah hanya bus & driver
//! (`assign`) atau keberangkatan dibatalkan (`set_batal`).
//!
//! Kepemilikan: admin = semua; merchant = trayek miliknya. Pemilik trayek =
//! pemilik bus default-nya, jadi admin bisa membuatkan trayek untuk mitra.

use std::time::Duration;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::repository::route::{RouteRepository, RouteRow};
use crate::repository::schedule::parse_koordinat;
use crate::repository::ArmadaRepository;
use crate::service::auth::Claims;
use crate::web::jam::{self, HORIZON_HARI};
use crate::web::models::{NewRoute, Route, Schedule};

fn today_wib() -> NaiveDate {
    crate::web::components::today_wib()
}

#[derive(Clone)]
pub struct RouteService {
    repo: RouteRepository,
    armadas: ArmadaRepository,
}

/// Satu arah perjalanan dari form (pergi, atau balik dengan jam sendiri).
struct Leg<'a> {
    asal: &'a str,
    tujuan: &'a str,
    lokasi_jemput: &'a str,
    jemput: Option<(f64, f64)>,
    jam_berangkat: &'a str,
    jam_tiba: &'a str,
    armada_id: &'a str,
    driver_nama: &'a str,
    driver_telp: &'a str,
}

/// Validasi bagian bersama (harga, denah, hari) + satu arah → RouteRow.
fn build_row(r: &NewRoute, leg: Leg<'_>, merchant_id: Option<Uuid>) -> anyhow::Result<RouteRow> {
    let asal = leg.asal.trim();
    let tujuan = leg.tujuan.trim();
    if asal.is_empty() || tujuan.is_empty() {
        anyhow::bail!("Kota asal dan tujuan wajib diisi");
    }
    if asal.eq_ignore_ascii_case(tujuan) {
        anyhow::bail!("Kota asal dan tujuan tidak boleh sama");
    }
    let berangkat = jam::normalisasi(leg.jam_berangkat)
        .ok_or_else(|| anyhow::anyhow!("Jam berangkat {asal} → {tujuan} tidak valid (contoh 17:00)"))?;
    let tiba = jam::normalisasi(leg.jam_tiba)
        .ok_or_else(|| anyhow::anyhow!("Jam tiba {asal} → {tujuan} tidak valid (contoh 05:00)"))?;
    if berangkat == tiba {
        anyhow::bail!("Jam tiba tidak boleh sama dengan jam berangkat");
    }
    if r.harga < 0 {
        anyhow::bail!("Harga tidak boleh negatif");
    }
    if !(1..=200).contains(&r.kapasitas) {
        anyhow::bail!("Kapasitas harus 1–200 kursi");
    }
    if !crate::web::seats::KONFIGURASI.iter().any(|(k, _)| *k == r.konfigurasi) {
        anyhow::bail!("Konfigurasi kursi tidak valid");
    }
    let kursi_wanita = crate::web::seats::parse_kode_list(&r.kursi_wanita);
    let denah = crate::web::seats::layout(r.kapasitas, &r.konfigurasi, r.dua_dek);
    if let Some(k) = kursi_wanita.iter().find(|k| !denah.iter().any(|d| &d.kode == *k)) {
        anyhow::bail!("Kursi wanita {k} tidak ada di denah ({} kursi, {})", r.kapasitas, r.konfigurasi);
    }
    let hari_operasi = if r.hari_operasi <= 0 { jam::SEMUA_HARI } else { r.hari_operasi & jam::SEMUA_HARI };
    let armada_id = Uuid::parse_str(leg.armada_id.trim()).map_err(|_| anyhow::anyhow!("Pilih bus untuk {asal} → {tujuan}"))?;
    Ok(RouteRow {
        merchant_id,
        asal: asal.to_string(),
        tujuan: tujuan.to_string(),
        lokasi_jemput: leg.lokasi_jemput.trim().to_string(),
        jemput: leg.jemput,
        jam_berangkat: berangkat,
        jam_tiba: tiba,
        harga: r.harga,
        kapasitas: r.kapasitas,
        konfigurasi: r.konfigurasi.clone(),
        dua_dek: r.dua_dek,
        kursi_wanita,
        catatan: r.catatan.trim().to_string(),
        armada_id,
        driver_nama: leg.driver_nama.trim().to_string(),
        driver_telp: leg.driver_telp.trim().to_string(),
        hari_operasi,
    })
}

fn pergi(r: &NewRoute) -> anyhow::Result<Leg<'_>> {
    Ok(Leg {
        asal: &r.asal,
        tujuan: &r.tujuan,
        lokasi_jemput: &r.lokasi_jemput,
        jemput: parse_koordinat(&r.jemput_lat, &r.jemput_lng)?,
        jam_berangkat: &r.jam_berangkat,
        jam_tiba: &r.jam_tiba,
        armada_id: &r.armada_id,
        driver_nama: &r.driver_nama,
        driver_telp: &r.driver_telp,
    })
}

impl RouteService {
    pub fn new(repo: RouteRepository, armadas: ArmadaRepository) -> Self {
        Self { repo, armadas }
    }

    /// Buat jadwal untuk semua trayek aktif, lalu ulangi tiap jam supaya
    /// jendela `HORIZON_HARI` ikut bergeser setiap pergantian hari.
    pub fn spawn_generator(&self) {
        let svc = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(3600));
            loop {
                tick.tick().await;
                match svc.repo.generate(None, today_wib(), HORIZON_HARI).await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!(jadwal = n, "trayek tetap: jadwal harian baru dibuat"),
                    Err(e) => tracing::warn!(error = %e, "trayek tetap: gagal membuat jadwal harian"),
                }
            }
        });
    }

    /// `Ok(None)` = admin (tanpa filter), `Ok(Some(id))` = merchant itu.
    fn scope(actor: &Claims) -> anyhow::Result<Option<&str>> {
        match actor.role.as_str() {
            "admin" => Ok(None),
            "merchant" => Ok(Some(actor.user_id.as_str())),
            _ => anyhow::bail!("Peran tidak diizinkan"),
        }
    }

    /// Pemilik bus; merchant hanya boleh memakai bus miliknya sendiri.
    async fn bus_owner(&self, actor: &Claims, armada_id: &str) -> anyhow::Result<Option<String>> {
        let owner = self.armadas.merchant_id_of(armada_id.trim()).await?;
        if actor.role == "merchant" && owner.as_deref() != Some(actor.user_id.as_str()) {
            anyhow::bail!("Anda hanya boleh memakai bus milik sendiri");
        }
        Ok(owner)
    }

    async fn owned_route(&self, actor: &Claims, id: &str) -> anyhow::Result<Route> {
        let route = self.repo.get(id).await?.ok_or_else(|| anyhow::anyhow!("Trayek tidak ditemukan"))?;
        if actor.role != "admin" && route.merchant_id.as_deref() != Some(actor.user_id.as_str()) {
            anyhow::bail!("Anda tidak berhak atas trayek ini");
        }
        Ok(route)
    }

    pub async fn list(&self, actor: &Claims) -> anyhow::Result<Vec<Route>> {
        self.repo.list(Self::scope(actor)?).await
    }

    /// Buat trayek (dan rute baliknya bila diminta), lalu buka jadwalnya.
    pub async fn create(&self, actor: &Claims, r: NewRoute) -> anyhow::Result<()> {
        Self::scope(actor)?;
        let owner = self.bus_owner(actor, &r.armada_id).await?;
        let owner_uuid = owner.as_deref().map(Uuid::parse_str).transpose()?;
        let row = build_row(&r, pergi(&r)?, owner_uuid)?;

        let balik_row = if r.balik {
            let armada = if r.balik_armada_id.trim().is_empty() { &r.armada_id } else { &r.balik_armada_id };
            if self.bus_owner(actor, armada).await? != owner {
                anyhow::bail!("Bus rute balik harus milik PO yang sama");
            }
            let leg = Leg {
                asal: &r.tujuan,
                tujuan: &r.asal,
                lokasi_jemput: &r.balik_lokasi_jemput,
                jemput: None,
                jam_berangkat: &r.balik_jam_berangkat,
                jam_tiba: &r.balik_jam_tiba,
                armada_id: armada,
                driver_nama: &r.balik_driver_nama,
                driver_telp: &r.balik_driver_telp,
            };
            Some(build_row(&r, leg, owner_uuid)?)
        } else {
            None
        };

        let id = self.repo.create(&row).await?;
        if let Some(b) = balik_row {
            let balik_id = self.repo.create(&b).await?;
            self.repo.link_pair(id, balik_id).await?;
            self.repo.generate(Some(balik_id), today_wib(), HORIZON_HARI).await?;
        }
        self.repo.generate(Some(id), today_wib(), HORIZON_HARI).await?;
        Ok(())
    }

    /// Ubah satu arah trayek (kolom `balik_*` diabaikan).
    pub async fn update(&self, actor: &Claims, id: &str, r: NewRoute) -> anyhow::Result<()> {
        let old = self.owned_route(actor, id).await?;
        let owner = self.bus_owner(actor, &r.armada_id).await?;
        if owner != old.merchant_id {
            anyhow::bail!("Bus default harus milik PO pemilik trayek");
        }
        let row = build_row(&r, pergi(&r)?, old.merchant_id.as_deref().map(Uuid::parse_str).transpose()?)?;
        let today = today_wib();
        self.repo.update(id, &old, &row, today).await?;
        self.repo.generate(Some(Uuid::parse_str(id)?), today, HORIZON_HARI).await?;
        Ok(())
    }

    /// Jeda = jadwal mendatang tanpa order dihapus & tak dibuat lagi.
    pub async fn set_active(&self, actor: &Claims, id: &str, aktif: bool) -> anyhow::Result<()> {
        self.owned_route(actor, id).await?;
        self.repo.set_active(id, aktif).await?;
        if aktif {
            self.repo.generate(Some(Uuid::parse_str(id)?), today_wib(), HORIZON_HARI).await?;
        } else {
            self.repo.clear_future(id, today_wib()).await?;
        }
        Ok(())
    }

    /// Hapus trayek. Keberangkatan yang sudah punya penumpang tetap ada
    /// (menjadi jadwal sekali jalan), sisanya ikut dihapus.
    pub async fn delete(&self, actor: &Claims, id: &str) -> anyhow::Result<()> {
        self.owned_route(actor, id).await?;
        self.repo.clear_future(id, today_wib()).await?;
        self.repo.delete(id).await
    }

    /// Keberangkatan trayek mulai hari ini (14 hari) untuk penugasan harian.
    pub async fn upcoming(&self, actor: &Claims, id: &str) -> anyhow::Result<Vec<Schedule>> {
        self.owned_route(actor, id).await?;
        self.repo.schedules_of(id, today_wib(), 14).await
    }

    async fn owned_schedule(&self, actor: &Claims, schedule_id: &str) -> anyhow::Result<Option<String>> {
        let owner = self.repo.schedule_owner(schedule_id).await?;
        if actor.role != "admin" && owner.as_deref() != Some(actor.user_id.as_str()) {
            anyhow::bail!("Anda tidak berhak atas jadwal ini");
        }
        Ok(owner)
    }

    /// Ganti bus & driver untuk SATU keberangkatan.
    pub async fn assign(
        &self,
        actor: &Claims,
        schedule_id: &str,
        armada_id: &str,
        driver_nama: &str,
        driver_telp: &str,
    ) -> anyhow::Result<()> {
        let owner = self.owned_schedule(actor, schedule_id).await?;
        if self.bus_owner(actor, armada_id).await? != owner {
            anyhow::bail!("Bus harus milik PO pemilik jadwal ini");
        }
        self.repo
            .assign(schedule_id, armada_id.trim(), driver_nama.trim(), driver_telp.trim())
            .await
    }

    pub async fn set_batal(&self, actor: &Claims, schedule_id: &str, batal: bool) -> anyhow::Result<()> {
        self.owned_schedule(actor, schedule_id).await?;
        self.repo.set_batal(schedule_id, batal).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> NewRoute {
        NewRoute {
            asal: " Solo ".into(),
            tujuan: "Jakarta".into(),
            jam_berangkat: "17.00".into(),
            jam_tiba: "5:00".into(),
            harga: 250_000,
            kapasitas: 40,
            konfigurasi: "2-2".into(),
            armada_id: Uuid::nil().to_string(),
            ..Default::default()
        }
    }

    fn row(r: &NewRoute) -> anyhow::Result<RouteRow> {
        build_row(r, pergi(r)?, None)
    }

    #[test]
    fn trayek_valid_dirapikan() {
        let r = row(&input()).unwrap();
        assert_eq!(r.asal, "Solo");
        assert_eq!((r.jam_berangkat.as_str(), r.jam_tiba.as_str()), ("17:00", "05:00"));
        assert_eq!(r.hari_operasi, jam::SEMUA_HARI);
    }

    #[test]
    fn trayek_tak_valid_ditolak() {
        assert!(row(&NewRoute { tujuan: "solo".into(), ..input() }).is_err());
        assert!(row(&NewRoute { jam_tiba: "".into(), ..input() }).is_err());
        assert!(row(&NewRoute { jam_tiba: "17:00".into(), ..input() }).is_err());
        assert!(row(&NewRoute { jam_berangkat: "25:00".into(), ..input() }).is_err());
        assert!(row(&NewRoute { armada_id: "".into(), ..input() }).is_err());
        assert!(row(&NewRoute { kursi_wanita: "99Z".into(), ..input() }).is_err());
        assert!(row(&NewRoute { kapasitas: 0, ..input() }).is_err());
    }
}
