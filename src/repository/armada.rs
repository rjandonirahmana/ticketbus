use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::Armada;

const SELECT_WITH_RATING: &str = "
    SELECT a.id, a.name, a.color_hex, a.urutan, a.merchant_id,
           COALESCE(AVG(r.rating_armada), 0)::float8 AS avg_armada,
           COALESCE(AVG(r.rating_driver), 0)::float8 AS avg_driver,
           COUNT(r.id)::int AS jumlah_rating
      FROM armadas a
      LEFT JOIN ratings r ON r.armada_id = a.id";

#[derive(Clone)]
pub struct ArmadaRepository {
    pool: Pool,
}

impl ArmadaRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Semua armada (platform + seluruh merchant) — dipakai buyer browse & admin.
    pub async fn list(&self) -> anyhow::Result<Vec<Armada>> {
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_WITH_RATING} GROUP BY a.id ORDER BY a.urutan, a.name");
        let rows = conn.query(&sql, &[]).await?;
        Ok(rows.iter().map(row_to_armada).collect())
    }

    /// Armada milik satu merchant saja — dipakai halaman `/merchant`.
    pub async fn list_by_merchant(&self, merchant_id: &str) -> anyhow::Result<Vec<Armada>> {
        let merchant_uuid = Uuid::parse_str(merchant_id)?;
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_WITH_RATING} WHERE a.merchant_id = $1 GROUP BY a.id ORDER BY a.urutan, a.name");
        let rows = conn.query(&sql, &[&merchant_uuid]).await?;
        Ok(rows.iter().map(row_to_armada).collect())
    }

    /// `Ok(None)` = armada milik platform (admin). `Ok(Some(id))` = milik
    /// merchant itu. `Err` = armada tak ditemukan. Dipakai gerbang kepemilikan
    /// di service layer sebelum update/delete/upload foto oleh merchant.
    pub async fn merchant_id_of(&self, id: &str) -> anyhow::Result<Option<String>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt("SELECT merchant_id FROM armadas WHERE id = $1", &[&uuid])
            .await?
            .ok_or_else(|| anyhow::anyhow!("Armada tidak ditemukan"))?;
        Ok(row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()))
    }

    pub async fn create(&self, name: &str, color_hex: &str, merchant_id: Option<&str>) -> anyhow::Result<Armada> {
        let merchant_uuid = merchant_id.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let urutan: i32 = conn
            .query_one("SELECT COALESCE(MAX(urutan), 0) + 1 AS n FROM armadas", &[])
            .await?
            .get("n");
        let row = conn
            .query_one(
                "INSERT INTO armadas (name, color_hex, urutan, merchant_id) VALUES ($1, $2, $3, $4)
                 RETURNING id, name, color_hex, urutan, merchant_id",
                &[&name, &color_hex, &urutan, &merchant_uuid],
            )
            .await?;
        Ok(Armada {
            id: row.get::<_, Uuid>("id").to_string(),
            name: row.get("name"),
            color_hex: row.get("color_hex"),
            urutan: row.get("urutan"),
            merchant_id: row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()),
            avg_rating_armada: 0.0,
            avg_rating_driver: 0.0,
            jumlah_rating: 0,
        })
    }

    pub async fn update(&self, id: &str, name: &str, color_hex: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE armadas SET name = $2, color_hex = $3 WHERE id = $1",
            &[&uuid, &name, &color_hex],
        )
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM armadas WHERE id = $1", &[&uuid])
            .await?;
        Ok(())
    }
}

fn row_to_armada(row: &tokio_postgres::Row) -> Armada {
    Armada {
        id: row.get::<_, Uuid>("id").to_string(),
        name: row.get("name"),
        color_hex: row.get("color_hex"),
        urutan: row.get("urutan"),
        merchant_id: row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()),
        avg_rating_armada: row.get("avg_armada"),
        avg_rating_driver: row.get("avg_driver"),
        jumlah_rating: row.get("jumlah_rating"),
    }
}
