use chrono::NaiveDate;
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::{NewSchedule, Schedule};

const SELECT_JOIN_ARMADA: &str = "
    SELECT s.id, s.armada_id, a.name AS armada_name, a.color_hex AS armada_color_hex,
           s.tanggal, s.tujuan, s.lokasi_jemput, s.jam, s.harga, s.catatan,
           s.kapasitas, s.kursi_terjual, s.driver_nama, s.driver_telp,
           s.konfigurasi, s.dua_dek, s.kursi_wanita
      FROM schedules s
      JOIN armadas a ON a.id = s.armada_id";

#[derive(Clone)]
pub struct ScheduleRepository {
    pool: Pool,
}

impl ScheduleRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list_by_month(&self, year: i32, month: u32) -> anyhow::Result<Vec<Schedule>> {
        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| anyhow::anyhow!("bulan tak valid"))?;
        let end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| anyhow::anyhow!("bulan tak valid"))?;

        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_JOIN_ARMADA} WHERE s.tanggal >= $1 AND s.tanggal < $2 ORDER BY s.tanggal, s.jam");
        let rows = conn.query(&sql, &[&start, &end]).await?;
        Ok(rows.iter().map(row_to_schedule).collect())
    }

    /// Jadwal akan datang (>= hari ini), lintas seluruh armada — dipakai
    /// halaman browse publik (`/`).
    pub async fn list_upcoming(&self) -> anyhow::Result<Vec<Schedule>> {
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_JOIN_ARMADA} WHERE s.tanggal >= CURRENT_DATE ORDER BY s.tanggal, s.jam");
        let rows = conn.query(&sql, &[]).await?;
        Ok(rows.iter().map(row_to_schedule).collect())
    }

    pub async fn list_by_armada(&self, armada_id: Option<&str>) -> anyhow::Result<Vec<Schedule>> {
        let conn = self.pool.get().await?;
        let rows = match armada_id {
            Some(id) => {
                let uuid = Uuid::parse_str(id)?;
                let sql = format!("{SELECT_JOIN_ARMADA} WHERE s.armada_id = $1 ORDER BY s.tanggal DESC, s.jam");
                conn.query(&sql, &[&uuid]).await?
            }
            None => {
                let sql = format!("{SELECT_JOIN_ARMADA} ORDER BY s.tanggal DESC, s.jam");
                conn.query(&sql, &[]).await?
            }
        };
        Ok(rows.iter().map(row_to_schedule).collect())
    }

    /// Jadwal milik armada-armada SATU merchant — dipakai halaman `/merchant`.
    pub async fn list_by_merchant(&self, merchant_id: &str) -> anyhow::Result<Vec<Schedule>> {
        let merchant_uuid = Uuid::parse_str(merchant_id)?;
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_JOIN_ARMADA} WHERE a.merchant_id = $1 ORDER BY s.tanggal DESC, s.jam");
        let rows = conn.query(&sql, &[&merchant_uuid]).await?;
        Ok(rows.iter().map(row_to_schedule).collect())
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Schedule>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_JOIN_ARMADA} WHERE s.id = $1");
        let row = conn.query_opt(&sql, &[&uuid]).await?;
        Ok(row.map(|r| row_to_schedule(&r)))
    }

    /// `armada_id` pemilik jadwal — dipakai gerbang kepemilikan merchant
    /// (join lewat armada, sama seperti `ArmadaRepository::merchant_id_of`).
    pub async fn armada_id_of(&self, schedule_id: &str) -> anyhow::Result<String> {
        let uuid = Uuid::parse_str(schedule_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt("SELECT armada_id FROM schedules WHERE id = $1", &[&uuid])
            .await?
            .ok_or_else(|| anyhow::anyhow!("Jadwal tidak ditemukan"))?;
        Ok(row.get::<_, Uuid>("armada_id").to_string())
    }

    pub async fn create(&self, input: &NewSchedule) -> anyhow::Result<Schedule> {
        let armada_uuid = Uuid::parse_str(&input.armada_id)?;
        let tanggal = NaiveDate::parse_from_str(&input.tanggal, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("format tanggal harus YYYY-MM-DD"))?;
        if input.kapasitas <= 0 {
            anyhow::bail!("Kapasitas harus lebih dari 0");
        }
        let kursi_wanita = crate::web::seats::parse_kode_list(&input.kursi_wanita);
        let conn = self.pool.get().await?;
        let id: Uuid = conn
            .query_one(
                "INSERT INTO schedules (armada_id, tanggal, tujuan, lokasi_jemput, jam, harga, catatan, kapasitas,
                                        driver_nama, driver_telp, konfigurasi, dua_dek, kursi_wanita)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                 RETURNING id",
                &[
                    &armada_uuid,
                    &tanggal,
                    &input.tujuan,
                    &input.lokasi_jemput,
                    &input.jam,
                    &input.harga,
                    &input.catatan,
                    &input.kapasitas,
                    &input.driver_nama,
                    &input.driver_telp,
                    &input.konfigurasi,
                    &input.dua_dek,
                    &kursi_wanita,
                ],
            )
            .await?
            .get("id");
        self.get(&id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("jadwal hilang setelah dibuat"))
    }

    /// Kode kursi yang sudah terjual (bernomor) untuk satu jadwal.
    pub async fn seats_taken(&self, id: &str) -> anyhow::Result<Vec<String>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let rows = conn
            .query("SELECT kode FROM order_seats WHERE schedule_id = $1 ORDER BY kode", &[&uuid])
            .await?;
        Ok(rows.iter().map(|r| r.get("kode")).collect())
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM schedules WHERE id = $1", &[&uuid])
            .await?;
        Ok(())
    }
}

fn row_to_schedule(row: &tokio_postgres::Row) -> Schedule {
    let tanggal: NaiveDate = row.get("tanggal");
    Schedule {
        id: row.get::<_, Uuid>("id").to_string(),
        armada_id: row.get::<_, Uuid>("armada_id").to_string(),
        armada_name: row.get("armada_name"),
        armada_color_hex: row.get("armada_color_hex"),
        tanggal: tanggal.format("%Y-%m-%d").to_string(),
        tujuan: row.get("tujuan"),
        lokasi_jemput: row.get("lokasi_jemput"),
        jam: row.get("jam"),
        harga: row.get("harga"),
        catatan: row.get("catatan"),
        kapasitas: row.get("kapasitas"),
        kursi_terjual: row.get("kursi_terjual"),
        driver_nama: row.get("driver_nama"),
        driver_telp: row.get("driver_telp"),
        konfigurasi: row.get("konfigurasi"),
        dua_dek: row.get("dua_dek"),
        kursi_wanita: row.get("kursi_wanita"),
    }
}
