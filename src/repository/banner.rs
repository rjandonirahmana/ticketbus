//! repository/banner.rs — banner promo beranda. Validasi ada di service.

use chrono::NaiveDate;
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::{Banner, NewBanner};

const SELECT: &str = "
    SELECT id, judul, subjudul, label, kode_promo, cta_label, link_url, gambar_url, tema,
           mulai, selesai, urutan, aktif
      FROM banners";

#[derive(Clone)]
pub struct BannerRepository {
    pool: Pool,
}

impl BannerRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Banner yang tayang pada `today` (aktif & dalam jadwal) — untuk beranda.
    pub async fn list_live(&self, today: NaiveDate) -> anyhow::Result<Vec<Banner>> {
        let conn = self.pool.get().await?;
        let sql = format!(
            "{SELECT} WHERE aktif AND (mulai IS NULL OR mulai <= $1) AND (selesai IS NULL OR selesai >= $1)
              ORDER BY urutan, created_at DESC LIMIT 10"
        );
        let rows = conn.query(&sql, &[&today]).await?;
        Ok(rows.iter().map(row_to_banner).collect())
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Banner>> {
        let conn = self.pool.get().await?;
        let rows = conn
            .query(&format!("{SELECT} ORDER BY urutan, created_at DESC"), &[])
            .await?;
        Ok(rows.iter().map(row_to_banner).collect())
    }

    pub async fn create(&self, b: &NewBanner, mulai: Option<NaiveDate>, selesai: Option<NaiveDate>) -> anyhow::Result<String> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO banners (judul, subjudul, label, kode_promo, cta_label, link_url, gambar_url,
                     tema, mulai, selesai, urutan)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 RETURNING id",
                &[
                    &b.judul,
                    &b.subjudul,
                    &b.label,
                    &b.kode_promo,
                    &b.cta_label,
                    &b.link_url,
                    &b.gambar_url,
                    &b.tema,
                    &mulai,
                    &selesai,
                    &b.urutan,
                ],
            )
            .await?;
        Ok(row.get::<_, Uuid>("id").to_string())
    }

    pub async fn update(
        &self,
        id: &str,
        b: &NewBanner,
        mulai: Option<NaiveDate>,
        selesai: Option<NaiveDate>,
    ) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let n = conn
            .execute(
                "UPDATE banners SET judul = $2, subjudul = $3, label = $4, kode_promo = $5, cta_label = $6,
                     link_url = $7, gambar_url = $8, tema = $9, mulai = $10, selesai = $11, urutan = $12
                 WHERE id = $1",
                &[
                    &uuid,
                    &b.judul,
                    &b.subjudul,
                    &b.label,
                    &b.kode_promo,
                    &b.cta_label,
                    &b.link_url,
                    &b.gambar_url,
                    &b.tema,
                    &mulai,
                    &selesai,
                    &b.urutan,
                ],
            )
            .await?;
        if n == 0 {
            anyhow::bail!("Banner tidak ditemukan");
        }
        Ok(())
    }

    pub async fn set_active(&self, id: &str, aktif: bool) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("UPDATE banners SET aktif = $2 WHERE id = $1", &[&uuid, &aktif])
            .await?;
        Ok(())
    }

    /// "Rilis sekarang": aktifkan dan mulai tayang hari ini.
    pub async fn release_now(&self, id: &str, today: NaiveDate) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE banners SET aktif = TRUE, mulai = $2,
                 selesai = CASE WHEN selesai < $2 THEN NULL ELSE selesai END
             WHERE id = $1",
            &[&uuid, &today],
        )
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM banners WHERE id = $1", &[&uuid]).await?;
        Ok(())
    }
}

fn row_to_banner(row: &tokio_postgres::Row) -> Banner {
    let fmt = |d: Option<NaiveDate>| d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
    Banner {
        id: row.get::<_, Uuid>("id").to_string(),
        judul: row.get("judul"),
        subjudul: row.get("subjudul"),
        label: row.get("label"),
        kode_promo: row.get("kode_promo"),
        cta_label: row.get("cta_label"),
        link_url: row.get("link_url"),
        gambar_url: row.get("gambar_url"),
        tema: row.get("tema"),
        mulai: fmt(row.get("mulai")),
        selesai: fmt(row.get("selesai")),
        urutan: row.get("urutan"),
        aktif: row.get("aktif"),
    }
}
