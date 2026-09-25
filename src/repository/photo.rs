use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::TripPhoto;

const SELECT_JOIN_ARMADA: &str = "
    SELECT p.id, p.armada_id, a.name AS armada_name, p.url, p.caption, p.created_at
      FROM trip_photos p
      JOIN armadas a ON a.id = p.armada_id";

#[derive(Clone)]
pub struct TripPhotoRepository {
    pool: Pool,
}

impl TripPhotoRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, armada_id: Option<&str>) -> anyhow::Result<Vec<TripPhoto>> {
        let conn = self.pool.get().await?;
        let rows = match armada_id {
            Some(id) => {
                let uuid = Uuid::parse_str(id)?;
                let sql = format!("{SELECT_JOIN_ARMADA} WHERE p.armada_id = $1 ORDER BY p.created_at DESC");
                conn.query(&sql, &[&uuid]).await?
            }
            None => {
                let sql = format!("{SELECT_JOIN_ARMADA} ORDER BY p.created_at DESC");
                conn.query(&sql, &[]).await?
            }
        };
        Ok(rows.iter().map(row_to_photo).collect())
    }

    /// Foto dari SEMUA armada milik satu merchant.
    pub async fn list_by_merchant(&self, merchant_id: &str) -> anyhow::Result<Vec<TripPhoto>> {
        let merchant_uuid = Uuid::parse_str(merchant_id)?;
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_JOIN_ARMADA} WHERE a.merchant_id = $1 ORDER BY p.created_at DESC");
        let rows = conn.query(&sql, &[&merchant_uuid]).await?;
        Ok(rows.iter().map(row_to_photo).collect())
    }

    pub async fn create(&self, armada_id: &str, url: &str, caption: &str) -> anyhow::Result<TripPhoto> {
        let uuid = Uuid::parse_str(armada_id)?;
        let conn = self.pool.get().await?;
        let id: Uuid = conn
            .query_one(
                "INSERT INTO trip_photos (armada_id, url, caption) VALUES ($1, $2, $3) RETURNING id",
                &[&uuid, &url, &caption],
            )
            .await?
            .get("id");
        let sql = format!("{SELECT_JOIN_ARMADA} WHERE p.id = $1");
        let row = conn.query_one(&sql, &[&id]).await?;
        Ok(row_to_photo(&row))
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM trip_photos WHERE id = $1", &[&uuid])
            .await?;
        Ok(())
    }

    /// `armada_id` pemilik satu foto — dipakai gerbang kepemilikan merchant
    /// sebelum menghapus (lihat `web/api/server_fns/photo.rs`).
    pub async fn armada_id_of(&self, id: &str) -> anyhow::Result<String> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt("SELECT armada_id FROM trip_photos WHERE id = $1", &[&uuid])
            .await?
            .ok_or_else(|| anyhow::anyhow!("Foto tidak ditemukan"))?;
        Ok(row.get::<_, Uuid>("armada_id").to_string())
    }
}

fn row_to_photo(row: &tokio_postgres::Row) -> TripPhoto {
    let created_at: DateTime<Utc> = row.get("created_at");
    TripPhoto {
        id: row.get::<_, Uuid>("id").to_string(),
        armada_id: row.get::<_, Uuid>("armada_id").to_string(),
        armada_name: row.get("armada_name"),
        url: row.get("url"),
        caption: row.get("caption"),
        created_at: created_at.to_rfc3339(),
    }
}
