use chrono::{DateTime, NaiveDate, Utc};
use deadpool_postgres::Pool;
use rand::Rng;
use uuid::Uuid;

use crate::web::models::{NewOrder, OrderDetail, OrderSummary};

#[derive(Clone)]
pub struct OrderRepository {
    pool: Pool,
}

impl OrderRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Checkout dalam SATU transaksi: kunci baris jadwal (`FOR UPDATE`),
    /// pastikan kapasitas masih cukup, baru insert order + tambah
    /// `kursi_terjual`. Mencegah dua pembeli oversell kursi yang sama saat
    /// checkout serentak.
    ///
    /// Kursi bernomor: tiap kode divalidasi terhadap denah jadwal
    /// (`web::seats::layout`) lalu disimpan di `order_seats`, yang PRIMARY
    /// KEY-nya (schedule_id, kode) menolak kursi yang sudah dimiliki order
    /// lain — dua pembeli yang mengklik kursi yang sama bersamaan, hanya satu
    /// yang lolos, dan transaksi yang kalah dibatalkan utuh.
    pub async fn create(&self, buyer_id: &str, input: &NewOrder) -> anyhow::Result<OrderDetail> {
        let kursi = crate::web::seats::parse_kode_list(&input.kursi);
        if kursi.is_empty() {
            anyhow::bail!("Pilih minimal 1 kursi");
        }
        if kursi.len() > 10 {
            anyhow::bail!("Maksimal 10 kursi per pemesanan");
        }
        let jumlah = kursi.len() as i32;
        let buyer_uuid = Uuid::parse_str(buyer_id)?;
        let schedule_uuid = Uuid::parse_str(&input.schedule_id)?;

        let order_id = {
            let mut conn = self.pool.get().await?;
            let tx = conn.transaction().await?;

            let row = tx
                .query_opt(
                    "SELECT harga, kapasitas, kursi_terjual, konfigurasi, dua_dek, tanggal, batal
                       FROM schedules WHERE id = $1 FOR UPDATE",
                    &[&schedule_uuid],
                )
                .await?;
            let Some(row) = row else {
                anyhow::bail!("Jadwal tidak ditemukan");
            };
            let harga: i64 = row.get("harga");
            let kapasitas: i32 = row.get("kapasitas");
            let kursi_terjual: i32 = row.get("kursi_terjual");
            let konfigurasi: String = row.get("konfigurasi");
            let dua_dek: bool = row.get("dua_dek");
            let tanggal: NaiveDate = row.get("tanggal");
            if row.get::<_, bool>("batal") {
                anyhow::bail!("Keberangkatan ini dibatalkan operator");
            }
            if tanggal < (Utc::now() + chrono::Duration::hours(7)).date_naive() {
                anyhow::bail!("Jadwal ini sudah lewat");
            }
            let denah = crate::web::seats::layout(kapasitas, &konfigurasi, dua_dek);
            if let Some(k) = kursi.iter().find(|k| !denah.iter().any(|d| &d.kode == *k)) {
                anyhow::bail!("Kursi {k} tidak ada di denah bus ini");
            }
            let sisa = kapasitas - kursi_terjual;
            if sisa < jumlah {
                anyhow::bail!("Kursi tidak cukup, sisa {sisa}");
            }

            tx.execute(
                "UPDATE schedules SET kursi_terjual = kursi_terjual + $1 WHERE id = $2",
                &[&jumlah, &schedule_uuid],
            )
            .await?;

            let total = harga * jumlah as i64;
            let kode_order = generate_order_code();

            let order_row = tx
                .query_one(
                    "INSERT INTO orders (kode_order, buyer_id, schedule_id, jumlah_tiket, harga_satuan, total_harga, nama_pemesan, telp_pemesan)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                     RETURNING id",
                    &[
                        &kode_order,
                        &buyer_uuid,
                        &schedule_uuid,
                        &jumlah,
                        &harga,
                        &total,
                        &input.nama_pemesan,
                        &input.telp_pemesan,
                    ],
                )
                .await?;

            let order_uuid: Uuid = order_row.get("id");

            for k in &kursi {
                tx.execute(
                    "INSERT INTO order_seats (schedule_id, kode, order_id) VALUES ($1, $2, $3)",
                    &[&schedule_uuid, k, &order_uuid],
                )
                .await
                .map_err(|e| {
                    if e.code() == Some(&tokio_postgres::error::SqlState::UNIQUE_VIOLATION) {
                        anyhow::anyhow!("Kursi {k} baru saja dipesan orang lain — silakan pilih kursi lain")
                    } else {
                        anyhow::Error::from(e)
                    }
                })?;
            }

            tx.commit().await?;
            order_uuid.to_string()
        };

        self.get_detail(&order_id, buyer_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("order hilang setelah dibuat"))
    }

    pub async fn get_detail(&self, order_id: &str, buyer_id: &str) -> anyhow::Result<Option<OrderDetail>> {
        let order_uuid = Uuid::parse_str(order_id)?;
        let buyer_uuid = Uuid::parse_str(buyer_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(
                "SELECT o.id, o.kode_order, o.jumlah_tiket, o.harga_satuan, o.total_harga,
                        o.nama_pemesan, o.telp_pemesan, o.status, o.created_at,
                        s.tanggal, s.tujuan, s.lokasi_jemput, s.jam, s.driver_nama, s.driver_telp,
                        a.id AS armada_id, a.name AS armada_name,
                        EXISTS(SELECT 1 FROM ratings r WHERE r.order_id = o.id) AS sudah_dirating,
                        ARRAY(SELECT kode FROM order_seats os WHERE os.order_id = o.id ORDER BY kode) AS kursi
                   FROM orders o
                   JOIN schedules s ON s.id = o.schedule_id
                   JOIN armadas a ON a.id = s.armada_id
                  WHERE o.id = $1 AND o.buyer_id = $2",
                &[&order_uuid, &buyer_uuid],
            )
            .await?;
        Ok(row.map(|r| row_to_detail(&r)))
    }

    pub async fn list_by_buyer(&self, buyer_id: &str) -> anyhow::Result<Vec<OrderSummary>> {
        let buyer_uuid = Uuid::parse_str(buyer_id)?;
        let conn = self.pool.get().await?;
        let rows = conn
            .query(
                "SELECT o.id, o.kode_order, o.jumlah_tiket, o.total_harga, o.status, o.created_at,
                        s.tanggal, s.tujuan, a.name AS armada_name,
                        EXISTS(SELECT 1 FROM ratings r WHERE r.order_id = o.id) AS sudah_dirating,
                        ARRAY(SELECT kode FROM order_seats os WHERE os.order_id = o.id ORDER BY kode) AS kursi
                   FROM orders o
                   JOIN schedules s ON s.id = o.schedule_id
                   JOIN armadas a ON a.id = s.armada_id
                  WHERE o.buyer_id = $1
                  ORDER BY o.created_at DESC",
                &[&buyer_uuid],
            )
            .await?;
        Ok(rows.iter().map(row_to_summary).collect())
    }
}

fn generate_order_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let today = Utc::now().format("%Y%m%d");
    let mut rng = rand::rng();
    let suffix: String = (0..4).map(|_| CHARS[rng.random_range(0..CHARS.len())] as char).collect();
    format!("LJB-{today}-{suffix}")
}

fn row_to_detail(row: &tokio_postgres::Row) -> OrderDetail {
    let tanggal: NaiveDate = row.get("tanggal");
    let created_at: DateTime<Utc> = row.get("created_at");
    OrderDetail {
        id: row.get::<_, Uuid>("id").to_string(),
        kode_order: row.get("kode_order"),
        armada_id: row.get::<_, Uuid>("armada_id").to_string(),
        armada_name: row.get("armada_name"),
        tanggal: tanggal.format("%Y-%m-%d").to_string(),
        tujuan: row.get("tujuan"),
        lokasi_jemput: row.get("lokasi_jemput"),
        jam: row.get("jam"),
        driver_nama: row.get("driver_nama"),
        driver_telp: row.get("driver_telp"),
        jumlah_tiket: row.get("jumlah_tiket"),
        harga_satuan: row.get("harga_satuan"),
        total_harga: row.get("total_harga"),
        nama_pemesan: row.get("nama_pemesan"),
        telp_pemesan: row.get("telp_pemesan"),
        status: row.get("status"),
        created_at: created_at.to_rfc3339(),
        sudah_dirating: row.get("sudah_dirating"),
        kursi: row.get("kursi"),
    }
}

fn row_to_summary(row: &tokio_postgres::Row) -> OrderSummary {
    let tanggal: NaiveDate = row.get("tanggal");
    let created_at: DateTime<Utc> = row.get("created_at");
    OrderSummary {
        id: row.get::<_, Uuid>("id").to_string(),
        kode_order: row.get("kode_order"),
        tanggal: tanggal.format("%Y-%m-%d").to_string(),
        tujuan: row.get("tujuan"),
        armada_name: row.get("armada_name"),
        jumlah_tiket: row.get("jumlah_tiket"),
        total_harga: row.get("total_harga"),
        status: row.get("status"),
        created_at: created_at.to_rfc3339(),
        sudah_dirating: row.get("sudah_dirating"),
        kursi: row.get("kursi"),
    }
}
