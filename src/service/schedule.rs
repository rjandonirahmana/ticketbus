use crate::repository::{ArmadaRepository, ScheduleRepository};
use crate::service::auth::Claims;
use crate::web::models::{NewSchedule, Schedule};

#[derive(Clone)]
pub struct ScheduleService {
    repo: ScheduleRepository,
    armadas: ArmadaRepository,
}

impl ScheduleService {
    pub fn new(repo: ScheduleRepository, armadas: ArmadaRepository) -> Self {
        Self { repo, armadas }
    }

    pub async fn list_by_month(&self, year: i32, month: u32) -> anyhow::Result<Vec<Schedule>> {
        self.repo.list_by_month(year, month).await
    }

    /// Jadwal akan datang lintas armada — halaman browse publik.
    pub async fn list_upcoming(&self) -> anyhow::Result<Vec<Schedule>> {
        self.repo.list_upcoming().await
    }

    pub async fn list_by_armada(&self, armada_id: Option<&str>) -> anyhow::Result<Vec<Schedule>> {
        self.repo.list_by_armada(armada_id).await
    }

    pub async fn list_by_merchant(&self, merchant_id: &str) -> anyhow::Result<Vec<Schedule>> {
        self.repo.list_by_merchant(merchant_id).await
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Schedule>> {
        self.repo.get(id).await
    }

    /// Tautan driver: hanya pemilik jadwal (merchant) atau admin.
    pub async fn driver_token(&self, actor: &Claims, id: &str) -> anyhow::Result<String> {
        if actor.role == "merchant" {
            let armada_id = self.repo.armada_id_of(id).await?;
            let owner = self.armadas.merchant_id_of(&armada_id).await?;
            if owner.as_deref() != Some(actor.user_id.as_str()) {
                anyhow::bail!("Anda tidak berhak atas jadwal ini");
            }
        } else if actor.role != "admin" {
            anyhow::bail!("Akses ditolak");
        }
        self.repo
            .tracking_token(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Jadwal tidak ditemukan"))
    }

    /// Radar/peta: jadwal HARI INI (WIB) yang punya lokasi.
    pub async fn nearby_today(&self) -> anyhow::Result<Vec<crate::web::models::NearbyBus>> {
        self.repo.with_location(crate::web::components::today_wib()).await
    }

    pub async fn driver_trip(&self, token: &str) -> anyhow::Result<Option<crate::web::models::DriverTrip>> {
        let Some(schedule) = self.repo.by_token(token.trim()).await? else {
            return Ok(None);
        };
        let posisi = self.repo.position(&schedule.id).await?;
        Ok(Some(crate::web::models::DriverTrip { schedule, posisi }))
    }

    /// Posisi dari HP driver. Hanya diterima dari H-1 s/d H+1 tanggal
    /// jadwal — tautan lama yang bocor tak bisa dipakai menulis selamanya.
    pub async fn report_position(
        &self,
        token: &str,
        lat: f64,
        lng: f64,
        speed_kmh: Option<f64>,
        heading: Option<f64>,
        accuracy_m: Option<f64>,
    ) -> anyhow::Result<()> {
        if !crate::web::geo::koordinat_valid(lat, lng) {
            anyhow::bail!("Koordinat tidak valid");
        }
        let s = self
            .repo
            .by_token(token.trim())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Tautan driver tidak dikenal"))?;
        let tgl = chrono::NaiveDate::parse_from_str(&s.tanggal, "%Y-%m-%d")?;
        let hari = (crate::web::components::today_wib() - tgl).num_days();
        if !(-1..=1).contains(&hari) {
            anyhow::bail!("Pelacakan hanya aktif sehari sebelum s/d sehari sesudah tanggal jadwal ({})", s.tanggal);
        }
        let clean = |v: Option<f64>| v.filter(|x| x.is_finite() && *x >= 0.0);
        self.repo
            .upsert_position(&s.id, lat, lng, clean(speed_kmh), clean(heading), clean(accuracy_m))
            .await
    }

    pub async fn stop_tracking(&self, token: &str) -> anyhow::Result<()> {
        let s = self
            .repo
            .by_token(token.trim())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Tautan driver tidak dikenal"))?;
        self.repo.delete_position(&s.id).await
    }

    pub async fn seat_map(&self, id: &str) -> anyhow::Result<Option<crate::web::models::SeatMap>> {
        let Some(schedule) = self.repo.get(id).await? else {
            return Ok(None);
        };
        let terisi = self.repo.seats_taken(id).await?;
        Ok(Some(crate::web::models::SeatMap { schedule, terisi }))
    }

    pub async fn create(&self, actor: &Claims, input: NewSchedule) -> anyhow::Result<Schedule> {
        if input.tujuan.trim().is_empty() {
            anyhow::bail!("Tujuan/penyewa tidak boleh kosong");
        }
        if input.harga < 0 {
            anyhow::bail!("Harga tidak boleh negatif");
        }
        if input.kapasitas <= 0 || input.kapasitas > 200 {
            anyhow::bail!("Kapasitas harus 1–200 kursi");
        }
        if !crate::web::seats::KONFIGURASI.iter().any(|(k, _)| *k == input.konfigurasi) {
            anyhow::bail!("Konfigurasi kursi tidak valid");
        }
        // Kursi wanita harus benar-benar ada di denah jadwal ini.
        let denah = crate::web::seats::layout(input.kapasitas, &input.konfigurasi, input.dua_dek);
        for kode in crate::web::seats::parse_kode_list(&input.kursi_wanita) {
            if !denah.iter().any(|k| k.kode == kode) {
                anyhow::bail!("Kursi wanita {kode} tidak ada di denah ({} kursi, {})", input.kapasitas, input.konfigurasi);
            }
        }
        if actor.role == "merchant" {
            let owner = self.armadas.merchant_id_of(&input.armada_id).await?;
            if owner.as_deref() != Some(actor.user_id.as_str()) {
                anyhow::bail!("Anda hanya boleh membuat jadwal untuk armada milik sendiri");
            }
        }
        self.repo.create(&input).await
    }

    pub async fn delete(&self, actor: &Claims, id: &str) -> anyhow::Result<()> {
        if actor.role == "merchant" {
            let armada_id = self.repo.armada_id_of(id).await?;
            let owner = self.armadas.merchant_id_of(&armada_id).await?;
            if owner.as_deref() != Some(actor.user_id.as_str()) {
                anyhow::bail!("Anda tidak berhak menghapus jadwal ini");
            }
        }
        self.repo.delete(id).await
    }
}
