//! repository/route.rs — trayek tetap + jadwal harian yang dibuat darinya.
//! Validasi & kepemilikan di service/route.rs.

use chrono::NaiveDate;
use deadpool_postgres::Pool;
use uuid::Uuid;

use super::schedule::{row_to_schedule, SELECT_JOIN_ARMADA};
use crate::web::models::{Route, Schedule};

const SELECT_ROUTE: &str = "
    SELECT r.id, r.merchant_id, r.pasangan_id, r.asal, r.tujuan, r.lokasi_jemput, r.jemput_lat, r.jemput_lng,
           r.jam_berangkat, r.jam_tiba, r.harga, r.kapasitas, r.konfigurasi, r.dua_dek, r.kursi_wanita,
           r.catatan, r.armada_id, COALESCE(a.name, '') AS armada_name, r.driver_nama, r.driver_telp,
           r.hari_operasi, r.aktif
      FROM routes r
      LEFT JOIN armadas a ON a.id = r.armada_id";

/// Jadwal "tanpa order" — hanya ini yang boleh dihapus/diubah denahnya
/// (orders.schedule_id tak ber-CASCADE, dan tiket terjual tak boleh hilang).
const TANPA_ORDER: &str = "NOT EXISTS (SELECT 1 FROM orders o WHERE o.schedule_id = schedules.id)";

/// Data trayek yang sudah divalidasi service.
pub struct RouteRow {
    pub merchant_id: Option<Uuid>,
    pub asal: String,
    pub tujuan: String,
    pub lokasi_jemput: String,
    pub jemput: Option<(f64, f64)>,
    pub jam_berangkat: String,
    pub jam_tiba: String,
    pub harga: i64,
    pub kapasitas: i32,
    pub konfigurasi: String,
    pub dua_dek: bool,
    pub kursi_wanita: Vec<String>,
    pub catatan: String,
    pub armada_id: Uuid,
    pub driver_nama: String,
    pub driver_telp: String,
    pub hari_operasi: i16,
}

#[derive(Clone)]
pub struct RouteRepository {
    pool: Pool,
}

impl RouteRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// `merchant = None` → semua trayek (admin).
    pub async fn list(&self, merchant: Option<&str>) -> anyhow::Result<Vec<Route>> {
        let merchant = merchant.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let sql = format!(
            "{SELECT_ROUTE} WHERE ($1::uuid IS NULL OR r.merchant_id = $1)
              ORDER BY LEAST(r.asal, r.tujuan), GREATEST(r.asal, r.tujuan), r.jam_berangkat"
        );
        let rows = conn.query(&sql, &[&merchant]).await?;
        Ok(rows.iter().map(row_to_route).collect())
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Route>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn.query_opt(&format!("{SELECT_ROUTE} WHERE r.id = $1"), &[&uuid]).await?;
        Ok(row.as_ref().map(row_to_route))
    }

    pub async fn create(&self, r: &RouteRow) -> anyhow::Result<Uuid> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO routes (merchant_id, asal, tujuan, lokasi_jemput, jemput_lat, jemput_lng,
                     jam_berangkat, jam_tiba, harga, kapasitas, konfigurasi, dua_dek, kursi_wanita, catatan,
                     armada_id, driver_nama, driver_telp, hari_operasi)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                 RETURNING id",
                &[
                    &r.merchant_id,
                    &r.asal,
                    &r.tujuan,
                    &r.lokasi_jemput,
                    &r.jemput.map(|c| c.0),
                    &r.jemput.map(|c| c.1),
                    &r.jam_berangkat,
                    &r.jam_tiba,
                    &r.harga,
                    &r.kapasitas,
                    &r.konfigurasi,
                    &r.dua_dek,
                    &r.kursi_wanita,
                    &r.catatan,
                    &r.armada_id,
                    &r.driver_nama,
                    &r.driver_telp,
                    &r.hari_operasi,
                ],
            )
            .await?;
        Ok(row.get("id"))
    }

    /// Tautkan dua trayek sebagai pasangan pergi–balik.
    pub async fn link_pair(&self, a: Uuid, b: Uuid) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE routes SET pasangan_id = CASE WHEN id = $1 THEN $2 ELSE $1 END WHERE id IN ($1, $2)",
            &[&a, &b],
        )
        .await?;
        Ok(())
    }

    /// Ubah trayek lalu teruskan perubahan ke jadwalnya mulai `today`:
    /// - rute, jam, harga, catatan → semua jadwal (harga order lama tetap,
    ///   karena total order disimpan saat pembelian);
    /// - denah kursi → hanya jadwal yang belum punya order;
    /// - bus & driver → hanya hari yang masih memakai default LAMA (hari yang
    ///   sudah diganti manual tidak ditimpa);
    /// - hari yang tak lagi beroperasi & belum punya order → dihapus.
    pub async fn update(&self, id: &str, old: &Route, r: &RouteRow, today: NaiveDate) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let old_armada = old.armada_id.as_deref().map(Uuid::parse_str).transpose()?;
        let mut conn = self.pool.get().await?;
        let tx = conn.transaction().await?;
        tx.execute(
            "UPDATE routes SET asal = $2, tujuan = $3, lokasi_jemput = $4, jemput_lat = $5, jemput_lng = $6,
                 jam_berangkat = $7, jam_tiba = $8, harga = $9, kapasitas = $10, konfigurasi = $11,
                 dua_dek = $12, kursi_wanita = $13, catatan = $14, armada_id = $15, driver_nama = $16,
                 driver_telp = $17, hari_operasi = $18
             WHERE id = $1",
            &[
                &uuid,
                &r.asal,
                &r.tujuan,
                &r.lokasi_jemput,
                &r.jemput.map(|c| c.0),
                &r.jemput.map(|c| c.1),
                &r.jam_berangkat,
                &r.jam_tiba,
                &r.harga,
                &r.kapasitas,
                &r.konfigurasi,
                &r.dua_dek,
                &r.kursi_wanita,
                &r.catatan,
                &r.armada_id,
                &r.driver_nama,
                &r.driver_telp,
                &r.hari_operasi,
            ],
        )
        .await?;
        tx.execute(
            "UPDATE schedules SET asal = $3, tujuan = $4, lokasi_jemput = $5, jemput_lat = $6, jemput_lng = $7,
                 jam = $8, jam_tiba = $9, harga = $10, catatan = $11
             WHERE route_id = $1 AND tanggal >= $2",
            &[
                &uuid,
                &today,
                &r.asal,
                &r.tujuan,
                &r.lokasi_jemput,
                &r.jemput.map(|c| c.0),
                &r.jemput.map(|c| c.1),
                &r.jam_berangkat,
                &r.jam_tiba,
                &r.harga,
                &r.catatan,
            ],
        )
        .await?;
        tx.execute(
            &format!(
                "UPDATE schedules SET kapasitas = $3, konfigurasi = $4, dua_dek = $5, kursi_wanita = $6
                  WHERE route_id = $1 AND tanggal >= $2 AND {TANPA_ORDER}"
            ),
            &[&uuid, &today, &r.kapasitas, &r.konfigurasi, &r.dua_dek, &r.kursi_wanita],
        )
        .await?;
        if let Some(old_armada) = old_armada {
            tx.execute(
                "UPDATE schedules SET armada_id = $4 WHERE route_id = $1 AND tanggal >= $2 AND armada_id = $3",
                &[&uuid, &today, &old_armada, &r.armada_id],
            )
            .await?;
        }
        tx.execute(
            "UPDATE schedules SET driver_nama = $5, driver_telp = $6
              WHERE route_id = $1 AND tanggal >= $2 AND driver_nama = $3 AND driver_telp = $4",
            &[&uuid, &today, &old.driver_nama, &old.driver_telp, &r.driver_nama, &r.driver_telp],
        )
        .await?;
        tx.execute(
            &format!(
                "DELETE FROM schedules
                  WHERE route_id = $1 AND tanggal >= $2 AND {TANPA_ORDER}
                    AND ($3::int & (1 << (EXTRACT(ISODOW FROM tanggal)::int - 1))) = 0"
            ),
            &[&uuid, &today, &(r.hari_operasi as i32)],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn set_active(&self, id: &str, aktif: bool) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("UPDATE routes SET aktif = $2 WHERE id = $1", &[&uuid, &aktif]).await?;
        Ok(())
    }

    /// Hapus jadwal mendatang trayek ini yang belum punya order (dipakai saat
    /// trayek dijeda/dihapus). Jadwal dengan tiket terjual tetap jalan.
    pub async fn clear_future(&self, id: &str, today: NaiveDate) -> anyhow::Result<u64> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        Ok(conn
            .execute(
                &format!("DELETE FROM schedules WHERE route_id = $1 AND tanggal >= $2 AND {TANPA_ORDER}"),
                &[&uuid, &today],
            )
            .await?)
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM routes WHERE id = $1", &[&uuid]).await?;
        Ok(())
    }

    /// Buat jadwal harian `from` .. `from + hari - 1` untuk trayek aktif
    /// (`route = None` → semua trayek). Idempoten: tanggal yang sudah ada
    /// (termasuk yang dibatalkan) dilewati lewat ON CONFLICT.
    pub async fn generate(&self, route: Option<Uuid>, from: NaiveDate, hari: i32) -> anyhow::Result<u64> {
        let conn = self.pool.get().await?;
        Ok(conn
            .execute(
                "INSERT INTO schedules (armada_id, tanggal, route_id, asal, tujuan, lokasi_jemput, jemput_lat,
                     jemput_lng, jam, jam_tiba, harga, catatan, kapasitas, konfigurasi, dua_dek, kursi_wanita,
                     driver_nama, driver_telp)
                 SELECT r.armada_id, d.tgl, r.id, r.asal, r.tujuan, r.lokasi_jemput, r.jemput_lat, r.jemput_lng,
                        r.jam_berangkat, r.jam_tiba, r.harga, r.catatan, r.kapasitas, r.konfigurasi, r.dua_dek,
                        r.kursi_wanita, r.driver_nama, r.driver_telp
                   FROM routes r
                  CROSS JOIN (SELECT ($1::date + g) AS tgl FROM generate_series(0, $2::int - 1) g) d
                  WHERE r.aktif AND r.armada_id IS NOT NULL
                    AND ($3::uuid IS NULL OR r.id = $3)
                    AND (r.hari_operasi::int & (1 << (EXTRACT(ISODOW FROM d.tgl)::int - 1))) <> 0
                 ON CONFLICT (route_id, tanggal) DO NOTHING",
                &[&from, &hari, &route],
            )
            .await?)
    }

    /// Jadwal satu trayek mulai `from` (termasuk yang dibatalkan) — untuk
    /// panel penugasan bus & driver harian.
    pub async fn schedules_of(&self, id: &str, from: NaiveDate, hari: i32) -> anyhow::Result<Vec<Schedule>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let sql = format!(
            "{SELECT_JOIN_ARMADA} WHERE s.route_id = $1 AND s.tanggal >= $2 AND s.tanggal < $2 + $3::int
              ORDER BY s.tanggal"
        );
        let rows = conn.query(&sql, &[&uuid, &from, &hari]).await?;
        Ok(rows.iter().map(row_to_schedule).collect())
    }

    /// Pemilik jadwal: pemilik trayeknya, atau pemilik armada untuk jadwal
    /// sekali jalan. `None` = milik platform.
    pub async fn schedule_owner(&self, schedule_id: &str) -> anyhow::Result<Option<String>> {
        let uuid = Uuid::parse_str(schedule_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(
                "SELECT CASE WHEN s.route_id IS NOT NULL THEN r.merchant_id ELSE a.merchant_id END AS owner
                   FROM schedules s
                   JOIN armadas a ON a.id = s.armada_id
                   LEFT JOIN routes r ON r.id = s.route_id
                  WHERE s.id = $1",
                &[&uuid],
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("Jadwal tidak ditemukan"))?;
        Ok(row.get::<_, Option<Uuid>>("owner").map(|u| u.to_string()))
    }

    pub async fn assign(&self, schedule_id: &str, armada_id: &str, driver_nama: &str, driver_telp: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(schedule_id)?;
        let armada = Uuid::parse_str(armada_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE schedules SET armada_id = $2, driver_nama = $3, driver_telp = $4 WHERE id = $1",
            &[&uuid, &armada, &driver_nama, &driver_telp],
        )
        .await?;
        Ok(())
    }

    /// Batalkan keberangkatan satu hari — ditolak bila sudah ada order.
    pub async fn set_batal(&self, schedule_id: &str, batal: bool) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(schedule_id)?;
        let conn = self.pool.get().await?;
        if batal {
            let n = conn
                .execute(
                    &format!("UPDATE schedules SET batal = TRUE WHERE id = $1 AND {TANPA_ORDER}"),
                    &[&uuid],
                )
                .await?;
            if n == 0 {
                anyhow::bail!("Keberangkatan ini sudah punya penumpang — tidak bisa dibatalkan dari sini");
            }
        } else {
            conn.execute("UPDATE schedules SET batal = FALSE WHERE id = $1", &[&uuid]).await?;
        }
        Ok(())
    }
}

fn row_to_route(row: &tokio_postgres::Row) -> Route {
    Route {
        id: row.get::<_, Uuid>("id").to_string(),
        merchant_id: row.get::<_, Option<Uuid>>("merchant_id").map(|u| u.to_string()),
        pasangan_id: row.get::<_, Option<Uuid>>("pasangan_id").map(|u| u.to_string()),
        asal: row.get("asal"),
        tujuan: row.get("tujuan"),
        lokasi_jemput: row.get("lokasi_jemput"),
        jemput_lat: row.get("jemput_lat"),
        jemput_lng: row.get("jemput_lng"),
        jam_berangkat: row.get("jam_berangkat"),
        jam_tiba: row.get("jam_tiba"),
        harga: row.get("harga"),
        kapasitas: row.get("kapasitas"),
        konfigurasi: row.get("konfigurasi"),
        dua_dek: row.get("dua_dek"),
        kursi_wanita: row.get("kursi_wanita"),
        catatan: row.get("catatan"),
        armada_id: row.get::<_, Option<Uuid>>("armada_id").map(|u| u.to_string()),
        armada_name: row.get("armada_name"),
        driver_nama: row.get("driver_nama"),
        driver_telp: row.get("driver_telp"),
        hari_operasi: row.get("hari_operasi"),
        aktif: row.get("aktif"),
    }
}
