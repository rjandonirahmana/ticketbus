use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::{NewRating, Rating};

#[derive(Clone)]
pub struct RatingRepository {
    pool: Pool,
}

impl RatingRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, buyer_id: &str, armada_id: &str, input: &NewRating) -> anyhow::Result<Rating> {
        if !(1..=5).contains(&input.rating_armada) || !(1..=5).contains(&input.rating_driver) {
            anyhow::bail!("Rating harus antara 1 dan 5");
        }
        let buyer_uuid = Uuid::parse_str(buyer_id)?;
        let armada_uuid = Uuid::parse_str(armada_id)?;
        let order_uuid = Uuid::parse_str(&input.order_id)?;
        let rating_armada = input.rating_armada as i16;
        let rating_driver = input.rating_driver as i16;

        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO ratings (order_id, buyer_id, armada_id, rating_armada, rating_driver, komentar)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING id, order_id, rating_armada, rating_driver, komentar, created_at",
                &[&order_uuid, &buyer_uuid, &armada_uuid, &rating_armada, &rating_driver, &input.komentar],
            )
            .await?;
        Ok(row_to_rating(&row))
    }

    /// `(rata2 rating armada, rata2 rating driver, jumlah rating)`.
    pub async fn avg_by_armada(&self, armada_id: &str) -> anyhow::Result<(f64, f64, i32)> {
        let armada_uuid = Uuid::parse_str(armada_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "SELECT COALESCE(AVG(rating_armada), 0)::float8 AS avg_armada,
                        COALESCE(AVG(rating_driver), 0)::float8 AS avg_driver,
                        COUNT(*)::int AS jumlah
                   FROM ratings WHERE armada_id = $1",
                &[&armada_uuid],
            )
            .await?;
        Ok((row.get("avg_armada"), row.get("avg_driver"), row.get("jumlah")))
    }
}

fn row_to_rating(row: &tokio_postgres::Row) -> Rating {
    let created_at: DateTime<Utc> = row.get("created_at");
    let rating_armada: i16 = row.get("rating_armada");
    let rating_driver: i16 = row.get("rating_driver");
    Rating {
        id: row.get::<_, Uuid>("id").to_string(),
        order_id: row.get::<_, Uuid>("order_id").to_string(),
        rating_armada: rating_armada as i32,
        rating_driver: rating_driver as i32,
        komentar: row.get("komentar"),
        created_at: created_at.to_rfc3339(),
    }
}
