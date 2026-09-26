use chrono::NaiveDate;
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::{BusPosition, NearbyBus, NewSchedule, Schedule};

const SELECT_JOIN_ARMADA: &str = "
    SELECT s.id, s.armada_id, a.name AS armada_name, a.color_hex AS armada_color_hex,
           s.tanggal, s.tujuan, s.lokasi_jemput, s.jam, s.harga, s.catatan,
           s.kapasitas, s.kursi_terjual, s.driver_nama, s.driver_telp,
           s.konfigurasi, s.dua_dek, s.kursi_wanita, s.jemput_lat, s.jemput_lng
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
        let jemput = parse_koordinat(&input.jemput_lat, &input.jemput_lng)?;
        let conn = self.pool.get().await?;
        let id: Uuid = conn
            .query_one(
                "INSERT INTO schedules (armada_id, tanggal, tujuan, lokasi_jemput, jam, harga, catatan, kapasitas,
                                        driver_nama, driver_telp, konfigurasi, dua_dek, kursi_wanita,
                                        jemput_lat, jemput_lng)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
                    &jemput.map(|c| c.0),
                    &jemput.map(|c| c.1),
                ],
            )
            .await?
            .get("id");
        self.get(&id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("jadwal hilang setelah dibuat"))
    }

    /// Token tautan driver — HANYA untuk pemilik jadwal & admin (dicek service).
    pub async fn tracking_token(&self, id: &str) -> anyhow::Result<Option<String>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt("SELECT tracking_token FROM schedules WHERE id = $1", &[&uuid])
            .await?;
        Ok(row.map(|r| r.get("tracking_token")))
    }

    pub async fn by_token(&self, token: &str) -> anyhow::Result<Option<Schedule>> {
        let conn = self.pool.get().await?;
        let sql = format!("{SELECT_JOIN_ARMADA} WHERE s.tracking_token = $1");
        let row = conn.query_opt(&sql, &[&token]).await?;
        Ok(row.as_ref().map(row_to_schedule))
    }

    pub async fn upsert_position(
        &self,
        schedule_id: &str,
        lat: f64,
        lng: f64,
        speed_kmh: Option<f64>,
        heading: Option<f64>,
        accuracy_m: Option<f64>,
    ) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(schedule_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO bus_positions (schedule_id, lat, lng, speed_kmh, heading, accuracy_m, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW())
             ON CONFLICT (schedule_id) DO UPDATE
               SET lat = EXCLUDED.lat, lng = EXCLUDED.lng, speed_kmh = EXCLUDED.speed_kmh,
                   heading = EXCLUDED.heading, accuracy_m = EXCLUDED.accuracy_m, updated_at = NOW()",
            &[&uuid, &lat, &lng, &speed_kmh, &heading, &accuracy_m],
        )
        .await?;
        Ok(())
    }

    pub async fn delete_position(&self, schedule_id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(schedule_id)?;
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM bus_positions WHERE schedule_id = $1", &[&uuid]).await?;
        Ok(())
    }

    /// Posisi terakhir satu jadwal (sinyal ≤ 10 menit).
    pub async fn position(&self, schedule_id: &str) -> anyhow::Result<Option<BusPosition>> {
        let uuid = Uuid::parse_str(schedule_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(
                "SELECT lat, lng, speed_kmh, heading,
                        EXTRACT(EPOCH FROM NOW() - updated_at)::bigint AS umur
                   FROM bus_positions
                  WHERE schedule_id = $1 AND updated_at > NOW() - INTERVAL '10 minutes'",
                &[&uuid],
            )
            .await?;
        Ok(row.as_ref().map(row_to_position))
    }

    /// Jadwal `tanggal` yang punya lokasi: posisi live (≤ 10 menit) dan/atau
    /// koordinat titik jemput. Untuk radar beranda & halaman Peta Bus.
    pub async fn with_location(&self, tanggal: NaiveDate) -> anyhow::Result<Vec<NearbyBus>> {
        let conn = self.pool.get().await?;
        let sql = format!(
            "SELECT q.*, bp.lat AS bp_lat, bp.lng AS bp_lng, bp.speed_kmh AS bp_speed, bp.heading AS bp_heading,
                    EXTRACT(EPOCH FROM NOW() - bp.updated_at)::bigint AS bp_umur
               FROM ({SELECT_JOIN_ARMADA} WHERE s.tanggal = $1) q
               LEFT JOIN bus_positions bp
                      ON bp.schedule_id = q.id AND bp.updated_at > NOW() - INTERVAL '10 minutes'
              WHERE q.jemput_lat IS NOT NULL OR bp.schedule_id IS NOT NULL
              ORDER BY q.jam"
        );
        let rows = conn.query(&sql, &[&tanggal]).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let lat: Option<f64> = r.get("bp_lat");
                NearbyBus {
                    schedule: row_to_schedule(r),
                    posisi: lat.map(|lat| BusPosition {
                        lat,
                        lng: r.get("bp_lng"),
                        speed_kmh: r.get("bp_speed"),
                        heading: r.get("bp_heading"),
                        umur_detik: r.get("bp_umur"),
                    }),
                }
            })
            .collect())
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

fn row_to_position(r: &tokio_postgres::Row) -> BusPosition {
    BusPosition {
        lat: r.get("lat"),
        lng: r.get("lng"),
        speed_kmh: r.get("speed_kmh"),
        heading: r.get("heading"),
        umur_detik: r.get("umur"),
    }
}

/// "" / "" → None; keduanya harus terisi & valid bila salah satunya diisi.
fn parse_koordinat(lat: &str, lng: &str) -> anyhow::Result<Option<(f64, f64)>> {
    let (lat, lng) = (lat.trim(), lng.trim());
    if lat.is_empty() && lng.is_empty() {
        return Ok(None);
    }
    let (Ok(a), Ok(b)) = (lat.parse::<f64>(), lng.parse::<f64>()) else {
        anyhow::bail!("Koordinat titik jemput tidak valid");
    };
    if !crate::web::geo::koordinat_valid(a, b) {
        anyhow::bail!("Koordinat titik jemput di luar jangkauan");
    }
    Ok(Some((a, b)))
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
        jemput_lat: row.get("jemput_lat"),
        jemput_lng: row.get("jemput_lng"),
    }
}
