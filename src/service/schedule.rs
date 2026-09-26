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
