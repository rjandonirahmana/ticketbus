//! service/rental.rs — paket wisata, katalog bus charter, dan permintaan
//! sewa. Aturan kepemilikan sama dengan armada: admin boleh semua, merchant
//! hanya item miliknya; item buatan admin = milik platform (merchant_id NULL).

use std::sync::Arc;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::repository::rental::{ListingTable, NewRequestRow, RentalRepository};
use crate::service::auth::Claims;
use crate::service::rate_limit::RateLimiter;
use crate::service::waha::WahaClient;
use crate::service::wa_message;
use crate::utils::phone;
use crate::web::models::{CharterBus, NewCharterBus, NewRentalRequest, NewTourPackage, RentalRequest, TourPackage};

const KATEGORI: &[&str] = &["alam", "budaya", "religi", "edukasi", "lainnya"];
const STATUS: &[&str] = &["baru", "dihubungi", "deal", "batal"];
const REQUEST_MAX_PER_HOUR: usize = 5;

#[derive(Clone)]
pub struct RentalService {
    repo: RentalRepository,
    waha: WahaClient,
    rate: Arc<RateLimiter>,
    /// Nomor CS (ADMIN_PHONE ternormalisasi, mis. "6281234567890").
    cs_phone: String,
}

/// "AC, Toilet\nKaraoke" → ["AC", "Toilet", "Karaoke"] (maks 12 item).
pub fn parse_fasilitas(raw: &str) -> Vec<String> {
    raw.split([',', '\n'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(12)
        .collect()
}

fn parse_date(s: &str) -> anyhow::Result<Option<NaiveDate>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| anyhow::anyhow!("Format tanggal tidak valid"))
}

fn owner_for(actor: &Claims) -> anyhow::Result<Option<&str>> {
    match actor.role.as_str() {
        "admin" => Ok(None),
        "merchant" => Ok(Some(actor.user_id.as_str())),
        _ => anyhow::bail!("Peran tidak diizinkan"),
    }
}

fn check_url(url: &str) -> anyhow::Result<()> {
    if url.is_empty() || url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        anyhow::bail!("URL foto tidak valid")
    }
}

impl RentalService {
    pub fn new(repo: RentalRepository, waha: WahaClient, rate: Arc<RateLimiter>, cs_phone: String) -> Self {
        Self { repo, waha, rate, cs_phone }
    }

    pub fn cs_phone(&self) -> &str {
        &self.cs_phone
    }

    // ── Publik ──────────────────────────────────────────────────────────────

    pub async fn public_packages(&self) -> anyhow::Result<Vec<TourPackage>> {
        self.repo.list_packages(true, None).await
    }

    pub async fn public_buses(&self) -> anyhow::Result<Vec<CharterBus>> {
        self.repo.list_buses(true, None).await
    }

    // ── Dashboard (admin: semua, merchant: miliknya) ────────────────────────

    pub async fn manage_packages(&self, actor: &Claims) -> anyhow::Result<Vec<TourPackage>> {
        self.repo.list_packages(false, owner_for(actor)?).await
    }

    pub async fn manage_buses(&self, actor: &Claims) -> anyhow::Result<Vec<CharterBus>> {
        self.repo.list_buses(false, owner_for(actor)?).await
    }

    pub async fn create_package(&self, actor: &Claims, mut p: NewTourPackage) -> anyhow::Result<String> {
        p.judul = p.judul.trim().to_string();
        p.kawasan = p.kawasan.trim().to_string();
        p.durasi = p.durasi.trim().to_string();
        if p.judul.is_empty() || p.kawasan.is_empty() || p.durasi.is_empty() {
            anyhow::bail!("Judul, kawasan, dan durasi wajib diisi");
        }
        if !KATEGORI.contains(&p.kategori.as_str()) {
            anyhow::bail!("Kategori tidak valid");
        }
        if p.harga_pax <= 0 && p.harga_charter <= 0 {
            anyhow::bail!("Isi harga per orang atau harga charter");
        }
        if p.harga_pax < 0 || p.harga_charter < 0 {
            anyhow::bail!("Harga tidak boleh negatif");
        }
        p.min_pax = p.min_pax.max(1);
        check_url(&p.foto_url)?;
        let fasilitas = parse_fasilitas(&p.fasilitas);
        self.repo.create_package(&p, &fasilitas, owner_for(actor)?).await
    }

    pub async fn create_bus(&self, actor: &Claims, mut b: NewCharterBus) -> anyhow::Result<String> {
        b.nama = b.nama.trim().to_string();
        if b.nama.is_empty() {
            anyhow::bail!("Nama bus/PO wajib diisi");
        }
        if !(1..=100).contains(&b.kapasitas) {
            anyhow::bail!("Kapasitas harus 1–100 kursi");
        }
        if b.harga_harian <= 0 {
            anyhow::bail!("Harga sewa harian wajib diisi");
        }
        check_url(&b.foto_url)?;
        let fasilitas = parse_fasilitas(&b.fasilitas);
        self.repo.create_bus(&b, &fasilitas, owner_for(actor)?).await
    }

    async fn check_owner(&self, actor: &Claims, table: ListingTable, id: &str) -> anyhow::Result<()> {
        if actor.role == "admin" {
            return Ok(());
        }
        let owner = self.repo.owner_of(table, id).await?;
        if actor.role == "merchant" && owner.as_deref() == Some(actor.user_id.as_str()) {
            return Ok(());
        }
        anyhow::bail!("Anda tidak berhak mengubah data ini")
    }

    pub async fn set_active(&self, actor: &Claims, table: ListingTable, id: &str, aktif: bool) -> anyhow::Result<()> {
        self.check_owner(actor, table, id).await?;
        self.repo.set_active(table, id, aktif).await
    }

    pub async fn delete(&self, actor: &Claims, table: ListingTable, id: &str) -> anyhow::Result<()> {
        self.check_owner(actor, table, id).await?;
        self.repo.delete(table, id).await
    }

    // ── Permintaan sewa ─────────────────────────────────────────────────────

    /// Publik (boleh tanpa login). Pemilik item & nama item disalin dari DB —
    /// tidak pernah dipercaya dari input klien.
    pub async fn create_request(&self, user_id: Option<&str>, r: NewRentalRequest) -> anyhow::Result<RentalRequest> {
        let nama = r.nama.trim();
        if nama.is_empty() {
            anyhow::bail!("Nama pemesan wajib diisi");
        }
        let telp = phone::normalize(&r.telp).ok_or_else(|| anyhow::anyhow!("Nomor WhatsApp tidak valid"))?;
        if !(1..=1000).contains(&r.jumlah_orang) {
            anyhow::bail!("Jumlah rombongan harus 1–1000 orang");
        }
        let tgl_berangkat = parse_date(&r.tgl_berangkat)?;
        let tgl_pulang = parse_date(&r.tgl_pulang)?;
        if let (Some(a), Some(b)) = (tgl_berangkat, tgl_pulang) {
            if b < a {
                anyhow::bail!("Tanggal pulang tidak boleh sebelum tanggal berangkat");
            }
        }
        if !self.rate.allow(&format!("rental:{telp}"), REQUEST_MAX_PER_HOUR, 3600) {
            anyhow::bail!("Terlalu banyak permintaan dari nomor ini, coba lagi nanti");
        }

        let (item_id, item_nama, merchant_id) = match r.jenis.as_str() {
            "paket" => {
                let p = self
                    .repo
                    .get_package(&r.item_id)
                    .await?
                    .filter(|p| p.aktif)
                    .ok_or_else(|| anyhow::anyhow!("Paket wisata tidak tersedia"))?;
                (Some(p.id), p.judul, p.merchant_id)
            }
            "charter" => {
                let b = self
                    .repo
                    .get_bus(&r.item_id)
                    .await?
                    .filter(|b| b.aktif)
                    .ok_or_else(|| anyhow::anyhow!("Bus sewa tidak tersedia"))?;
                (Some(b.id), format!("{} ({} seat)", b.nama, b.kapasitas), b.merchant_id)
            }
            "custom" => (None, "Rencana wisata custom".to_string(), None),
            _ => anyhow::bail!("Jenis permintaan tidak valid"),
        };
        let to_uuid = |s: Option<&str>| s.map(Uuid::parse_str).transpose();

        let created = self
            .repo
            .create_request(&NewRequestRow {
                jenis: &r.jenis,
                item_id: to_uuid(item_id.as_deref())?,
                item_nama: &item_nama,
                merchant_id: to_uuid(merchant_id.as_deref())?,
                user_id: to_uuid(user_id)?,
                nama,
                telp: &telp,
                jemput: r.jemput.trim(),
                tujuan: r.tujuan.trim(),
                tgl_berangkat,
                tgl_pulang,
                tipe_perjalanan: r.tipe_perjalanan.trim(),
                jumlah_orang: r.jumlah_orang,
                catatan: r.catatan.trim(),
            })
            .await?;

        // Notifikasi WA best-effort: permintaan sudah tersimpan, jadi WAHA
        // yang mati tidak boleh menggagalkannya.
        let waha = self.waha.clone();
        let cs = self.cs_phone.clone();
        let req = created.clone();
        tokio::spawn(async move {
            let to_customer = wa_message::rental_received(&req);
            if let Err(e) = waha.send_text(&phone::to_waha_chat_id(&req.telp), &to_customer).await {
                tracing::warn!(error = %e, "WA konfirmasi permintaan sewa gagal");
            }
            if !cs.is_empty() {
                let to_cs = wa_message::rental_to_cs(&req);
                if let Err(e) = waha.send_text(&phone::to_waha_chat_id(&cs), &to_cs).await {
                    tracing::warn!(error = %e, "WA notifikasi CS permintaan sewa gagal");
                }
            }
        });

        Ok(created)
    }

    pub async fn list_requests(&self, actor: &Claims) -> anyhow::Result<Vec<RentalRequest>> {
        self.repo.list_requests(owner_for(actor)?).await
    }

    pub async fn set_request_status(&self, actor: &Claims, id: &str, status: &str) -> anyhow::Result<()> {
        if !STATUS.contains(&status) {
            anyhow::bail!("Status tidak valid");
        }
        if actor.role != "admin" {
            let owner = self.repo.request_owner(id).await?;
            if !(actor.role == "merchant" && owner.as_deref() == Some(actor.user_id.as_str())) {
                anyhow::bail!("Anda tidak berhak mengubah permintaan ini");
            }
        }
        self.repo.set_request_status(id, status).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fasilitas_dipisah_koma_dan_baris() {
        assert_eq!(parse_fasilitas("AC, Toilet\n\nKaraoke ,"), vec!["AC", "Toilet", "Karaoke"]);
        assert!(parse_fasilitas("  ").is_empty());
    }

    #[test]
    fn tanggal_opsional() {
        assert_eq!(parse_date("").unwrap(), None);
        assert!(parse_date("2026-10-18").unwrap().is_some());
        assert!(parse_date("18/10/2026").is_err());
    }
}
