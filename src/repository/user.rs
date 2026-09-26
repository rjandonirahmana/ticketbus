use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::web::models::PublicUser;

pub struct UserWithHash {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub role: String,
    pub password_hash: String,
    /// Password dari "lupa password" yang belum pernah dipakai login.
    pub pending_password_hash: Option<String>,
    pub pending_password_expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct UserRepository {
    pool: Pool,
}

impl UserRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn exists_by_phone(&self, phone: &str) -> anyhow::Result<bool> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_one("SELECT EXISTS(SELECT 1 FROM users WHERE phone = $1) AS ada", &[&phone])
            .await?;
        Ok(row.get("ada"))
    }

    pub async fn create(
        &self,
        phone: &str,
        name: &str,
        role: &str,
        password_hash: &str,
    ) -> anyhow::Result<PublicUser> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO users (phone, name, role, password_hash) VALUES ($1, $2, $3, $4)
                 RETURNING id, name, phone, role",
                &[&phone, &name, &role, &password_hash],
            )
            .await?;
        Ok(row_to_public(&row))
    }

    pub async fn find_by_phone_with_hash(&self, phone: &str) -> anyhow::Result<Option<UserWithHash>> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(
                "SELECT id, name, phone, role, password_hash,
                        pending_password_hash, pending_password_expires_at
                   FROM users WHERE phone = $1",
                &[&phone],
            )
            .await?;
        Ok(row.map(|r| UserWithHash {
            id: r.get::<_, Uuid>("id").to_string(),
            name: r.get("name"),
            phone: r.get("phone"),
            role: r.get("role"),
            password_hash: r.get("password_hash"),
            pending_password_hash: r.get("pending_password_hash"),
            pending_password_expires_at: r.get("pending_password_expires_at"),
        }))
    }

    pub async fn find_by_id(&self, id: &str) -> anyhow::Result<Option<PublicUser>> {
        let uuid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt("SELECT id, name, phone, role FROM users WHERE id = $1", &[&uuid])
            .await?;
        Ok(row.map(|r| row_to_public(&r)))
    }

    pub async fn admin_exists(&self) -> anyhow::Result<bool> {
        let conn = self.pool.get().await?;
        let row = conn
            .query_one("SELECT EXISTS(SELECT 1 FROM users WHERE role = 'admin') AS ada", &[])
            .await?;
        Ok(row.get("ada"))
    }

    /// Idempoten lewat `ON CONFLICT (phone) DO NOTHING` — aman dipanggil tiap
    /// startup tanpa membuat duplikat.
    pub async fn create_admin(&self, phone: &str, name: &str, password_hash: &str) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO users (phone, name, role, password_hash) VALUES ($1, $2, 'admin', $3)
             ON CONFLICT (phone) DO NOTHING",
            &[&phone, &name, &password_hash],
        )
        .await?;
        Ok(())
    }

    pub async fn set_pending_password(
        &self,
        user_id: &str,
        hash: &str,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE users SET pending_password_hash = $2, pending_password_expires_at = $3 WHERE id = $1",
            &[&uuid, &hash, &expires_at],
        )
        .await?;
        Ok(())
    }

    /// Password tertunda terbukti dipakai pemiliknya → jadikan password resmi.
    pub async fn promote_pending_password(&self, user_id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE users SET password_hash = pending_password_hash,
                              pending_password_hash = NULL, pending_password_expires_at = NULL,
                              password_changed_at = NOW()
              WHERE id = $1 AND pending_password_hash IS NOT NULL",
            &[&uuid],
        )
        .await?;
        Ok(())
    }

    /// Hash password + kapan terakhir diubah (fallback: waktu akun dibuat).
    pub async fn password_state(&self, user_id: &str) -> anyhow::Result<Option<(String, DateTime<Utc>)>> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt(
                "SELECT password_hash, COALESCE(password_changed_at, created_at) AS changed
                   FROM users WHERE id = $1",
                &[&uuid],
            )
            .await?;
        Ok(row.map(|r| (r.get("password_hash"), r.get("changed"))))
    }

    /// Ganti password (sekaligus membatalkan "lupa password" yang tertunda).
    /// `revoke_others` → sesi yang terbit sebelum `now` tak lagi berlaku.
    pub async fn update_password(
        &self,
        user_id: &str,
        hash: &str,
        now: DateTime<Utc>,
        revoke_others: bool,
    ) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE users SET password_hash = $2, password_changed_at = $3,
                              pending_password_hash = NULL, pending_password_expires_at = NULL,
                              sessions_valid_after = CASE WHEN $4 THEN $3 ELSE sessions_valid_after END
              WHERE id = $1",
            &[&uuid, &hash, &now, &revoke_others],
        )
        .await?;
        Ok(())
    }

    pub async fn set_sessions_valid_after(&self, user_id: &str, at: DateTime<Utc>) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute("UPDATE users SET sessions_valid_after = $2 WHERE id = $1", &[&uuid, &at])
            .await?;
        Ok(())
    }

    /// Semua pencabutan sesi yang masih relevan — dimuat sekali saat start.
    pub async fn session_revocations(&self) -> anyhow::Result<Vec<(String, i64)>> {
        let conn = self.pool.get().await?;
        let rows = conn
            .query("SELECT id, sessions_valid_after FROM users WHERE sessions_valid_after IS NOT NULL", &[])
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let at: DateTime<Utc> = r.get("sessions_valid_after");
                (r.get::<_, Uuid>("id").to_string(), at.timestamp())
            })
            .collect())
    }

    pub async fn clear_pending_password(&self, user_id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE users SET pending_password_hash = NULL, pending_password_expires_at = NULL
              WHERE id = $1 AND pending_password_hash IS NOT NULL",
            &[&uuid],
        )
        .await?;
        Ok(())
    }

    pub async fn update_phone(&self, user_id: &str, new_phone: &str) -> anyhow::Result<PublicUser> {
        let uuid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "UPDATE users SET phone = $2 WHERE id = $1 RETURNING id, name, phone, role",
                &[&uuid, &new_phone],
            )
            .await
            .map_err(|e| {
                // Nomor bisa diklaim akun lain di antara request & verifikasi.
                if e.code() == Some(&tokio_postgres::error::SqlState::UNIQUE_VIOLATION) {
                    anyhow::anyhow!("Nomor HP sudah dipakai akun lain")
                } else {
                    e.into()
                }
            })?;
        Ok(row_to_public(&row))
    }
}

fn row_to_public(row: &tokio_postgres::Row) -> PublicUser {
    PublicUser {
        id: row.get::<_, Uuid>("id").to_string(),
        name: row.get("name"),
        phone: row.get("phone"),
        role: row.get("role"),
    }
}
