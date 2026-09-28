use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;

pub struct PendingRegistration {
    pub name: String,
    pub role: String,
    pub password_hash: String,
    pub otp_code: String,
    pub attempts: i32,
    pub expires_at: DateTime<Utc>,
    /// Draf profil PO (JSON `NewMerchantProfile`) dari /daftar-mitra.
    pub profil_po: Option<String>,
}

/// Draf pendaftaran sebelum OTP diverifikasi. Satu draf aktif per nomor HP
/// (lihat `migration/002_marketplace.sql`) — TIDAK dipakai untuk rate-limit
/// (baris lama tertimpa tiap request baru, jadi tak ada histori untuk
/// dihitung); rate-limit permintaan OTP ada di `service::rate_limit`
/// (in-memory, lihat plan untuk alasan tak pakai Redis).
#[derive(Clone)]
pub struct OtpRepository {
    pool: Pool,
}

impl OtpRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn upsert(
        &self,
        phone: &str,
        name: &str,
        role: &str,
        password_hash: &str,
        otp_code: &str,
        expires_at: DateTime<Utc>,
        profil_po: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO otp_pending (phone, name, role, password_hash, otp_code, attempts, expires_at, profil_po)
             VALUES ($1, $2, $3, $4, $5, 0, $6, $7)
             ON CONFLICT (phone) DO UPDATE SET
                name = EXCLUDED.name, role = EXCLUDED.role, password_hash = EXCLUDED.password_hash,
                otp_code = EXCLUDED.otp_code, attempts = 0, expires_at = EXCLUDED.expires_at,
                profil_po = EXCLUDED.profil_po, created_at = NOW()",
            &[&phone, &name, &role, &password_hash, &otp_code, &expires_at, &profil_po],
        )
        .await?;
        Ok(())
    }

    pub async fn get(&self, phone: &str) -> anyhow::Result<Option<PendingRegistration>> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(
                "SELECT name, role, password_hash, otp_code, attempts, expires_at, profil_po
                   FROM otp_pending WHERE phone = $1",
                &[&phone],
            )
            .await?;
        Ok(row.map(|r| PendingRegistration {
            name: r.get("name"),
            role: r.get("role"),
            password_hash: r.get("password_hash"),
            otp_code: r.get("otp_code"),
            attempts: r.get("attempts"),
            expires_at: r.get("expires_at"),
            profil_po: r.get("profil_po"),
        }))
    }

    pub async fn increment_attempts(&self, phone: &str) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        conn.execute("UPDATE otp_pending SET attempts = attempts + 1 WHERE phone = $1", &[&phone])
            .await?;
        Ok(())
    }

    pub async fn delete(&self, phone: &str) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM otp_pending WHERE phone = $1", &[&phone]).await?;
        Ok(())
    }
}
