//! repository/merchant.rs — profil Mitra PO + data halaman profil publik.

use deadpool_postgres::Pool;
use uuid::Uuid;

use super::schedule::{row_to_schedule, PO_TAMPIL, SELECT_JOIN_ARMADA};
use crate::web::models::{MerchantProfile, NewMerchantProfile, Schedule};

const SELECT: &str = "
    SELECT mp.user_id, mp.nama_po, mp.kota, mp.alamat, mp.deskripsi, mp.tahun_berdiri, mp.jumlah_armada,
           mp.layanan, mp.nama_pemilik, mp.email, mp.nomor_izin, mp.dokumen_url, mp.logo_url, mp.sampul_url,
           mp.status, mp.catatan_admin, u.phone AS telp, mp.created_at,
           (SELECT COUNT(*) FROM armadas a WHERE a.merchant_id = mp.user_id)::int AS armada_aktif,
           (SELECT COUNT(*) FROM routes r WHERE r.merchant_id = mp.user_id AND r.aktif)::int AS trayek_aktif
      FROM merchant_profiles mp
      JOIN users u ON u.id = mp.user_id";

/// Profil yang sudah divalidasi service.
pub struct ProfileRow<'a> {
    pub p: &'a NewMerchantProfile,
    pub tahun_berdiri: Option<i32>,
    pub layanan: Vec<String>,
}

#[derive(Clone)]
pub struct MerchantRepository {
    pool: Pool,
}

impl MerchantRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, user_id: &str) -> anyhow::Result<Option<MerchantProfile>> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let row = conn.query_opt(&format!("{SELECT} WHERE mp.user_id = $1"), &[&uuid]).await?;
        Ok(row.as_ref().map(row_to_profile))
    }

    /// `status = None` → semua. Menunggu tampil paling atas.
    pub async fn list(&self, status: Option<&str>) -> anyhow::Result<Vec<MerchantProfile>> {
        let conn = self.pool.get().await?;
        let rows = conn
            .query(
                &format!(
                    "{SELECT} WHERE ($1::text IS NULL OR mp.status = $1)
                      ORDER BY (mp.status = 'menunggu') DESC, mp.created_at DESC"
                ),
                &[&status],
            )
            .await?;
        Ok(rows.iter().map(row_to_profile).collect())
    }

    /// Buat profil saat akun mitra baru terverifikasi (sekali; tak menimpa).
    pub async fn create(&self, user_id: &str, r: &ProfileRow<'_>) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO merchant_profiles (user_id, nama_po, kota, alamat, deskripsi, tahun_berdiri,
                 jumlah_armada, layanan, nama_pemilik, email, nomor_izin, dokumen_url, logo_url, sampul_url)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (user_id) DO NOTHING",
            &[
                &uuid,
                &r.p.nama_po,
                &r.p.kota,
                &r.p.alamat,
                &r.p.deskripsi,
                &r.tahun_berdiri,
                &r.p.jumlah_armada,
                &r.layanan,
                &r.p.nama_pemilik,
                &r.p.email,
                &r.p.nomor_izin,
                &r.p.dokumen_url,
                &r.p.logo_url,
                &r.p.sampul_url,
            ],
        )
        .await?;
        Ok(())
    }

    /// Ubah profil oleh pemiliknya. Profil yang ditolak otomatis diajukan
    /// ulang (kembali `menunggu`); yang sudah disetujui tetap disetujui.
    pub async fn update(&self, user_id: &str, r: &ProfileRow<'_>) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let n = conn
            .execute(
                "UPDATE merchant_profiles SET nama_po = $2, kota = $3, alamat = $4, deskripsi = $5,
                     tahun_berdiri = $6, jumlah_armada = $7, layanan = $8, nama_pemilik = $9, email = $10,
                     nomor_izin = $11, dokumen_url = $12, logo_url = $13, sampul_url = $14, updated_at = NOW(),
                     status = CASE WHEN status = 'ditolak' THEN 'menunggu' ELSE status END
                 WHERE user_id = $1",
                &[
                    &uuid,
                    &r.p.nama_po,
                    &r.p.kota,
                    &r.p.alamat,
                    &r.p.deskripsi,
                    &r.tahun_berdiri,
                    &r.p.jumlah_armada,
                    &r.layanan,
                    &r.p.nama_pemilik,
                    &r.p.email,
                    &r.p.nomor_izin,
                    &r.p.dokumen_url,
                    &r.p.logo_url,
                    &r.p.sampul_url,
                ],
            )
            .await?;
        if n == 0 {
            anyhow::bail!("Profil PO tidak ditemukan");
        }
        Ok(())
    }

    pub async fn set_status(&self, user_id: &str, status: &str, catatan: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let n = conn
            .execute(
                "UPDATE merchant_profiles SET status = $2, catatan_admin = $3, diputuskan_at = NOW() WHERE user_id = $1",
                &[&uuid, &status, &catatan],
            )
            .await?;
        if n == 0 {
            anyhow::bail!("Profil PO tidak ditemukan");
        }
        Ok(())
    }

    /// Keberangkatan mendatang milik PO (untuk halaman publik).
    pub async fn upcoming(&self, user_id: &str) -> anyhow::Result<Vec<Schedule>> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let sql = format!(
            "{SELECT_JOIN_ARMADA} WHERE a.merchant_id = $1 AND s.tanggal >= CURRENT_DATE AND NOT s.batal AND {PO_TAMPIL}
              ORDER BY s.tanggal, s.jam LIMIT 200"
        );
        let rows = conn.query(&sql, &[&uuid]).await?;
        Ok(rows.iter().map(row_to_schedule).collect())
    }
}

fn row_to_profile(row: &tokio_postgres::Row) -> MerchantProfile {
    let created: chrono::DateTime<chrono::Utc> = row.get("created_at");
    MerchantProfile {
        user_id: row.get::<_, Uuid>("user_id").to_string(),
        nama_po: row.get("nama_po"),
        kota: row.get("kota"),
        alamat: row.get("alamat"),
        deskripsi: row.get("deskripsi"),
        tahun_berdiri: row.get("tahun_berdiri"),
        jumlah_armada: row.get("jumlah_armada"),
        layanan: row.get("layanan"),
        nama_pemilik: row.get("nama_pemilik"),
        email: row.get("email"),
        nomor_izin: row.get("nomor_izin"),
        dokumen_url: row.get("dokumen_url"),
        logo_url: row.get("logo_url"),
        sampul_url: row.get("sampul_url"),
        status: row.get("status"),
        catatan_admin: row.get("catatan_admin"),
        telp: row.get("telp"),
        created_at: created.to_rfc3339(),
        armada_aktif: row.get("armada_aktif"),
        trayek_aktif: row.get("trayek_aktif"),
    }
}
