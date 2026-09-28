//! repository/rental.rs — paket wisata, katalog bus charter, dan permintaan
//! sewa. Kepemilikan (merchant_id) dicek di service layer, bukan di sini.

use chrono::{DateTime, NaiveDate, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::{CharterBus, NewCharterBus, NewTourPackage, RentalRequest, TourPackage};

const SELECT_PACKAGE: &str = "
    SELECT p.id, p.merchant_id, COALESCE(u.name, 'LajuBus') AS pemilik, p.judul, p.kategori,
           p.kawasan, p.durasi, p.label_badge, p.label_tipe, p.rute, p.armada_info, p.fasilitas,
           p.harga_pax, p.min_pax, p.harga_charter, p.foto_url, p.aktif
      FROM tour_packages p
      LEFT JOIN users u ON u.id = p.merchant_id";

const SELECT_BUS: &str = "
    SELECT b.id, b.merchant_id, COALESCE(u.name, 'LajuBus') AS pemilik, b.nama, b.tipe_bus, b.kelas,
           b.kapasitas, b.konfigurasi, b.deskripsi, b.fasilitas, b.harga_harian, b.catatan_harga,
           b.foto_url, b.aktif
      FROM charter_buses b
      LEFT JOIN users u ON u.id = b.merchant_id";

const SELECT_REQUEST: &str = "
    SELECT id, jenis, item_nama, nama, telp, jemput, tujuan, tgl_berangkat, tgl_pulang,
           tipe_perjalanan, jumlah_orang, catatan, status, created_at
      FROM rental_requests";

/// Katalog publik hanya memuat item platform atau mitra PO yang disetujui.
fn po_ok(alias: &str) -> String {
    format!(
        "({alias}.merchant_id IS NULL OR EXISTS (SELECT 1 FROM merchant_profiles mp
            WHERE mp.user_id = {alias}.merchant_id AND mp.status = 'disetujui'))"
    )
}

/// Tabel yang punya kolom `merchant_id` + `aktif` — dipakai helper generik
/// kepemilikan / aktif / hapus supaya tak ada nama tabel dari input.
#[derive(Clone, Copy)]
pub enum ListingTable {
    Package,
    Bus,
}

impl ListingTable {
    fn name(self) -> &'static str {
        match self {
            ListingTable::Package => "tour_packages",
            ListingTable::Bus => "charter_buses",
        }
    }
}

/// Data yang disalin ke permintaan saat dibuat.
pub struct NewRequestRow<'a> {
    pub jenis: &'a str,
    pub item_id: Option<Uuid>,
    pub item_nama: &'a str,
    pub merchant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub nama: &'a str,
    pub telp: &'a str,
    pub jemput: &'a str,
    pub tujuan: &'a str,
    pub tgl_berangkat: Option<NaiveDate>,
    pub tgl_pulang: Option<NaiveDate>,
    pub tipe_perjalanan: &'a str,
    pub jumlah_orang: i32,
    pub catatan: &'a str,
}

#[derive(Clone)]
pub struct RentalRepository {
    pool: Pool,
}

impl RentalRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // ── Paket wisata ────────────────────────────────────────────────────────

    /// `only_active` untuk halaman publik; `merchant` untuk dashboard mitra.
    pub async fn list_packages(&self, only_active: bool, merchant: Option<&str>) -> anyhow::Result<Vec<TourPackage>> {
        let merchant = merchant.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let po_ok = po_ok("p");
        let sql = format!(
            "{SELECT_PACKAGE} WHERE ($1 = FALSE OR (p.aktif AND {po_ok})) AND ($2::uuid IS NULL OR p.merchant_id = $2)
              ORDER BY p.created_at DESC"
        );
        let rows = conn.query(&sql, &[&only_active, &merchant]).await?;
        Ok(rows.iter().map(row_to_package).collect())
    }

    pub async fn get_package(&self, id: &str) -> anyhow::Result<Option<TourPackage>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn.query_opt(&format!("{SELECT_PACKAGE} WHERE p.id = $1"), &[&uuid]).await?;
        Ok(row.as_ref().map(row_to_package))
    }

    pub async fn create_package(
        &self,
        p: &NewTourPackage,
        fasilitas: &[String],
        merchant_id: Option<&str>,
    ) -> anyhow::Result<String> {
        let merchant = merchant_id.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO tour_packages (merchant_id, judul, kategori, kawasan, durasi, label_badge,
                     label_tipe, rute, armada_info, fasilitas, harga_pax, min_pax, harga_charter, foto_url)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING id",
                &[
                    &merchant,
                    &p.judul,
                    &p.kategori,
                    &p.kawasan,
                    &p.durasi,
                    &p.label_badge,
                    &p.label_tipe,
                    &p.rute,
                    &p.armada_info,
                    &fasilitas,
                    &p.harga_pax,
                    &p.min_pax,
                    &p.harga_charter,
                    &p.foto_url,
                ],
            )
            .await?;
        Ok(row.get::<_, Uuid>("id").to_string())
    }

    // ── Bus charter ─────────────────────────────────────────────────────────

    pub async fn list_buses(&self, only_active: bool, merchant: Option<&str>) -> anyhow::Result<Vec<CharterBus>> {
        let merchant = merchant.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let po_ok = po_ok("b");
        let sql = format!(
            "{SELECT_BUS} WHERE ($1 = FALSE OR (b.aktif AND {po_ok})) AND ($2::uuid IS NULL OR b.merchant_id = $2)
              ORDER BY b.kapasitas DESC, b.harga_harian"
        );
        let rows = conn.query(&sql, &[&only_active, &merchant]).await?;
        Ok(rows.iter().map(row_to_bus).collect())
    }

    pub async fn get_bus(&self, id: &str) -> anyhow::Result<Option<CharterBus>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn.query_opt(&format!("{SELECT_BUS} WHERE b.id = $1"), &[&uuid]).await?;
        Ok(row.as_ref().map(row_to_bus))
    }

    pub async fn create_bus(&self, b: &NewCharterBus, fasilitas: &[String], merchant_id: Option<&str>) -> anyhow::Result<String> {
        let merchant = merchant_id.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO charter_buses (merchant_id, nama, tipe_bus, kelas, kapasitas, konfigurasi,
                     deskripsi, fasilitas, harga_harian, catatan_harga, foto_url)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 RETURNING id",
                &[
                    &merchant,
                    &b.nama,
                    &b.tipe_bus,
                    &b.kelas,
                    &b.kapasitas,
                    &b.konfigurasi,
                    &b.deskripsi,
                    &fasilitas,
                    &b.harga_harian,
                    &b.catatan_harga,
                    &b.foto_url,
                ],
            )
            .await?;
        Ok(row.get::<_, Uuid>("id").to_string())
    }

    // ── Helper umum paket/bus ───────────────────────────────────────────────

    /// `Ok(None)` = milik platform, `Ok(Some(id))` = milik merchant itu.
    pub async fn owner_of(&self, table: ListingTable, id: &str) -> anyhow::Result<Option<String>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(&format!("SELECT merchant_id FROM {} WHERE id = $1", table.name()), &[&uuid])
            .await?
            .ok_or_else(|| anyhow::anyhow!("Data tidak ditemukan"))?;
        Ok(row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()))
    }

    pub async fn set_active(&self, table: ListingTable, id: &str, aktif: bool) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute(&format!("UPDATE {} SET aktif = $2 WHERE id = $1", table.name()), &[&uuid, &aktif])
            .await?;
        Ok(())
    }

    pub async fn delete(&self, table: ListingTable, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute(&format!("DELETE FROM {} WHERE id = $1", table.name()), &[&uuid])
            .await?;
        Ok(())
    }

    // ── Permintaan sewa ─────────────────────────────────────────────────────

    pub async fn create_request(&self, r: &NewRequestRow<'_>) -> anyhow::Result<RentalRequest> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO rental_requests (jenis, item_id, item_nama, merchant_id, user_id, nama, telp,
                     jemput, tujuan, tgl_berangkat, tgl_pulang, tipe_perjalanan, jumlah_orang, catatan)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING id, jenis, item_nama, nama, telp, jemput, tujuan, tgl_berangkat, tgl_pulang,
                           tipe_perjalanan, jumlah_orang, catatan, status, created_at",
                &[
                    &r.jenis,
                    &r.item_id,
                    &r.item_nama,
                    &r.merchant_id,
                    &r.user_id,
                    &r.nama,
                    &r.telp,
                    &r.jemput,
                    &r.tujuan,
                    &r.tgl_berangkat,
                    &r.tgl_pulang,
                    &r.tipe_perjalanan,
                    &r.jumlah_orang,
                    &r.catatan,
                ],
            )
            .await?;
        Ok(row_to_request(&row))
    }

    /// `merchant = None` → semua permintaan (admin).
    pub async fn list_requests(&self, merchant: Option<&str>) -> anyhow::Result<Vec<RentalRequest>> {
        let merchant = merchant.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_REQUEST} WHERE ($1::uuid IS NULL OR merchant_id = $1) ORDER BY created_at DESC LIMIT 200");
        let rows = conn.query(&sql, &[&merchant]).await?;
        Ok(rows.iter().map(row_to_request).collect())
    }

    pub async fn request_owner(&self, id: &str) -> anyhow::Result<Option<String>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt("SELECT merchant_id FROM rental_requests WHERE id = $1", &[&uuid])
            .await?
            .ok_or_else(|| anyhow::anyhow!("Permintaan tidak ditemukan"))?;
        Ok(row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()))
    }

    pub async fn set_request_status(&self, id: &str, status: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("UPDATE rental_requests SET status = $2 WHERE id = $1", &[&uuid, &status])
            .await?;
        Ok(())
    }
}

fn row_to_package(row: &tokio_postgres::Row) -> TourPackage {
    TourPackage {
        id: row.get::<_, Uuid>("id").to_string(),
        merchant_id: row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()),
        pemilik: row.get("pemilik"),
        judul: row.get("judul"),
        kategori: row.get("kategori"),
        kawasan: row.get("kawasan"),
        durasi: row.get("durasi"),
        label_badge: row.get("label_badge"),
        label_tipe: row.get("label_tipe"),
        rute: row.get("rute"),
        armada_info: row.get("armada_info"),
        fasilitas: row.get("fasilitas"),
        harga_pax: row.get("harga_pax"),
        min_pax: row.get("min_pax"),
        harga_charter: row.get("harga_charter"),
        foto_url: row.get("foto_url"),
        aktif: row.get("aktif"),
    }
}

fn row_to_bus(row: &tokio_postgres::Row) -> CharterBus {
    CharterBus {
        id: row.get::<_, Uuid>("id").to_string(),
        merchant_id: row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()),
        pemilik: row.get("pemilik"),
        nama: row.get("nama"),
        tipe_bus: row.get("tipe_bus"),
        kelas: row.get("kelas"),
        kapasitas: row.get("kapasitas"),
        konfigurasi: row.get("konfigurasi"),
        deskripsi: row.get("deskripsi"),
        fasilitas: row.get("fasilitas"),
        harga_harian: row.get("harga_harian"),
        catatan_harga: row.get("catatan_harga"),
        foto_url: row.get("foto_url"),
        aktif: row.get("aktif"),
    }
}

fn row_to_request(row: &tokio_postgres::Row) -> RentalRequest {
    let fmt = |d: Option<NaiveDate>| d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
    let created_at: DateTime<Utc> = row.get("created_at");
    RentalRequest {
        id: row.get::<_, Uuid>("id").to_string(),
        jenis: row.get("jenis"),
        item_nama: row.get("item_nama"),
        nama: row.get("nama"),
        telp: row.get("telp"),
        jemput: row.get("jemput"),
        tujuan: row.get("tujuan"),
        tgl_berangkat: fmt(row.get("tgl_berangkat")),
        tgl_pulang: fmt(row.get("tgl_pulang")),
        tipe_perjalanan: row.get("tipe_perjalanan"),
        jumlah_orang: row.get("jumlah_orang"),
        catatan: row.get("catatan"),
        status: row.get("status"),
        created_at: created_at.to_rfc3339(),
    }
}
