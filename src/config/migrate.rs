//! config/migrate.rs — penjalan migrasi database.
//!
//! Berkas `migration/*.sql` di-embed ke binari lewat `build.rs`, dikirim UTUH
//! ke Postgres lewat `batch_execute` (Postgres sendiri yang memisah
//! pernyataannya — bukan klien yang memotong tiap titik-koma), dicatat di
//! `schema_migrations`, dan dijaga dari dua instance yang jalan bersamaan
//! lewat `pg_advisory_lock`. Pola disalin dari e-ticketing, disederhanakan:
//! proyek ini fresh, tidak ada histori "sudah dimigrasi tangan" yang perlu
//! garis dasar.

use anyhow::{Context, Result};
use deadpool_postgres::Pool;

include!(concat!(env!("OUT_DIR"), "/migrations.rs"));

const LOCK_KEY: i64 = 0x4249_5330_3030_3031;

fn checksum(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{h:016x}")
}

pub async fn run(pool: &Pool) -> Result<()> {
    let mut conn = pool.get().await.context("migrate: ambil koneksi")?;

    conn.batch_execute("SET statement_timeout = 0")
        .await
        .context("migrate: mematikan statement_timeout")?;

    conn.execute("SELECT pg_advisory_lock($1)", &[&LOCK_KEY])
        .await
        .context("migrate: pg_advisory_lock")?;

    let hasil = jalankan(&mut conn).await;

    if let Err(e) = conn
        .execute("SELECT pg_advisory_unlock($1)", &[&LOCK_KEY])
        .await
    {
        tracing::warn!(error = %e, "migrate: gagal melepas advisory lock");
    }

    if let Err(e) = conn.batch_execute("RESET statement_timeout").await {
        tracing::warn!(error = %e, "migrate: gagal mengembalikan statement_timeout");
    }

    hasil
}

async fn jalankan(conn: &mut deadpool_postgres::Object) -> Result<()> {
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version     TEXT        PRIMARY KEY,
             checksum    TEXT        NOT NULL,
             applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
         )",
    )
    .await
    .context("migrate: buat schema_migrations")?;

    let baris = conn
        .query("SELECT version, checksum FROM schema_migrations", &[])
        .await
        .context("migrate: baca schema_migrations")?;

    let mut sudah: std::collections::HashMap<String, String> =
        std::collections::HashMap::with_capacity(baris.len());
    for r in baris {
        sudah.insert(r.get::<_, String>(0), r.get::<_, String>(1));
    }

    let mut dijalankan = 0_usize;

    for (nama, sql) in MIGRATIONS {
        let cs = checksum(sql);

        if let Some(lama) = sudah.get(*nama) {
            if lama != &cs {
                tracing::warn!(
                    migration = nama,
                    "migrate: berkas BERUBAH setelah dijalankan — perubahan itu tidak ikut masuk"
                );
            }
            continue;
        }

        tracing::info!(migration = nama, "migrate: menjalankan");

        let tx = conn
            .transaction()
            .await
            .with_context(|| format!("migrate: buka transaksi {nama}"))?;

        tx.batch_execute(sql)
            .await
            .with_context(|| format!("migrate: GAGAL di {nama}"))?;

        tx.execute(
            "INSERT INTO schema_migrations (version, checksum) VALUES ($1, $2)",
            &[nama, &cs],
        )
        .await
        .with_context(|| format!("migrate: catat {nama}"))?;

        tx.commit()
            .await
            .with_context(|| format!("migrate: commit {nama}"))?;

        dijalankan += 1;
    }

    if dijalankan == 0 {
        tracing::info!("migrate: skema sudah mutakhir");
    } else {
        tracing::info!(count = dijalankan, "migrate: selesai");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daftar_migrasi_terurut() {
        let nama: Vec<&str> = MIGRATIONS.iter().map(|(n, _)| *n).collect();
        let mut urut = nama.clone();
        urut.sort_unstable();
        assert_eq!(nama, urut, "MIGRATIONS harus urut menurut nama berkas");
        assert!(!MIGRATIONS.is_empty(), "tak ada migrasi yang ter-embed");
    }

    #[test]
    fn checksum_stabil_dan_peka() {
        let a = "CREATE TABLE x (id int);";
        assert_eq!(checksum(a), checksum(a));
        assert_ne!(checksum(a), checksum("CREATE TABLE y (id int);"));
    }
}
