use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

pub struct PendingPhoneChange {
    pub new_phone: String,
    pub otp_code: String,
    pub attempts: i32,
    pub expires_at: DateTime<Utc>,
}

/// Permintaan ganti nomor yang menunggu OTP dari nomor BARU. Satu per user.
#[derive(Clone)]
pub struct PhoneChangeRepository {
    pool: Pool,
}

impl PhoneChangeRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn upsert(
        &self,
        user_id: &str,
        new_phone: &str,
        otp_code: &str,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO phone_change_pending (user_id, new_phone, otp_code, attempts, expires_at)
             VALUES ($1, $2, $3, 0, $4)
             ON CONFLICT (user_id) DO UPDATE SET
                new_phone = EXCLUDED.new_phone, otp_code = EXCLUDED.otp_code,
                attempts = 0, expires_at = EXCLUDED.expires_at, created_at = NOW()",
            &[&uuid, &new_phone, &otp_code, &expires_at],
        )
        .await?;
        Ok(())
    }

    pub async fn get(&self, user_id: &str) -> anyhow::Result<Option<PendingPhoneChange>> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(
                "SELECT new_phone, otp_code, attempts, expires_at FROM phone_change_pending WHERE user_id = $1",
                &[&uuid],
            )
            .await?;
        Ok(row.map(|r| PendingPhoneChange {
            new_phone: r.get("new_phone"),
            otp_code: r.get("otp_code"),
            attempts: r.get("attempts"),
            expires_at: r.get("expires_at"),
        }))
    }

    pub async fn increment_attempts(&self, user_id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE phone_change_pending SET attempts = attempts + 1 WHERE user_id = $1",
            &[&uuid],
        )
        .await?;
        Ok(())
    }

    pub async fn delete(&self, user_id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute("DELETE FROM phone_change_pending WHERE user_id = $1", &[&uuid])
            .await?;
        Ok(())
    }
}
